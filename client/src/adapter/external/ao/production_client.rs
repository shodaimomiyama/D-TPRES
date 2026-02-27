#![allow(clippy::disallowed_names, clippy::large_futures)]

use async_trait::async_trait;
use reqwest::Client;

use super::client::AOClient;
use super::config::AOConfig;
use super::data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};
use super::message::{AOResponse, Binary, ExecuteMsg, QueryMsg};
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

        Ok(Self {
            config,
            http_client,
            signer,
        })
    }

    async fn post_to_mu(
        &self,
        process_id: &str,
        item_bytes: &[u8],
    ) -> Result<String, AOCommunicationError> {
        let url = self.config.mu_url();
        let response = self
            .http_client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .body(item_bytes.to_vec())
            .send()
            .await
            .map_err(|e| Self::map_reqwest_error(&e, "MU POST", self.config.timeout_ms()))?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(AOCommunicationError::ExecutionError {
                process_id: process_id.to_string(),
                details: format!("MU returned status {status}: {body_text}"),
            });
        }

        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| AOCommunicationError::DeserializationError {
                    details: format!("Failed to parse MU response: {e}"),
                })?;

        body["id"].as_str().map(|s| s.to_string()).ok_or_else(|| {
            AOCommunicationError::DeserializationError {
                details: "MU response missing 'id' field".to_string(),
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
            self.config.cu_url(),
            message_id,
            process_id
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
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
                details: format!("CU returned status {status}"),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AOCommunicationError::DeserializationError {
                details: format!("Failed to parse CU result: {e}"),
            })
    }

    async fn post_cu_dry_run(
        &self,
        process_id: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let url = format!("{}/dry-run?process-id={}", self.config.cu_url(), process_id);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
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
                details: format!("CU dry-run returned status {status}"),
            });
        }

        response
            .json()
            .await
            .map_err(|e| AOCommunicationError::DeserializationError {
                details: format!("Failed to parse CU dry-run response: {e}"),
            })
    }

    fn parse_cu_result(
        process_id: &str,
        json: &serde_json::Value,
        message_id: Option<String>,
    ) -> Result<AOResponse, AOCommunicationError> {
        if let Some(error) = json.get("Error") {
            if !error.is_null() {
                return Err(AOCommunicationError::ExecutionError {
                    process_id: process_id.to_string(),
                    details: format!("Process error: {error}"),
                });
            }
        }

        let output = json.get("Output");
        let data = if let Some(obj) = output {
            if let Some(data_str) = obj
                .get("data")
                .and_then(|d| d.as_str())
                .filter(|s| !s.is_empty())
            {
                use base64::{
                    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
                    Engine,
                };
                Some(
                    match STANDARD
                        .decode(data_str)
                        .or_else(|_| URL_SAFE_NO_PAD.decode(data_str))
                    {
                        Ok(decoded) => Binary::from(decoded),
                        Err(_) => Binary::from(data_str.as_bytes().to_vec()),
                    },
                )
            } else if obj.is_object() || obj.is_array() {
                let serialized = serde_json::to_vec(obj).map_err(|e| {
                    AOCommunicationError::DeserializationError {
                        details: format!("Failed to serialize Output: {e}"),
                    }
                })?;
                Some(Binary::from(serialized))
            } else {
                None
            }
        } else {
            None
        };

        Ok(AOResponse {
            success: true,
            data,
            events: Vec::new(),
            message_id,
        })
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

        let output = json.get("Output");

        if let Some(obj) = output {
            // Standard AO CU: Output.data is a base64-encoded string
            if let Some(data_str) = obj.get("data").and_then(|d| d.as_str()) {
                use base64::{
                    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
                    Engine,
                };
                return match STANDARD
                    .decode(data_str)
                    .or_else(|_| URL_SAFE_NO_PAD.decode(data_str))
                {
                    Ok(decoded) => Ok(Binary::from(decoded)),
                    Err(_) => Ok(Binary::from(data_str.as_bytes().to_vec())),
                };
            }
            // CWAO CU: Output is a direct JSON object/array without data field
            if obj.is_object() || obj.is_array() {
                let serialized = serde_json::to_vec(obj).map_err(|e| {
                    AOCommunicationError::DeserializationError {
                        details: format!("Failed to serialize Output: {e}"),
                    }
                })?;
                return Ok(Binary::from(serialized));
            }
        }

        Ok(Binary::from(Vec::new()))
    }

    fn map_reqwest_error(
        err: &reqwest::Error,
        operation: &str,
        timeout_ms: u64,
    ) -> AOCommunicationError {
        if err.is_timeout() {
            AOCommunicationError::Timeout {
                operation: operation.to_string(),
                timeout_ms,
            }
        } else if err.is_connect() {
            AOCommunicationError::ConnectionError {
                details: format!("{operation} connection failed: {err}"),
            }
        } else {
            AOCommunicationError::ConnectionError {
                details: format!("{operation} failed: {err}"),
            }
        }
    }
}

#[async_trait]
impl AOClient for ProductionAOClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError> {
        let item = DataItemBuilder::build_execute(process_id, &msg)?;
        let signed_bytes = self.signer.sign(&item)?;
        let message_id = self.post_to_mu(process_id, &signed_bytes).await?;
        let result_json = self.fetch_cu_result(process_id, &message_id).await?;
        Self::parse_cu_result(process_id, &result_json, Some(message_id))
    }

    async fn query(&self, process_id: &str, msg: QueryMsg) -> Result<Binary, AOCommunicationError> {
        let body = DataItemBuilder::build_query_body(process_id, &msg)?;
        let result_json = self.post_cu_dry_run(process_id, body).await?;
        Self::parse_cu_dryrun_data(process_id, &result_json)
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError> {
        let body = DataItemBuilder::build_dry_run_body(process_id, &msg)?;
        let result_json = self.post_cu_dry_run(process_id, body).await?;
        Self::parse_cu_result(process_id, &result_json, None)
    }
}
