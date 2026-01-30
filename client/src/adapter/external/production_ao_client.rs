use async_trait::async_trait;
use reqwest::Client;

use crate::adapter::errors::AOCommunicationError;
use crate::adapter::external::ao_client::AOClient;
use crate::adapter::external::ao_config::AOConfig;
use crate::adapter::external::ao_message::{AOResponse, Binary, ExecuteMsg, QueryMsg};
use crate::adapter::external::data_item::{ArweaveJWK, DataItemBuilder, DataItemSigner};

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

    async fn post_to_mu(&self, item_bytes: &[u8]) -> Result<String, AOCommunicationError> {
        let url = self.config.mu_url();
        let response = self
            .http_client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .body(item_bytes.to_vec())
            .send()
            .await
            .map_err(|e| Self::map_reqwest_error(&e, "MU POST"))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AOCommunicationError::ExecutionError {
                process_id: String::new(),
                details: format!("MU returned status {status}"),
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
            "{}/result/{}?process-id={}",
            self.config.cu_url(),
            message_id,
            process_id
        );

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| Self::map_reqwest_error(&e, "CU result"))?;

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
            .map_err(|e| Self::map_reqwest_error(&e, "CU dry-run"))?;

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

        let data = json
            .get("Output")
            .and_then(|o| o.get("data"))
            .and_then(|d| d.as_str())
            .map(|s| Binary::from(s.as_bytes().to_vec()));

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

        let data_str = json
            .get("Output")
            .and_then(|o| o.get("data"))
            .and_then(|d| d.as_str())
            .unwrap_or("");

        Ok(Binary::from(data_str.as_bytes().to_vec()))
    }

    fn map_reqwest_error(err: &reqwest::Error, operation: &str) -> AOCommunicationError {
        if err.is_timeout() {
            AOCommunicationError::Timeout {
                operation: operation.to_string(),
                timeout_ms: 0,
            }
        } else if err.is_connect() {
            AOCommunicationError::ConnectionError {
                details: format!("{operation} connection failed: {err}"),
            }
        } else {
            AOCommunicationError::ExecutionError {
                process_id: String::new(),
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
        let message_id = self.post_to_mu(&signed_bytes).await?;
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use crate::adapter::external::data_item::ArweaveJWK;

    fn test_jwk() -> ArweaveJWK {
        use base64::Engine;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use rand::rngs::OsRng;
        use rsa::RsaPrivateKey;
        use rsa::traits::{PrivateKeyParts, PublicKeyParts};

        let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
        let public_key = private_key.to_public_key();
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
        let d = URL_SAFE_NO_PAD.encode(private_key.d().to_bytes_be());
        let primes = private_key.primes();
        let p = URL_SAFE_NO_PAD.encode(primes[0].to_bytes_be());
        let q = URL_SAFE_NO_PAD.encode(primes[1].to_bytes_be());

        ArweaveJWK {
            kty: "RSA".to_string(),
            n,
            e,
            d,
            p,
            q,
            dp: String::new(),
            dq: String::new(),
            qi: String::new(),
        }
    }

    fn setup_client(mu_url: &str, cu_url: &str) -> ProductionAOClient {
        let jwk = test_jwk();
        let config = AOConfig::new(mu_url, cu_url, "https://arweave.net", 5000).unwrap();
        ProductionAOClient::new(config, &jwk).unwrap()
    }

    // Valid base64url-encoded 32-byte target for tests
    const TEST_PROCESS_ID: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn delegate_kfrag_msg() -> ExecuteMsg {
        ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf-1".to_string(),
            kfrag: Binary::new(vec![1, 2, 3]),
        }
    }

    // --- Pure unit tests (no HTTP) ---

    #[test]
    fn test_parse_cu_result_success() {
        let json = serde_json::json!({
            "Output": { "data": "hello" }
        });
        let result =
            ProductionAOClient::parse_cu_result("proc-1", &json, Some("msg-1".to_string()));
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.success);
        assert_eq!(resp.message_id, Some("msg-1".to_string()));
        assert!(resp.data.is_some());
    }

    #[test]
    fn test_parse_cu_result_error() {
        let json = serde_json::json!({ "Error": "something went wrong" });
        let result = ProductionAOClient::parse_cu_result("proc-1", &json, None);
        assert!(matches!(
            result,
            Err(AOCommunicationError::ExecutionError { .. })
        ));
    }

    #[test]
    fn test_parse_cu_result_null_error_field() {
        let json = serde_json::json!({
            "Error": null,
            "Output": { "data": "ok" }
        });
        let result = ProductionAOClient::parse_cu_result("proc-1", &json, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_cu_dryrun_data_success() {
        let json = serde_json::json!({
            "Output": { "data": "{\"cfrag\":\"abc\"}" }
        });
        let result = ProductionAOClient::parse_cu_dryrun_data("proc-1", &json);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_parse_cu_dryrun_data_error() {
        let json = serde_json::json!({ "Error": "process not found" });
        let result = ProductionAOClient::parse_cu_dryrun_data("proc-1", &json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_cu_result_no_output() {
        let json = serde_json::json!({});
        let result = ProductionAOClient::parse_cu_result("proc-1", &json, None);
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.data.is_none());
        assert_eq!(resp.message_id, None);
    }

    // --- wiremock integration tests ---

    #[tokio::test]
    async fn test_execute_success() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-abc-123"})),
            )
            .mount(&mu)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/result/.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"Output": {"data": "success"}})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.success);
    }

    #[tokio::test]
    async fn test_execute_message_id_captured() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-xyz-789"})),
            )
            .mount(&mu)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/result/.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"Output": {"data": "ok"}})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let resp = client
            .execute(TEST_PROCESS_ID, delegate_kfrag_msg())
            .await
            .unwrap();
        assert_eq!(resp.message_id, Some("msg-xyz-789".to_string()));
    }

    #[tokio::test]
    async fn test_execute_mu_returns_500() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ExecutionError { .. })
        ));
    }

    #[tokio::test]
    async fn test_execute_mu_missing_id_field() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})),
            )
            .mount(&mu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::DeserializationError { .. })
        ));
    }

    #[tokio::test]
    async fn test_execute_cu_returns_404() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})),
            )
            .mount(&mu)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/result/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ProcessNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_execute_cu_error_field() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})),
            )
            .mount(&mu)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/result/.*"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"Error": "bad input"})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ExecutionError { .. })
        ));
    }

    #[tokio::test]
    async fn test_execute_cu_returns_500() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})),
            )
            .mount(&mu)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/result/.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ExecutionError { .. })
        ));
    }

    #[tokio::test]
    async fn test_execute_connection_refused() {
        // Point to a port with no server
        let client = setup_client("http://127.0.0.1:19999", "http://127.0.0.1:19998");
        let result = client.execute(TEST_PROCESS_ID, delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ConnectionError { .. })
        ));
    }

    #[tokio::test]
    async fn test_query_success() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/dry-run.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"Output": {"data": "{\"cfrag\":\"abc\"}"}})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let msg = QueryMsg::GetCFrag {
            kfrag_id: "kf-1".to_string(),
            capsule_id: "cap-1".to_string(),
        };
        let result = client.query("proc-1", msg).await;
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_query_cu_404() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/dry-run.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let msg = QueryMsg::GetCFrag {
            kfrag_id: "kf-1".to_string(),
            capsule_id: "cap-1".to_string(),
        };
        let result = client.query("proc-1", msg).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ProcessNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_query_cu_error_field() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/dry-run.*"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"Error": "not found"})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let msg = QueryMsg::GetCFrag {
            kfrag_id: "kf-1".to_string(),
            capsule_id: "cap-1".to_string(),
        };
        let result = client.query("proc-1", msg).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ExecutionError { .. })
        ));
    }

    #[tokio::test]
    async fn test_dry_run_success() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/dry-run.*"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"Output": {"data": "simulated"}})),
            )
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let resp = client
            .dry_run("proc-1", delegate_kfrag_msg())
            .await
            .unwrap();
        assert!(resp.success);
        assert_eq!(resp.message_id, None);
    }

    #[tokio::test]
    async fn test_dry_run_cu_404() {
        let mu = MockServer::start().await;
        let cu = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"/dry-run.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&cu)
            .await;

        let client = setup_client(&mu.uri(), &cu.uri());
        let result = client.dry_run("proc-1", delegate_kfrag_msg()).await;
        assert!(matches!(
            result,
            Err(AOCommunicationError::ProcessNotFound { .. })
        ));
    }

    #[tokio::test]
    async fn test_new_client_valid() {
        let jwk = test_jwk();
        let config = AOConfig::default();
        let result = ProductionAOClient::new(config, &jwk);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_implements_ao_client_trait() {
        fn assert_ao_client<T: AOClient>() {}
        assert_ao_client::<ProductionAOClient>();
    }
}
