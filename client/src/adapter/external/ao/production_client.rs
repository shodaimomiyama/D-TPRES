//! ProductionAOClient for HyperBEAM AO Network.
//!
//! Port of `ao_cwao/production_client.rs`.
//! HTTP MU/CU communication is unchanged; message types are updated
//! to `AOExecuteMsg` / `AOQueryMsg` / `AONativeResponse`.

#![allow(clippy::large_futures)]

use async_trait::async_trait;
use reqwest::Client;

use super::client::AOClient;
use super::config::AOConfig;
use super::data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
use super::message::{AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary};
use crate::adapter::errors::AOCommunicationError;

pub struct ProductionAOClient {
    config: AOConfig,
    http_client: Client,
    signer: DataItemSigner,
}

impl ProductionAOClient {
    pub fn new(config: AOConfig, jwk: &ArweaveJWK) -> Result<Self, AOCommunicationError> {
        let signer = DataItemSigner::new(jwk)?;
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms()))
            .build()
            .map_err(|e| AOCommunicationError::ConnectionError {
                details: format!("Failed to create HTTP client: {e}"),
            })?;
        Ok(Self { config, http_client, signer })
    }

    async fn post_to_mu(&self, item_bytes: &[u8]) -> Result<String, AOCommunicationError> {
        let response = self.http_client
            .post(self.config.mu_url())
            .header("Content-Type", "application/octet-stream")
            .body(item_bytes.to_vec())
            .send()
            .await
            .map_err(|e| Self::map_reqwest_error(&e, "MU POST", self.config.timeout_ms()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(AOCommunicationError::ExecutionError {
                process_id: String::new(),
                details: format!("MU returned {status}: {body}"),
            });
        }
        let body: serde_json::Value = response.json().await
            .map_err(|e| AOCommunicationError::DeserializationError {
                details: format!("MU response parse: {e}"),
            })?;
        body["id"].as_str().map(str::to_string).ok_or_else(|| {
            AOCommunicationError::DeserializationError {
                details: "MU response missing 'id'".to_string(),
            }
        })
    }

    async fn fetch_cu_result(
        &self,
        process_id: &str,
        message_id: &str,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let url = format!(
            "{}/result/{}?process-id={}&no-busy",
            self.config.cu_url(), message_id, process_id
        );
        let response = self.http_client.get(&url).send().await
            .map_err(|e| Self::map_reqwest_error(&e, "CU result", self.config.timeout_ms()))?;

        let status = response.status();
        if status.as_u16() == 404 {
            return Err(AOCommunicationError::ProcessNotFound {
                process_id: process_id.to_string(),
            });
        }
        if !status.is_success() {
            return Err(AOCommunicationError::ExecutionError {
                process_id: process_id.to_string(),
                details: format!("CU returned {status}"),
            });
        }
        response.json().await.map_err(|e| AOCommunicationError::DeserializationError {
            details: format!("CU result parse: {e}"),
        })
    }

    async fn post_cu_dry_run(
        &self,
        process_id: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let url = format!("{}/dry-run?process-id={}", self.config.cu_url(), process_id);
        let response = self.http_client.post(&url).json(&body).send().await
            .map_err(|e| Self::map_reqwest_error(&e, "CU dry-run", self.config.timeout_ms()))?;

        let status = response.status();
        if status.as_u16() == 404 {
            return Err(AOCommunicationError::ProcessNotFound {
                process_id: process_id.to_string(),
            });
        }
        if !status.is_success() {
            return Err(AOCommunicationError::ExecutionError {
                process_id: process_id.to_string(),
                details: format!("CU dry-run returned {status}"),
            });
        }
        response.json().await.map_err(|e| AOCommunicationError::DeserializationError {
            details: format!("CU dry-run parse: {e}"),
        })
    }

    /// Parse AO-native response from CU result.
    ///
    /// The new contract returns `{ "ok": bool, "data": {...}, "error": "..." }`.
    /// `ok: false` or any unparseable Output.data is treated as an error —
    /// never silently promoted to success.
    fn parse_cu_result(
        process_id: &str,
        json: &serde_json::Value,
        message_id: Option<String>,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        // AO-level error (process not found, scheduler rejection, etc.)
        if let Some(error) = json.get("Error") {
            if !error.is_null() {
                return Err(AOCommunicationError::ExecutionError {
                    process_id: process_id.to_string(),
                    details: format!("Process error: {error}"),
                });
            }
        }

        // Extract Output.data (contract response JSON string)
        let output = json.get("Output");
        if let Some(obj) = output {
            if let Some(data_str) = obj.get("data").and_then(|d| d.as_str()) {
                // Must parse as AONativeResponse; if the contract returned well-formed JSON,
                // this always succeeds. Non-JSON output is a contract bug → surface as error.
                let mut native_resp =
                    serde_json::from_str::<AONativeResponse>(data_str).map_err(|e| {
                        AOCommunicationError::DeserializationError {
                            details: format!(
                                "Contract Output.data is not a valid AONativeResponse: {e}; \
                                 raw output: {data_str}"
                            ),
                        }
                    })?;

                // ok: false means the contract reported a logical error
                if !native_resp.ok {
                    let reason = native_resp
                        .error
                        .unwrap_or_else(|| "contract returned ok:false".to_string());
                    return Err(AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: reason,
                    });
                }

                native_resp.message_id = message_id;
                return Ok(native_resp);
            }

            // Output is a JSON object/array (not a string) — wrap it
            if obj.is_object() || obj.is_array() {
                return Ok(AONativeResponse {
                    ok: true,
                    data: Some(obj.clone()),
                    error: None,
                    message_id,
                });
            }
        }

        // No Output block — treat as empty success (e.g. MU acknowledgment only)
        Ok(AONativeResponse { ok: true, data: None, error: None, message_id })
    }

    fn parse_cu_dryrun_data(
        process_id: &str,
        json: &serde_json::Value,
    ) -> Result<Binary, AOCommunicationError> {
        if let Some(error) = json.get("Error") {
            if !error.is_null() {
                return Err(AOCommunicationError::ExecutionError {
                    process_id: process_id.to_string(),
                    details: format!("Process error: {error}"),
                });
            }
        }

        if let Some(obj) = json.get("Output") {
            if let Some(data_str) = obj.get("data").and_then(|d| d.as_str()) {
                // Try base64 decode, fallback to raw bytes
                use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine};
                return match STANDARD.decode(data_str)
                    .or_else(|_| URL_SAFE_NO_PAD.decode(data_str))
                {
                    Ok(decoded) => Ok(Binary::from(decoded)),
                    Err(_) => Ok(Binary::from(data_str.as_bytes().to_vec())),
                };
            }
            if obj.is_object() || obj.is_array() {
                let serialized = serde_json::to_vec(obj).map_err(|e| {
                    AOCommunicationError::DeserializationError {
                        details: format!("Output serialize: {e}"),
                    }
                })?;
                return Ok(Binary::from(serialized));
            }
        }

        Ok(Binary::from(Vec::new()))
    }

    fn map_reqwest_error(err: &reqwest::Error, operation: &str, timeout_ms: u64) -> AOCommunicationError {
        if err.is_timeout() {
            AOCommunicationError::Timeout { operation: operation.to_string(), timeout_ms }
        } else if err.is_connect() {
            AOCommunicationError::ConnectionError { details: format!("{operation} connect: {err}") }
        } else {
            AOCommunicationError::ExecutionError {
                process_id: String::new(),
                details: format!("{operation}: {err}"),
            }
        }
    }
}

#[async_trait]
impl AOClient for ProductionAOClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        let item = DataItemBuilder::build_execute(process_id, &msg)?;
        let signed_bytes = self.signer.sign(&item)?;
        let message_id = self.post_to_mu(&signed_bytes).await?;
        let result_json = self.fetch_cu_result(process_id, &message_id).await?;
        Self::parse_cu_result(process_id, &result_json, Some(message_id))
    }

    async fn query(
        &self,
        process_id: &str,
        msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError> {
        let body = DataItemBuilder::build_query_body(process_id, &msg)?;
        let result_json = self.post_cu_dry_run(process_id, body).await?;
        Self::parse_cu_dryrun_data(process_id, &result_json)
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        let body = DataItemBuilder::build_dry_run_body(process_id, &msg)?;
        let result_json = self.post_cu_dry_run(process_id, body).await?;
        Self::parse_cu_result(process_id, &result_json, None)
    }
}
