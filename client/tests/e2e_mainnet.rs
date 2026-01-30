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

use d_tpres::adapter::external::ao_client::AOClient;
use d_tpres::adapter::external::ao_config::AOConfig;
use d_tpres::adapter::external::ao_message::{Binary, ExecuteMsg};
use d_tpres::adapter::external::data_item::ArweaveJWK;
use d_tpres::adapter::external::production_ao_client::ProductionAOClient;

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
    println!("Message: https://www.ao.link/#/message/{message_id}");
    println!("============================");
}

fn print_ao_link_entity(process_id: &str) {
    println!("=== AO Link Process ===");
    println!("Process: https://www.ao.link/#/entity/{process_id}");
    println!("=======================");
}

#[tokio::test]
#[ignore]
async fn test_mainnet_execute_returns_message_id() {
    let client = create_client();
    let process_id = mainnet_process_id();

    print_ao_link_entity(&process_id);

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "mainnet-kf-1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3, 4]),
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

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "mainnet-dry-kf-1".to_string(),
        kfrag: Binary::new(vec![10, 20]),
    };

    let result = client.dry_run(&process_id, msg).await;
    println!("dry_run result: {result:?}");

    match result {
        Ok(resp) => {
            assert!(resp.success);
            assert_eq!(resp.message_id, None);
            println!("Dry run succeeded (no message_id, as expected)");
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

    print_ao_link_entity(&process_id);
    println!();

    // Step 1: DelegateKFrag
    println!("--- Step 1: DelegateKFrag ---");
    let r1 = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "mainnet-flow-kf".to_string(),
                kfrag: Binary::new(vec![1, 2, 3]),
            },
        )
        .await
        .unwrap();
    if let Some(ref mid) = r1.message_id {
        print_ao_link_message(mid);
    }

    // Step 2: DelegateCapsule
    println!("--- Step 2: DelegateCapsule ---");
    let r2 = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateCapsule {
                kfrag_id: "mainnet-flow-kf".to_string(),
                capsule_id: "mainnet-flow-cap".to_string(),
                capsule: Binary::new(vec![4, 5, 6]),
            },
        )
        .await
        .unwrap();
    if let Some(ref mid) = r2.message_id {
        print_ao_link_message(mid);
    }

    // Step 3: Reencrypt
    println!("--- Step 3: Reencrypt ---");
    let r3 = client
        .execute(
            &process_id,
            ExecuteMsg::Reencrypt {
                kfrag_id: "mainnet-flow-kf".to_string(),
                capsule_id: "mainnet-flow-cap".to_string(),
            },
        )
        .await
        .unwrap();
    if let Some(ref mid) = r3.message_id {
        print_ao_link_message(mid);
    }

    println!();
    println!("=== All 3 steps completed. Verify each message on AO Link above. ===");
}
