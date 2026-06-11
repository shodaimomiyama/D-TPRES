//! HyperBEAM E2E Integration Test
//!
//! Verifies the complete share() + recover() flow against a real HyperBEAM node.
//! Requires: running HyperBEAM instance, wallet file, compiled WASM binary.
//!
//! Run: `cargo test -p formix --features hyperbeam test_hyperbeam_e2e -- --ignored --nocapture`
#![allow(clippy::large_futures)]

use std::collections::BTreeMap;
use std::sync::Arc;

use zeroize::Zeroizing;

use formix::adapter::external::ao::signer;
use formix::adapter::external::ao::wallet::ArweaveJWK;
use formix::adapter::external::ao::{AOClient, AOConfig, AOExecuteMsg, HyperBEAMClient};
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl as CoreCryptoServiceImpl};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{
    SecretRecoveryWorkflowService, SecretRecoveryWorkflowServiceImpl, SecretSharingWorkflowService,
    SecretSharingWorkflowServiceImpl,
};
use formix::usecase::{SecretRecoveryRequest, SecretSharingRequest};

// ─── Helper functions ────────────────────────────────────────────────────────

fn load_wallet() -> Option<ArweaveJWK> {
    let Ok(path) = std::env::var("WALLET_PATH") else {
        eprintln!("WALLET_PATH not set — skipping HyperBEAM E2E test");
        return None;
    };
    match ArweaveJWK::from_file(&path) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("Failed to load wallet from {path}: {e}");
            None
        }
    }
}

async fn check_hyperbeam_reachable(base_url: &str) -> bool {
    let url = format!("{base_url}/~meta@1.0/info");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();
    client
        .get(&url)
        .send()
        .await
        .is_ok_and(|resp| resp.status().is_success())
}

fn wasm_image_id() -> Option<String> {
    match std::env::var("WASM_IMAGE_ID") {
        Ok(id) if !id.trim().is_empty() => Some(id.trim().to_string()),
        _ => {
            eprintln!(
                "WASM_IMAGE_ID not set — run via ao/scripts/deploy-hyperbeam.sh, \
                 which caches the image and sets it"
            );
            None
        }
    }
}

async fn spawn_process(
    http: &reqwest::Client,
    base_url: &str,
    image_id: &str,
    scheduler_location: &str,
    rsa_key: &rsa::RsaPrivateKey,
    sig_name: &str,
) -> Result<String, String> {
    let url = format!("{base_url}/schedule");
    let seed = uuid::Uuid::new_v4().to_string();

    // Mirrors dev_process:test_aos_process — the canonical JSON-Iface stack
    // config. List/integer fields must carry ao-types annotations and
    // structured-field encoding; sending them as plain strings breaks
    // dev_stack (case_clause) and Multipass.
    let headers: BTreeMap<String, String> = BTreeMap::from([
        (
            "ao-types".to_string(),
            "device-stack=\"list\", passes=\"integer\", stack-keys=\"list\"".to_string(),
        ),
        ("authority".to_string(), scheduler_location.to_string()),
        ("device".to_string(), "process@1.0".to_string()),
        (
            "device-stack".to_string(),
            "\"WASI@1.0\", \"JSON-Iface@1.0\", \"WASM-64@1.0\", \"Multipass@1.0\"".to_string(),
        ),
        ("execution-device".to_string(), "stack@1.0".to_string()),
        ("image".to_string(), image_id.to_string()),
        ("output-prefix".to_string(), "wasm".to_string()),
        ("passes".to_string(), "2".to_string()),
        ("patch-from".to_string(), "/results/outbox".to_string()),
        ("scheduler".to_string(), scheduler_location.to_string()),
        ("scheduler-device".to_string(), "scheduler@1.0".to_string()),
        (
            "scheduler-location".to_string(),
            scheduler_location.to_string(),
        ),
        (
            "stack-keys".to_string(),
            "\"init\", \"compute\", \"snapshot\", \"normalize\"".to_string(),
        ),
        ("test-random-seed".to_string(), seed),
        ("type".to_string(), "Process".to_string()),
    ]);

    let signed = signer::sign_message(rsa_key, &headers, &[], sig_name)
        .map_err(|e| format!("sign_message failed: {e}"))?;

    let mut req = http.post(&url);
    for (name, value) in &headers {
        req = req.header(name.as_str(), value.as_str());
    }
    if !signed.content_digest_header.is_empty() {
        req = req.header("content-digest", &signed.content_digest_header);
    }
    let resp = req
        .header("signature", &signed.signature_header)
        .header("signature-input", &signed.signature_input_header)
        .send()
        .await
        .map_err(|e| format!("spawn POST failed: {e}"))?;

    let status = resp.status();
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp
        .text()
        .await
        .map_err(|e| format!("read spawn response: {e}"))?;

    if !status.is_success() {
        return Err(format!("spawn HTTP {status}: {body}"));
    }

    // Extract process ID from response headers or body
    let process_id = resp_headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "process-id" || k.to_lowercase() == "process")
        .map(|(_, v)| v.clone())
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| {
                    v.get("process")
                        .or_else(|| v.get("id"))
                        .or_else(|| v.get("process-id"))
                        .and_then(|p| p.as_str())
                        .map(ToString::to_string)
                })
        })
        .unwrap_or_else(|| body.trim().to_string());

    // Verify process exists via GET /{process_id}/compute with slot=0
    let verify_url = format!("{base_url}/{process_id}/compute");
    let verify_resp = http
        .get(&verify_url)
        .header("slot", "0")
        .send()
        .await
        .map_err(|e| format!("verify process GET failed: {e}"))?;

    if verify_resp.status().as_u16() >= 500 {
        return Err(format!(
            "process {process_id} verification failed: HTTP {}",
            verify_resp.status()
        ));
    }

    Ok(process_id)
}

// ─── Main E2E test ───────────────────────────────────────────────────────────

#[tokio::test]
#[ignore]
async fn test_hyperbeam_e2e_share_and_recover() {
    // 1. Load wallet
    let Some(wallet) = load_wallet() else {
        return;
    };

    // 2. Create AOConfig with 60s timeout
    let base_url =
        std::env::var("HYPERBEAM_URL").unwrap_or_else(|_| "http://localhost:10000".to_string());
    let config = AOConfig::new(&base_url, &base_url, &base_url, 60_000)
        .expect("AOConfig creation should not fail");

    // 3. Check HyperBEAM reachable
    if !check_hyperbeam_reachable(&base_url).await {
        eprintln!("HyperBEAM not reachable at {base_url} — skipping E2E test");
        return;
    }

    // 4. Image ID of the cached WASM module
    let Some(image_id) = wasm_image_id() else {
        return;
    };

    // 5. Extract signing material from wallet
    let rsa_key = wallet
        .to_rsa_private_key()
        .expect("wallet RSA key extraction failed");
    let sig_name = wallet.sig_name().expect("wallet sig_name failed");
    let address = wallet.address().expect("wallet address failed");

    // 6. Create reqwest::Client with 60s timeout
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("HTTP client creation failed");

    // 7. Spawn process referencing the cached image
    let process_id = spawn_process(&http, &base_url, &image_id, &address, &rsa_key, &sig_name)
        .await
        .expect("process spawn failed");
    eprintln!("Process spawned: {process_id}");

    // 9. Create HyperBEAMClient, send Init("Owner")
    let hb_client =
        Arc::new(HyperBEAMClient::new(config, &wallet).expect("HyperBEAMClient creation failed"));

    let init_resp = hb_client
        .execute(&process_id, AOExecuteMsg::init("Combined"))
        .await
        .expect("Init message failed");
    assert!(init_resp.ok, "Init should succeed: {:?}", init_resp.error);
    eprintln!("Process initialized as Combined");

    // 10. Wire services (same pattern as workflow_roundtrip_test.rs)
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(Arc::clone(
        &hb_client,
    )));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

    let sharing_service =
        SecretSharingWorkflowServiceImpl::new(Arc::clone(&service_crypto), Arc::clone(&storage));
    let recovery_service = SecretRecoveryWorkflowServiceImpl::new(service_crypto, storage);

    // 11. Generate keypairs
    let (owner_sk, owner_pk) = core_crypto.generate_keypair().unwrap();
    let (requester_sk, requester_pk) = core_crypto.generate_keypair().unwrap();

    let original_secret = b"HyperBEAM E2E test secret (k=3, n=5)!";
    let secret_data: Vec<u8> = original_secret.to_vec();

    // 12. Phase 1: execute_secret_sharing
    eprintln!("Executing Phase 1: secret sharing...");
    let phase1 = sharing_service
        .execute_secret_sharing(SecretSharingRequest {
            secret: Zeroizing::new(secret_data.clone()),
            owner_secret_key: owner_sk,
            owner_public_key: owner_pk.clone(),
            requester_public_key: requester_pk,
            threshold: 3,
            total_shares: 5,
            owner_process_id: process_id.clone(),
            metadata: None,
        })
        .await
        .expect("Phase 1 secret sharing failed");

    eprintln!(
        "Phase 1 complete: secret_id={}, kfrag_count={}, share_txs={}",
        phase1.secret_id,
        phase1.kfrag_count,
        phase1.share_tx_ids.len()
    );
    assert_eq!(phase1.kfrag_count, 5);
    assert_eq!(phase1.share_tx_ids.len(), 5);

    // 13. Phase 3: execute_secret_recovery
    eprintln!("Executing Phase 3: secret recovery...");
    let phase3 = recovery_service
        .execute_secret_recovery(SecretRecoveryRequest {
            secret_id: phase1.secret_id,
            requester_secret_key: requester_sk,
            owner_public_key: owner_pk,
            requester_process_id: process_id,
        })
        .await
        .expect("Phase 3 secret recovery failed");

    // 14. Verify recovered secret matches original
    assert_eq!(
        phase3.recovered_secret, secret_data,
        "Recovered secret must match original"
    );
    eprintln!("E2E SUCCESS: recovered secret matches original");
}
