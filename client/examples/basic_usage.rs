#![allow(clippy::expect_used, clippy::large_futures)]

use std::sync::Arc;

use formix::actions::{ActionError, FormixClient, InitConfig};
use formix::adapter::external::mock_ao::MockAOClient;
use formix::domain::value_objects::SecretId;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::dto::SecretMetadata;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("=== FORMIX Basic Usage Example ===\n");

    // --- Step 1: FormixClient::init() (not yet implemented) ---
    println!("--- Step 1: FormixClient::init() ---");
    let config = InitConfig {
        wallet_path: "wallet.json".to_string(),
        ao_gateway_url: None,
        arweave_gateway_url: None,
    };
    match FormixClient::init(config) {
        Ok(_) => println!("  Client initialized (unexpected)"),
        Err(e) => println!("  Expected error: {e}"),
    }
    println!();

    // --- Step 2: FormixClient::with_storage() (MockAO backend) ---
    println!("--- Step 2: Create client with MockAO storage ---");
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));

    let client = FormixClient::with_storage(
        "process_001".to_string(),
        "wallet_address_example".to_string(),
        "https://ao.arweave.net".to_string(),
        "https://arweave.net".to_string(),
        arweave,
        contract,
    );
    println!("  Process ID:   {}", client.process_id());
    println!("  Wallet:       {}", client.wallet_address());
    println!("  AO Gateway:   {}", client.ao_gateway_url());
    println!("  AR Gateway:   {}", client.arweave_gateway_url());
    println!();

    // --- Step 3: generate_keypair() ---
    println!("--- Step 3: Generate key pairs ---");
    let (owner_sk, owner_pk) = client.generate_keypair().expect("owner keypair");
    println!("  Owner  public key ({} bytes)", owner_pk.key_data.len());

    let (requester_sk, requester_pk) = client.generate_keypair().expect("requester keypair");
    println!(
        "  Requester public key ({} bytes)",
        requester_pk.key_data.len()
    );
    println!();

    // --- Step 4: share() builder (Phase 1) ---
    println!("--- Step 4: Share a secret (Phase 1) ---");
    let metadata = SecretMetadata {
        name: Some("example-secret".to_string()),
        description: Some("Demo secret for basic_usage example".to_string()),
        expires_at: None,
        tags: vec!["demo".to_string()],
    };

    let share_result = client
        .share()
        .secret(b"Hello, FORMIX!".to_vec())
        .threshold(2)
        .total_shares(3)
        .owner_key(owner_sk)
        .requester_key(requester_pk)
        .metadata(Some(metadata))
        .execute()
        .await;

    let sharing = match share_result {
        Ok(r) => {
            println!("  Secret ID:     {}", r.secret_id);
            println!("  Capsule TX:    {}", r.capsule_tx_id);
            println!("  Share TXs:     {:?}", r.share_tx_ids);
            println!("  kFrag count:   {}", r.kfrag_count);
            Some(r)
        }
        Err(e) => {
            println!("  Share failed: {e}");
            None
        }
    };
    println!();

    // --- Step 5: recover() builder (Phase 3) ---
    println!("--- Step 5: Recover the secret (Phase 3) ---");
    let secret_id = sharing
        .as_ref()
        .map_or_else(|| SecretId::new("fallback-id"), |s| s.secret_id.clone());

    let recover_result = client
        .recover()
        .secret_id(&secret_id)
        .requester_key(requester_sk)
        .owner_key(owner_pk)
        .execute()
        .await;

    match recover_result {
        Ok(r) => {
            println!(
                "  Recovered: {:?}",
                String::from_utf8_lossy(&r.recovered_secret)
            );
            println!("  Audit TX:  {}", r.audit_tx_id);
        }
        Err(ActionError::ResourceNotFound { resource }) => {
            println!("  Expected ResourceNotFound: {resource}");
            println!("  (MockStorage has no cFrag data for recovery)");
        }
        Err(e) => {
            println!("  Recovery error: {e}");
            println!("  (Expected with MockStorage — full recovery requires AO Network)");
        }
    }

    println!("\n=== Done ===");
}
