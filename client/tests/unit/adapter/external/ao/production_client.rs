use formix::adapter::errors::AOCommunicationError;
use formix::adapter::external::ao::{
    AOClient, AOConfig, ArweaveJWK, Binary, ExecuteMsg, ProductionAOClient, QueryMsg,
};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[allow(clippy::many_single_char_names)]
fn test_jwk() -> ArweaveJWK {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rand::rngs::OsRng;
    use rsa::RsaPrivateKey;
    use rsa::traits::{PrivateKeyParts, PublicKeyParts};

    let private_key = RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
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

const TEST_PROCESS_ID: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

fn delegate_kfrag_msg() -> ExecuteMsg {
    ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf-1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    }
}

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
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"Output": {"data": "ok"}})),
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"status": "ok"})))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})))
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
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "msg-1"})))
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
