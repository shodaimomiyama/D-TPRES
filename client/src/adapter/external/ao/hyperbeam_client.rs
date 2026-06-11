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
    rsa_key: rsa::RsaPrivateKey,
    sig_name: String,
}

impl HyperBEAMClient {
    pub fn new(config: AOConfig, wallet: &ArweaveJWK) -> Result<Self, AOCommunicationError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms()))
            .build()
            .map_err(|e| AOCommunicationError::connection_error(format!("HTTP client: {e}")))?;

        let rsa_key = wallet.to_rsa_private_key()?;
        let sig_name = wallet.sig_name()?;

        Ok(Self {
            http,
            config,
            rsa_key,
            sig_name,
        })
    }

    fn base_url(&self) -> &str {
        self.config.mu_url()
    }

    async fn schedule(
        &self,
        process_id: &str,
        action: &str,
        payload_data: &serde_json::Value,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = format!("{}/{}/schedule", self.base_url(), process_id);

        let payload = serde_json::to_string(payload_data).map_err(|e| {
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

        let signed = signer::sign_message(&self.rsa_key, &header_fields, body, &self.sig_name)
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
        let body_text = Box::pin(resp.text()).await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read schedule body: {e}"))
        })?;

        Ok((status, body_text, headers))
    }

    // Drills into a sub-field of the computed slot (e.g. "results/data") and
    // asks for the JSON codec so the response is plain JSON instead of TABM.
    async fn compute_subpath(
        &self,
        process_id: &str,
        slot: u64,
        subpath: &str,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let url = if subpath.is_empty() {
            format!("{}/{}/compute", self.base_url(), process_id)
        } else {
            format!("{}/{}/compute/{}", self.base_url(), process_id, subpath)
        };
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
        let body = Box::pin(resp.text()).await.map_err(|e| {
            AOCommunicationError::connection_error(format!("read compute body: {e}"))
        })?;

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
}

impl HyperBEAMClient {
    async fn schedule_and_compute(
        &self,
        process_id: &str,
        action: &str,
        payload_data: &serde_json::Value,
        subpath: &str,
    ) -> Result<(u16, String, Vec<(String, String)>), AOCommunicationError> {
        let (sched_status, sched_body, sched_headers) =
            Box::pin(self.schedule(process_id, action, payload_data)).await?;

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

        Box::pin(self.compute_subpath(process_id, slot, subpath)).await
    }
}

#[async_trait]
impl AOClient for HyperBEAMClient {
    // Schedule-only: computing intermediate slots leaves broken wasm-64
    // snapshots on the node (cross-request restore loses all state), so the
    // process is computed exactly once — when query() reads the results.
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        let (status, body, _headers) =
            Box::pin(self.schedule(process_id, msg.action(), msg.data())).await?;

        if status >= 400 {
            return Err(AOCommunicationError::execution_error(
                process_id,
                format!(
                    "schedule failed (HTTP {}): {}",
                    status,
                    &body[..body.len().min(200)]
                ),
            ));
        }
        Ok(AONativeResponse::success(serde_json::Value::Null))
    }

    // Read-only queries are still AO messages on HyperBEAM: schedule the
    // query action and read the contract's JSON response from compute.
    async fn query(
        &self,
        process_id: &str,
        msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError> {
        let (status, body, _headers) = Box::pin(self.schedule_and_compute(
            process_id,
            msg.action(),
            msg.data(),
            // JSON-Iface stores the contract's response payload at results/data;
            // the trailing device call renders it as a plain JSON body.
            "results/data/serialize~json@1.0",
        ))
        .await?;
        if status >= 400 {
            return Err(AOCommunicationError::execution_error(
                process_id,
                format!(
                    "query compute failed (HTTP {}): {}",
                    status,
                    &body[..body.len().min(200)]
                ),
            ));
        }
        Ok(Binary(body.into_bytes()))
    }

    // HyperBEAM does not expose a separate dry-run endpoint.
    // This delegates to execute(), which performs a real schedule+compute.
    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        self.execute(process_id, msg).await
    }
}
