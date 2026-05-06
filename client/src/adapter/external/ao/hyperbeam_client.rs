use async_trait::async_trait;
use std::collections::BTreeMap;

use super::client::AOClient;
use super::config::AOConfig;
use super::message::{AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary};
use super::signer;
use super::wallet::ArweaveJWK;
use crate::adapter::errors::AOCommunicationError;

pub struct HyperBEAMClient {
    http: reqwest::Client,
    config: AOConfig,
    wallet: ArweaveJWK,
}

impl HyperBEAMClient {
    pub fn new(config: AOConfig, wallet: ArweaveJWK) -> Result<Self, AOCommunicationError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms()))
            .build()
            .map_err(|e| AOCommunicationError::connection_error(format!("HTTP client: {e}")))?;

        Ok(Self {
            http,
            config,
            wallet,
        })
    }

    fn base_url(&self) -> &str {
        self.config.mu_url()
    }

    fn rsa_key(&self) -> Result<rsa::RsaPrivateKey, AOCommunicationError> {
        self.wallet.to_rsa_private_key()
    }

    async fn schedule(
        &self,
        process_id: &str,
        msg: &AOExecuteMsg,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/schedule", self.base_url(), process_id);
        let rsa_key = self.rsa_key()?;
        let sig_name = self.wallet.sig_name();

        let action = msg.action();
        let payload = serde_json::to_string(msg.data()).map_err(|e| {
            AOCommunicationError::serialization_error(format!("serialize msg data: {e}"))
        })?;
        let body = payload.as_bytes();

        let header_fields: BTreeMap<String, String> = BTreeMap::from([
            ("action".to_string(), action.to_string()),
            ("data-protocol".to_string(), "ao".to_string()),
            ("inline-body-key".to_string(), "data".to_string()),
            ("target".to_string(), process_id.to_string()),
            ("type".to_string(), "Message".to_string()),
            ("variant".to_string(), "ao.N.1".to_string()),
        ]);

        let signed = signer::sign_message(&rsa_key, &header_fields, body, &sig_name)
            .map_err(|e| AOCommunicationError::signing_error(format!("{e}")))?;

        let mut req = self.http.post(&url);
        for (name, value) in &header_fields {
            req = req.header(name.as_str(), value.as_str());
        }
        if !signed.content_digest_header.is_empty() {
            req = req.header("content-digest", &signed.content_digest_header);
        }
        let resp = req
            .header("signature", &signed.signature_header)
            .header("signature-input", &signed.signature_input_header)
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| AOCommunicationError::connection_error(format!("schedule POST: {e}")))?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body_text = resp.text().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read schedule body: {e}"))
        })?;

        Ok((status, body_text, headers))
    }

    async fn compute(
        &self,
        process_id: &str,
        slot: u64,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/compute", self.base_url(), process_id);
        let resp = self
            .http
            .get(&url)
            .header("slot", slot.to_string())
            .send()
            .await
            .map_err(|e| AOCommunicationError::connection_error(format!("compute GET: {e}")))?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp.text().await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read compute body: {e}"))
        })?;

        Ok((status, body, headers))
    }

    async fn now(
        &self,
        process_id: &str,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/now", self.base_url(), process_id);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| AOCommunicationError::connection_error(format!("now GET: {e}")))?;

        let status = resp.status().as_u16();
        let headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = resp
            .text()
            .await
            .map_err(|e| AOCommunicationError::connection_error(format!("read now body: {e}")))?;

        Ok((status, body, headers))
    }

    fn extract_slot(headers: &[(String, String)], body: &str) -> Option<u64> {
        for (key, val) in headers {
            if key.to_lowercase() == "slot" {
                return val.trim().parse().ok();
            }
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            if let Some(s) = v.get("slot").and_then(|s| s.as_u64()) {
                return Some(s);
            }
        }
        None
    }

    fn parse_response(status: u16, body: &str, _headers: &[(String, String)]) -> AONativeResponse {
        if status >= 400 {
            return AONativeResponse::error_response(format!(
                "HTTP {status}: {}",
                &body[..body.len().min(200)]
            ));
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(body) {
            let ok = v.get("ok").and_then(|o| o.as_bool()).unwrap_or(true);
            let error = v
                .get("error")
                .and_then(|e| e.as_str())
                .map(|s| s.to_string());
            if let Some(err) = error {
                return AONativeResponse::error_response(err);
            }
            if !ok {
                return AONativeResponse::error_response("Process returned ok=false");
            }
            AONativeResponse::success(v)
        } else {
            AONativeResponse::success(serde_json::Value::String(body.to_string()))
        }
    }
}

#[async_trait]
impl AOClient for HyperBEAMClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        let (sched_status, sched_body, sched_headers) = self.schedule(process_id, &msg).await?;

        if sched_status >= 400 {
            return Err(AOCommunicationError::execution_error(
                process_id,
                format!(
                    "schedule failed (HTTP {}): {}",
                    sched_status,
                    &sched_body[..sched_body.len().min(200)]
                ),
            ));
        }

        let slot = Self::extract_slot(&sched_headers, &sched_body).unwrap_or(1);

        let (comp_status, comp_body, comp_headers) = self.compute(process_id, slot).await?;

        Ok(Self::parse_response(comp_status, &comp_body, &comp_headers))
    }

    async fn query(
        &self,
        process_id: &str,
        _msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError> {
        let (_status, body, _headers) = self.now(process_id).await?;
        Ok(Binary(body.into_bytes()))
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        self.execute(process_id, msg).await
    }
}
