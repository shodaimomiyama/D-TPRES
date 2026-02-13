#![cfg(feature = "production-ao")]
//! ArLocal E2E tests for ProductionAOClient
//!
//! ## Important: cwao-units vs ao-localnet
//!
//! The `ao/` directory uses `cwao-units`, which does NOT provide ao-connect
//! compatible HTTP APIs (no `/dry-run`, `/result/{id}` returns empty, etc.).
//! ProductionAOClient requires ao-connect compatible endpoints.
//!
//! Use **ao-localnet** (Docker Compose) instead:
//!   <https://github.com/permaweb/ao-localnet/>
//!
//! ## Setup (ao-localnet)
//!
//! ```sh
//! git clone https://github.com/permaweb/ao-localnet.git
//! cd ao-localnet
//! cd wallets && ./generateAll.sh && ./printWalletAddresses.mjs && cd ..
//! docker compose up --detach
//! cd seed && ./download-aos-module.sh && ./seed-for-aos.sh
//! ```
//!
//! ## Run
//!
//! ```sh
//! AO_TEST_PROCESS_ID=<PROCESS_ID> \
//! AO_MU_URL=http://localhost:4002 \
//! AO_CU_URL=http://localhost:4004 \
//! AO_GATEWAY_URL=http://localhost:4000 \
//! ARWEAVE_WALLET_PATH=<path/to/wallet.json> \
//! cargo test --test e2e_arlocal --features production-ao -- --ignored --nocapture --test-threads=1
//! ```

#![allow(clippy::unwrap_used)]

use d_tpres::adapter::external::ao::{
    AOClient, AOConfig, ArweaveJWK, Binary, ExecuteMsg, ProductionAOClient, QueryMsg,
};

fn arlocal_config() -> AOConfig {
    let mu = std::env::var("AO_MU_URL").unwrap_or_else(|_| "http://localhost:4002".to_string());
    let cu = std::env::var("AO_CU_URL").unwrap_or_else(|_| "http://localhost:4004".to_string());
    let gw =
        std::env::var("AO_GATEWAY_URL").unwrap_or_else(|_| "http://localhost:4000".to_string());
    AOConfig::new(&mu, &cu, &gw, 30_000).unwrap()
}

fn test_process_id() -> String {
    std::env::var("AO_TEST_PROCESS_ID")
        .expect("Set AO_TEST_PROCESS_ID env var to a deployed process ID")
}

fn test_wallet() -> ArweaveJWK {
    let path = std::env::var("ARWEAVE_WALLET_PATH").unwrap_or_else(|_| {
        let default = "../ao/.cwao/accounts/mu.json";
        default.to_string()
    });
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("Wallet not found at {path}. Run ao/scripts/start.js first."));
    serde_json::from_str(&json).unwrap()
}

fn create_client() -> ProductionAOClient {
    let config = arlocal_config();
    let jwk = test_wallet();
    ProductionAOClient::new(config, &jwk).unwrap()
}

#[tokio::test]
#[ignore]
async fn test_arlocal_execute_delegate_kfrag() {
    let client = create_client();
    let process_id = test_process_id();

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "e2e-kfrag-1".to_string(),
        kfrag: Binary::new(vec![10, 20, 30, 40]),
    };

    let result = client.execute(&process_id, msg).await;
    println!("execute result: {result:?}");
    assert!(result.is_ok());

    let resp = result.unwrap();
    assert!(resp.success);
    assert!(resp.message_id.is_some());
    println!(
        "message_id: {}",
        resp.message_id.as_deref().unwrap_or("none")
    );
}

#[tokio::test]
#[ignore]
async fn test_arlocal_dry_run_delegate_kfrag() {
    let client = create_client();
    let process_id = test_process_id();

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "e2e-dry-kfrag-1".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };

    let result = client.dry_run(&process_id, msg).await;
    println!("dry_run result: {result:?}");
    assert!(result.is_ok());

    let resp = result.unwrap();
    assert!(resp.success);
    assert_eq!(resp.message_id, None);
}

#[tokio::test]
#[ignore]
async fn test_arlocal_query_get_cfrag() {
    let client = create_client();
    let process_id = test_process_id();

    // First: delegate kfrag + capsule + reencrypt
    let _ = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "e2e-q-kfrag".to_string(),
                kfrag: Binary::new(vec![1, 2, 3]),
            },
        )
        .await;

    let _ = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateCapsule {
                kfrag_id: "e2e-q-kfrag".to_string(),
                capsule_id: "e2e-q-cap".to_string(),
                capsule: Binary::new(vec![4, 5, 6]),
            },
        )
        .await;

    let _ = client
        .execute(
            &process_id,
            ExecuteMsg::Reencrypt {
                kfrag_id: "e2e-q-kfrag".to_string(),
                capsule_id: "e2e-q-cap".to_string(),
            },
        )
        .await;

    // Query
    let msg = QueryMsg::GetCFrag {
        kfrag_id: "e2e-q-kfrag".to_string(),
        capsule_id: "e2e-q-cap".to_string(),
    };

    let result = client.query(&process_id, msg).await;
    println!("query result: {result:?}");
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_arlocal_invalid_process_id() {
    let client = create_client();

    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kf-1".to_string(),
        kfrag: Binary::new(vec![1]),
    };

    let result = client.execute("nonexistent-process-id-12345", msg).await;
    println!("invalid process result: {result:?}");
    assert!(result.is_err());
}

#[tokio::test]
#[ignore]
async fn test_arlocal_full_kfrag_delegation_flow() {
    let client = create_client();
    let process_id = test_process_id();

    // Step 1: Delegate kFrag
    let r1 = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "e2e-flow-kf".to_string(),
                kfrag: Binary::new(vec![100, 200]),
            },
        )
        .await
        .unwrap();
    println!("Step 1 (DelegateKFrag) message_id: {:?}", r1.message_id);
    assert!(r1.success);

    // Step 2: Delegate Capsule
    let r2 = client
        .execute(
            &process_id,
            ExecuteMsg::DelegateCapsule {
                kfrag_id: "e2e-flow-kf".to_string(),
                capsule_id: "e2e-flow-cap".to_string(),
                capsule: Binary::new(vec![50, 60, 70]),
            },
        )
        .await
        .unwrap();
    println!("Step 2 (DelegateCapsule) message_id: {:?}", r2.message_id);
    assert!(r2.success);

    // Step 3: Reencrypt
    let r3 = client
        .execute(
            &process_id,
            ExecuteMsg::Reencrypt {
                kfrag_id: "e2e-flow-kf".to_string(),
                capsule_id: "e2e-flow-cap".to_string(),
            },
        )
        .await
        .unwrap();
    println!("Step 3 (Reencrypt) message_id: {:?}", r3.message_id);
    assert!(r3.success);

    // Step 4: Query cFrag
    let cfrag_data = client
        .query(
            &process_id,
            QueryMsg::GetCFrag {
                kfrag_id: "e2e-flow-kf".to_string(),
                capsule_id: "e2e-flow-cap".to_string(),
            },
        )
        .await
        .unwrap();
    println!("Step 4 (GetCFrag) data length: {}", cfrag_data.len());
    assert!(!cfrag_data.is_empty());
}
