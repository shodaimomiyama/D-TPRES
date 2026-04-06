#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args
)]

use formix::adapter::errors::AOCommunicationError;
use formix::adapter::external::ao::{AOClient, AOExecuteMsg, AOQueryMsg, Binary, GetCFragResponse};
use formix::adapter::external::mock_ao::MockAOClient;

/// Test data bundle containing valid Umbral crypto material for re-encryption tests.
struct TestCryptoData {
    /// bincode-serialized StoredKeyFragPayload (what DelegateKFrag receives)
    kfrag_bytes: Vec<u8>,
    /// kFrag identifier
    kfrag_id: String,
    /// Raw Capsule bytes (what DelegateCapsule receives)
    capsule_bytes: Vec<u8>,
}

/// Generate valid Umbral kFrag + Capsule test data for re-encryption.
fn create_test_kfrag_and_capsule() -> TestCryptoData {
    use serde::Serialize;
    use umbral_pre::{DefaultSerialize, SecretKey};

    #[derive(Serialize)]
    struct StoredKeyFragPayload {
        id: String,
        key_data: Vec<u8>,
        verification_data: Vec<u8>,
        precursor: Vec<u8>,
    }
    #[derive(Serialize)]
    struct VerificationData {
        verifying_pk: Vec<u8>,
        delegating_pk: Vec<u8>,
        receiving_pk: Vec<u8>,
    }

    let delegating_sk = SecretKey::random();
    let delegating_pk = delegating_sk.public_key();
    let receiving_sk = SecretKey::random();
    let receiving_pk = receiving_sk.public_key();
    let signing_sk = SecretKey::random();
    let verifying_pk = signing_sk.public_key();
    let signer = umbral_pre::Signer::new(signing_sk);

    // Encrypt test plaintext to get a Capsule
    let plaintext = b"test-data-for-reencryption";
    let (capsule, _ciphertext) = umbral_pre::encrypt(&delegating_pk, plaintext).unwrap();
    let capsule_bytes = bincode::serialize(&capsule).unwrap();

    // Generate kFrags (k=1, n=1 for simple tests)
    let verified_kfrags =
        umbral_pre::generate_kfrags(&delegating_sk, &receiving_pk, &signer, 1, 1, true, true);
    let kfrag = verified_kfrags[0].clone().unverify();
    let kfrag_data = kfrag.to_bytes().unwrap().to_vec();

    let vd = VerificationData {
        verifying_pk: bincode::serialize(&verifying_pk).unwrap(),
        delegating_pk: bincode::serialize(&delegating_pk).unwrap(),
        receiving_pk: bincode::serialize(&receiving_pk).unwrap(),
    };
    let vd_bytes = bincode::serialize(&vd).unwrap();

    let payload = StoredKeyFragPayload {
        id: "kfrag-1".to_string(),
        key_data: kfrag_data,
        verification_data: vd_bytes,
        precursor: vec![],
    };
    let kfrag_bytes = bincode::serialize(&payload).unwrap();

    TestCryptoData {
        kfrag_bytes,
        kfrag_id: "kfrag-1".to_string(),
        capsule_bytes,
    }
}

#[tokio::test]
async fn test_mock_ao_client_new() {
    let client = MockAOClient::new();
    assert!(client.get_stored_kfrags("test").is_empty());
}

#[tokio::test]
async fn test_execute_delegate_kfrag() {
    let client = MockAOClient::new();
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3, 4], "holder-1");

    let result = client.execute("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.ok);

    let stored = client.get_stored_kfrags("process-1");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, "kfrag-1");
    assert_eq!(stored[0].1, vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn test_execute_delegate_capsule_with_reencryption() {
    let client = MockAOClient::new();
    let td = create_test_kfrag_and_capsule();

    // Step 1: DelegateKFrag
    let msg = AOExecuteMsg::delegate_kfrag(&td.kfrag_id, td.kfrag_bytes, "holder-1");
    let result = client.execute("process-1", msg).await;
    assert!(result.is_ok());

    // Step 2: DelegateCapsule — triggers real re-encryption
    let msg =
        AOExecuteMsg::delegate_capsule(&td.kfrag_id, "capsule-1", td.capsule_bytes, "holder-1");
    let result = client.execute("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.ok);

    // Verify capsule was stored at holder process
    let stored = client.get_stored_capsules("holder-1");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, "capsule-1");

    // Verify cFrag was generated via real Umbral re-encryption
    let cfrags = client.get_stored_cfrags("holder-1");
    assert_eq!(cfrags.len(), 1);
}

#[tokio::test]
async fn test_execute_reencrypt() {
    let client = MockAOClient::new();
    let td = create_test_kfrag_and_capsule();

    // Store kfrag at process-1
    let msg = AOExecuteMsg::delegate_kfrag(&td.kfrag_id, td.kfrag_bytes, "holder-1");
    client.execute("process-1", msg).await.unwrap();

    // Store capsule at process-1 (Reencrypt looks up both from same process)
    let msg = AOExecuteMsg::delegate_capsule(
        &td.kfrag_id,
        "capsule-1",
        td.capsule_bytes.clone(),
        "process-1",
    );
    client.execute("process-1", msg).await.unwrap();

    // Reencrypt on process-1
    let result = client
        .execute(
            "process-1",
            AOExecuteMsg::reencrypt(&td.kfrag_id, "capsule-1"),
        )
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.ok);

    let cfrags = client.get_stored_cfrags("process-1");
    assert!(cfrags.len() >= 1);
}

#[tokio::test]
async fn test_query_get_cfrag() {
    let client = MockAOClient::new();
    let td = create_test_kfrag_and_capsule();

    // DelegateKFrag + DelegateCapsule → triggers re-encryption → cFrag stored
    let msg = AOExecuteMsg::delegate_kfrag(&td.kfrag_id, td.kfrag_bytes, "process-1");
    client.execute("process-1", msg).await.unwrap();

    let msg =
        AOExecuteMsg::delegate_capsule(&td.kfrag_id, "capsule-1", td.capsule_bytes, "process-1");
    client.execute("process-1", msg).await.unwrap();

    // Query the generated cFrag
    let result = client
        .query(
            "process-1",
            AOQueryMsg::get_cfrag(&td.kfrag_id, "capsule-1"),
        )
        .await;

    assert!(result.is_ok());
    let binary = result.unwrap();
    let response: GetCFragResponse = serde_json::from_slice(binary.as_slice()).unwrap();
    assert!(!response.cfrag.is_empty());
}

#[tokio::test]
async fn test_query_get_cfrag_not_found() {
    let client = MockAOClient::new();

    let result = client
        .query("process-1", AOQueryMsg::get_cfrag("kfrag-1", "capsule-1"))
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("CFrag not ready"));
}

#[tokio::test]
async fn test_query_list_capsules_by_kfrag() {
    let client = MockAOClient::new();

    // Pre-register kfrag
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "process-1");
    client.execute("process-1", msg).await.unwrap();

    for i in 1..=5u8 {
        let msg =
            AOExecuteMsg::delegate_capsule("kfrag-1", format!("capsule-{i}"), vec![i], "process-1");
        // Will fail re-encryption (dummy kfrag), but capsule still gets stored
        let _ = client.execute("process-1", msg).await;
    }

    let result = client
        .query(
            "process-1",
            AOQueryMsg::list_capsules_by_kfrag("kfrag-1", None, Some(3)),
        )
        .await;

    assert!(result.is_ok());
    let binary = result.unwrap();
    let value: serde_json::Value = serde_json::from_slice(binary.as_slice()).unwrap();
    let capsule_ids = value["capsule_ids"].as_array().unwrap();
    assert_eq!(capsule_ids.len(), 3);
}

#[tokio::test]
async fn test_dry_run() {
    let client = MockAOClient::new();
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3, 4], "holder-1");

    let result = client.dry_run("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.ok);

    let stored = client.get_stored_kfrags("process-1");
    assert!(stored.is_empty());
}

#[tokio::test]
async fn test_error_injection() {
    let client = MockAOClient::new();
    client.inject_error(AOCommunicationError::connection_error("test error"));

    let result = client
        .execute(
            "process-1",
            AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1"),
        )
        .await;

    assert!(result.is_err());
    if let Err(AOCommunicationError::ConnectionError { details }) = result {
        assert_eq!(details, "test error");
    } else {
        panic!("Expected ConnectionError");
    }

    // Error injection is one-shot
    let result2 = client
        .execute(
            "process-1",
            AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1"),
        )
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_clear_error() {
    let client = MockAOClient::new();
    client.inject_error(AOCommunicationError::connection_error("test error"));
    client.clear_error();

    let result = client
        .execute(
            "process-1",
            AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1"),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_empty_process_id_error() {
    let client = MockAOClient::new();
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1");

    let result = client.execute("", msg).await;
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(AOCommunicationError::ValidationError { .. })
    ));
}

#[tokio::test]
async fn test_clear_storage() {
    let client = MockAOClient::new();

    client
        .execute(
            "process-1",
            AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1"),
        )
        .await
        .unwrap();

    assert!(!client.get_stored_kfrags("process-1").is_empty());

    client.clear();

    assert!(client.get_stored_kfrags("process-1").is_empty());
}

#[tokio::test]
async fn test_multiple_processes() {
    let client = MockAOClient::new();

    client
        .execute(
            "process-1",
            AOExecuteMsg::delegate_kfrag("kfrag-1", vec![1, 2, 3], "holder-1"),
        )
        .await
        .unwrap();

    client
        .execute(
            "process-2",
            AOExecuteMsg::delegate_kfrag("kfrag-2", vec![4, 5, 6], "holder-2"),
        )
        .await
        .unwrap();

    let kfrags1 = client.get_stored_kfrags("process-1");
    let kfrags2 = client.get_stored_kfrags("process-2");

    assert_eq!(kfrags1.len(), 1);
    assert_eq!(kfrags2.len(), 1);
    assert_eq!(kfrags1[0].0, "kfrag-1");
    assert_eq!(kfrags2[0].0, "kfrag-2");
}

#[tokio::test]
async fn test_ao_client_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockAOClient>();
}
