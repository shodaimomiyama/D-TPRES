#![cfg(feature = "production-ao")]
//! AO Mainnet (Testnet) E2E tests for ProductionAOClient
//!
//! Prerequisites:
//!   1. Set ARWEAVE_WALLET_PATH to a funded Arweave JWK wallet file
//!   2. Set AO_MAINNET_PROCESS_ID to a deployed process ID on AO testnet
//!   3. Deploy contract: cd ao && node scripts/deploy.js
//!   4. Instantiate: cd ao && node scripts/instantiate.js
//!
//! Run: cargo test --test e2e_mainnet -- --ignored --nocapture --test-threads=1

#![allow(clippy::unwrap_used)]

use serde::{Deserialize, Serialize};

use d_tpres::adapter::external::ao_client::AOClient;
use d_tpres::adapter::external::ao_config::AOConfig;
use d_tpres::adapter::external::ao_message::{Binary, ExecuteMsg, QueryMsg};
use d_tpres::adapter::external::data_item::ArweaveJWK;
use d_tpres::adapter::external::production_ao_client::ProductionAOClient;
use d_tpres::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

fn mainnet_config() -> AOConfig {
    AOConfig::default()
}

fn mainnet_process_id() -> String {
    std::env::var("AO_MAINNET_PROCESS_ID")
        .expect("Set AO_MAINNET_PROCESS_ID env var to a deployed process ID on AO testnet")
}

fn mainnet_wallet() -> ArweaveJWK {
    let path = std::env::var("ARWEAVE_WALLET_PATH")
        .expect("Set ARWEAVE_WALLET_PATH env var to your funded JWK wallet file");
    let json = std::fs::read_to_string(&path).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn create_client() -> ProductionAOClient {
    let config = mainnet_config();
    let jwk = mainnet_wallet();
    ProductionAOClient::new(config, &jwk).unwrap()
}

fn print_ao_link_message(message_id: &str) {
    println!("=== AO Link Verification ===");
    println!("Message: https://aolink.ar.io/#/message/{message_id}");
    println!("============================");
}

fn print_ao_link_entity(process_id: &str) {
    println!("=== AO Link Process ===");
    println!("Process: https://aolink.ar.io/#/entity/{process_id}");
    println!("=======================");
}

// Mirrors ao/contracts/src/handlers.rs::StoredKeyFrag for bincode serialization
#[derive(Serialize, Deserialize, Clone, Debug)]
struct StoredKeyFrag {
    id: u8,
    key_data: Vec<u8>,
    verification_data: Vec<u8>,
    #[serde(default)]
    precursor: Vec<u8>,
}

struct CryptoFixture {
    kfrag_bytes: Vec<u8>,
    capsule_bytes: Vec<u8>,
}

fn build_crypto_fixture() -> CryptoFixture {
    let crypto = CryptoServiceImpl::new();
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (_accessor_sk, accessor_pk) = crypto.generate_keypair().unwrap();
    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &accessor_pk)
        .unwrap();
    let kfrags = crypto.create_kfrags(&reenc_key, 2, 2).unwrap();
    let (capsule, _ciphertext) = crypto
        .create_pre_capsule(&owner_pk, b"e2e test payload")
        .unwrap();

    let stored = StoredKeyFrag {
        id: kfrags[0].id,
        key_data: kfrags[0].key_data.clone(),
        verification_data: kfrags[0].verification_data.clone(),
        precursor: kfrags[0].precursor.clone(),
    };

    CryptoFixture {
        kfrag_bytes: bincode::serialize(&stored).unwrap(),
        capsule_bytes: capsule.capsule_bytes.clone(),
    }
}

#[tokio::test]
#[ignore]
async fn test_mainnet_execute_returns_message_id() {
    let client = create_client();
    let process_id = mainnet_process_id();
    let fixture = build_crypto_fixture();

    print_ao_link_entity(&process_id);

    let msg = ExecuteMsg::SubmitKFrag {
        kfrag_id: "mainnet-kf-1".to_string(),
        kfrag: Binary::new(fixture.kfrag_bytes),
    };

    let result = client.execute(&process_id, msg).await;
    println!("execute result: {result:?}");

    match result {
        Ok(resp) => {
            assert!(resp.success);
            assert!(resp.message_id.is_some(), "message_id should be present");
            let mid = resp.message_id.unwrap();
            print_ao_link_message(&mid);
        }
        Err(e) => {
            panic!("Execute failed: {e}");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_mainnet_dry_run() {
    let client = create_client();
    let process_id = mainnet_process_id();
    let fixture = build_crypto_fixture();

    let msg = ExecuteMsg::SubmitKFrag {
        kfrag_id: "mainnet-dry-kf-1".to_string(),
        kfrag: Binary::new(fixture.kfrag_bytes),
    };

    let result = client.dry_run(&process_id, msg).await;
    println!("dry_run result: {result:?}");

    match result {
        Ok(resp) => {
            assert!(resp.success);
            assert!(resp.message_id.is_some(), "dry_run now returns message_id via MU");
            println!("Dry run succeeded (message_id: {:?})", resp.message_id);
        }
        Err(e) => {
            panic!("Dry run failed: {e}");
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_mainnet_full_flow_with_ao_link() {
    let client = create_client();
    let process_id = mainnet_process_id();
    let fixture = build_crypto_fixture();

    print_ao_link_entity(&process_id);
    println!();

    let kfrag_id = "mainnet-flow-kf".to_string();
    let capsule_id = "mainnet-flow-cap".to_string();

    // Step 1: SubmitKFrag — store real kFrag data
    println!("--- Step 1: SubmitKFrag ---");
    let r1 = client
        .execute(
            &process_id,
            ExecuteMsg::SubmitKFrag {
                kfrag_id: kfrag_id.clone(),
                kfrag: Binary::new(fixture.kfrag_bytes),
            },
        )
        .await
        .unwrap();
    assert!(r1.success, "SubmitKFrag failed: {r1:?}");
    if let Some(ref mid) = r1.message_id {
        print_ao_link_message(mid);
    }

    // Step 2: SubmitCapsule — store capsule + auto-reencrypt + generate cFrag
    println!("--- Step 2: SubmitCapsule ---");
    let r2 = client
        .execute(
            &process_id,
            ExecuteMsg::SubmitCapsule {
                kfrag_id: kfrag_id.clone(),
                capsule_id: capsule_id.clone(),
                capsule: Binary::new(fixture.capsule_bytes),
            },
        )
        .await
        .unwrap();
    assert!(r2.success, "SubmitCapsule failed: {r2:?}");
    if let Some(ref mid) = r2.message_id {
        print_ao_link_message(mid);
    }

    // Step 3: GetCFrag — retrieve generated cFrag via query
    println!("--- Step 3: GetCFrag (query) ---");
    let query_result = client
        .query(
            &process_id,
            QueryMsg::GetCFrag {
                kfrag_id,
                capsule_id,
            },
        )
        .await;
    println!("GetCFrag result: {query_result:?}");

    match query_result {
        Ok(binary) => {
            println!(
                "cFrag retrieved successfully ({} bytes)",
                binary.len()
            );
        }
        Err(e) => {
            panic!("GetCFrag query failed: {e}");
        }
    }

    println!();
    println!("=== Full flow completed: SubmitKFrag -> SubmitCapsule (auto-reencrypt) -> GetCFrag ===");
}
