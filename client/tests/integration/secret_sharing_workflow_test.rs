//! PHASE 1 Integration Tests - Secret Sharing Workflow
//!
//! This module tests the complete PHASE 1 workflow using real CryptoService
//! and verifying that all cryptographic operations produce valid outputs.
#![allow(clippy::large_futures)]

use std::sync::Arc;

use zeroize::Zeroizing;

use formix::adapter::external::mock_ao::MockAOClient;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl as CoreCryptoServiceImpl};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{SecretSharingWorkflowService, SecretSharingWorkflowServiceImpl};
use formix::usecase::{SecretSharingRequest, WorkflowError};

type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
type TestStorageService =
    ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

/// Helper to create test service with real CryptoService
fn create_integration_service(
) -> SecretSharingWorkflowServiceImpl<TestCryptoService, TestStorageService> {
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    SecretSharingWorkflowServiceImpl::new(crypto, storage)
}

/// Helper to create a valid test request
fn create_valid_request(crypto: &CoreCryptoServiceImpl) -> SecretSharingRequest {
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();

    SecretSharingRequest {
        secret: Zeroizing::new(b"Integration test secret data".to_vec()),
        owner_secret_key: owner_sk,
        owner_public_key: owner_pk,
        requester_public_key: requester_pk,
        threshold: 3,
        total_shares: 5,
        owner_process_id: "owner-process-integration-test".to_string(),
        metadata: None,
    }
}

// ============================================================================
// PHASE 1 Integration Tests (Task 20)
// ============================================================================

#[tokio::test]
async fn test_phase1_integration_complete_flow() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Complete Flow");
    println!("========================================");

    let service = create_integration_service();
    let crypto = CoreCryptoServiceImpl::new();
    let original_secret = b"This is a highly confidential secret message for integration testing!";

    // Limit to 63 bytes (Shamir DATA_SIZE - 1)
    let secret_data: Vec<u8> = original_secret.iter().take(63).copied().collect();
    println!("\n[Step 1] Preparing request");
    println!("  Secret length: {} bytes", secret_data.len());
    println!(
        "  Secret preview: {:?}...",
        String::from_utf8_lossy(&secret_data[..20.min(secret_data.len())])
    );

    let mut request = create_valid_request(&crypto);
    request.secret = Zeroizing::new(secret_data.clone());
    request.threshold = 3;
    request.total_shares = 5;
    println!("  Threshold (k): {}", request.threshold);
    println!("  Total shares (n): {}", request.total_shares);

    println!("\n[Step 2] Executing PHASE 1 workflow");
    let result = service.execute_secret_sharing(request).await;

    assert!(
        result.is_ok(),
        "PHASE 1 workflow failed: {:?}",
        result.err()
    );
    let result = result.unwrap();

    println!("\n[Step 3] Verifying results");
    println!("  Secret ID: {}", result.secret_id);
    println!("  Capsule TX ID: {}", result.capsule_tx_id);
    println!("  kFrag count: {}", result.kfrag_count);
    println!("  Share TX IDs: {} items", result.share_tx_ids.len());

    // Verify all outputs
    assert!(
        !result.secret_id.as_str().is_empty(),
        "Secret ID should not be empty"
    );
    assert!(
        !result.capsule_tx_id.is_empty(),
        "Capsule TX ID should not be empty"
    );
    assert_eq!(
        result.kfrag_count, 5,
        "kFrag count should match total_shares"
    );
    assert_eq!(
        result.share_tx_ids.len(),
        5,
        "Share TX IDs count should match total_shares"
    );

    // Verify TX ID format
    for (i, tx_id) in result.share_tx_ids.iter().enumerate() {
        assert!(
            tx_id.starts_with("tx_"),
            "Share TX ID {} should have correct format",
            i
        );
        println!("  Share[{}] TX: {}", i, tx_id);
    }
    assert!(
        result.capsule_tx_id.starts_with("tx_"),
        "Capsule TX ID should have correct format"
    );

    println!("\n[PASS] PHASE 1 integration test completed successfully");
}

#[test]
fn test_phase1_integration_crypto_operations_valid() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Crypto Validity");
    println!("========================================");

    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto = Arc::clone(&core_crypto);
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    let _service = SecretSharingWorkflowServiceImpl::new(service_crypto, storage);

    println!("\n[Step 1] Testing symmetric key generation");
    let key1 = crypto.generate_symmetric_key().unwrap();
    let key2 = crypto.generate_symmetric_key().unwrap();
    assert_eq!(key1.len(), 32, "AES-256 key should be 32 bytes");
    assert_ne!(key1, key2, "Keys should be random");
    println!("  Generated two unique 32-byte keys");

    println!("\n[Step 2] Testing AES-GCM roundtrip");
    let plaintext = b"Test plaintext for AES-GCM";
    let ciphertext = crypto.aes_gcm_encrypt(&key1, plaintext).unwrap();
    let decrypted = crypto.aes_gcm_decrypt(&key1, &ciphertext).unwrap();
    assert_eq!(decrypted, plaintext, "Decrypted data should match original");
    println!("  AES-GCM encrypt/decrypt roundtrip successful");
    println!(
        "  Plaintext: {} bytes -> Ciphertext: {} bytes",
        plaintext.len(),
        ciphertext.len()
    );

    println!("\n[Step 3] Testing Shamir Secret Sharing roundtrip");
    let secret = b"Shamir test secret";
    let shares = crypto.split_secret_shamir(secret, 3, 5).unwrap();
    assert_eq!(shares.len(), 5, "Should generate 5 shares");
    println!("  Split secret into {} shares (threshold=3)", shares.len());

    // Reconstruct with exactly threshold shares
    let subset: Vec<_> = shares.into_iter().take(3).collect();
    let reconstructed = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();
    assert_eq!(
        reconstructed, secret,
        "Reconstructed secret should match original"
    );
    println!(
        "  Reconstructed secret with 3 shares: {:?}",
        String::from_utf8_lossy(&reconstructed)
    );

    println!("\n[Step 4] Testing PRE Capsule creation");
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    let (capsule, ciphertext) = crypto
        .create_pre_capsule(&owner_pk, &symmetric_key)
        .unwrap();
    assert!(
        !capsule.capsule_bytes.is_empty(),
        "Capsule should have data"
    );
    assert!(!ciphertext.is_empty(), "Ciphertext should not be empty");
    println!(
        "  Created PRE capsule: {} bytes",
        capsule.capsule_bytes.len()
    );
    println!("  Capsule ciphertext: {} bytes", ciphertext.len());

    println!("\n[Step 5] Testing kFrag generation");
    let (_req_sk, req_pk) = crypto.generate_keypair().unwrap();
    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &req_pk)
        .unwrap();
    let kfrags = crypto.create_kfrags(&reenc_key, 3, 5).unwrap();
    assert_eq!(kfrags.len(), 5, "Should generate 5 kFrags");
    for (i, kfrag) in kfrags.iter().enumerate() {
        assert!(!kfrag.key_data.is_empty(), "kFrag {} should have data", i);
        println!("  kFrag[{}]: {} bytes", i, kfrag.key_data.len());
    }

    println!("\n[PASS] All crypto operations produce valid outputs");
}

#[tokio::test]
async fn test_phase1_integration_various_threshold_combinations() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Threshold Combinations");
    println!("========================================");

    let service = create_integration_service();
    let crypto = CoreCryptoServiceImpl::new();

    let test_cases = [
        (2, 2, "minimum k=n"),
        (2, 3, "k < n"),
        (3, 5, "standard 3-of-5"),
        (5, 10, "larger threshold"),
        (2, 10, "low threshold, high n"),
        (10, 20, "max shares"),
    ];

    for (threshold, total, description) in test_cases {
        println!("\n[Test] {} (k={}, n={})", description, threshold, total);

        let mut request = create_valid_request(&crypto);
        request.threshold = threshold;
        request.total_shares = total;
        request.secret = Zeroizing::new(b"Test secret for threshold combo".to_vec());

        let result = service.execute_secret_sharing(request).await;
        assert!(
            result.is_ok(),
            "Failed for k={}, n={}: {:?}",
            threshold,
            total,
            result.err()
        );

        let result = result.unwrap();
        assert_eq!(result.kfrag_count, total, "kFrag count should be {}", total);
        assert_eq!(
            result.share_tx_ids.len(),
            total as usize,
            "Share count should be {}",
            total
        );

        println!(
            "  [PASS] Generated {} kFrags and {} shares",
            result.kfrag_count,
            result.share_tx_ids.len()
        );
    }

    println!("\n[PASS] All threshold combinations work correctly");
}

#[tokio::test]
async fn test_phase1_integration_secret_size_limits() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Secret Size Limits");
    println!("========================================");

    let service = create_integration_service();
    let crypto = CoreCryptoServiceImpl::new();

    let test_sizes = [
        (1, "minimum 1 byte"),
        (10, "small 10 bytes"),
        (32, "32 bytes (AES key size)"),
        (63, "maximum 63 bytes"),
    ];

    for (size, description) in test_sizes {
        println!("\n[Test] {} ({} bytes)", description, size);

        let mut request = create_valid_request(&crypto);
        request.secret = Zeroizing::new(vec![0xAB; size]);

        let result = service.execute_secret_sharing(request).await;
        assert!(
            result.is_ok(),
            "Failed for size {}: {:?}",
            size,
            result.err()
        );

        let result = result.unwrap();
        println!("  Secret ID: {}", result.secret_id);
        println!("  [PASS] Successfully processed {} byte secret", size);
    }

    println!("\n[PASS] All secret sizes within limits work correctly");
}

#[tokio::test]
async fn test_phase1_integration_unique_outputs() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Output Uniqueness");
    println!("========================================");

    let service = create_integration_service();
    let crypto = CoreCryptoServiceImpl::new();

    println!("\n[Step 1] Executing workflow multiple times");
    let mut secret_ids = Vec::new();
    let mut capsule_tx_ids = Vec::new();

    for i in 0..5 {
        let request = create_valid_request(&crypto);
        let result = service.execute_secret_sharing(request).await.unwrap();

        println!("  Execution {}: secret_id={}", i + 1, result.secret_id);
        secret_ids.push(result.secret_id.as_str().to_string());
        capsule_tx_ids.push(result.capsule_tx_id.clone());
    }

    println!("\n[Step 2] Verifying uniqueness");

    // Check secret IDs are unique
    let unique_secret_ids: std::collections::HashSet<_> = secret_ids.iter().collect();
    assert_eq!(
        unique_secret_ids.len(),
        secret_ids.len(),
        "All secret IDs should be unique"
    );
    println!("  [PASS] All {} secret IDs are unique", secret_ids.len());

    // Check capsule TX IDs are unique
    let unique_capsule_ids: std::collections::HashSet<_> = capsule_tx_ids.iter().collect();
    assert_eq!(
        unique_capsule_ids.len(),
        capsule_tx_ids.len(),
        "All capsule TX IDs should be unique"
    );
    println!(
        "  [PASS] All {} capsule TX IDs are unique",
        capsule_tx_ids.len()
    );

    println!("\n[PASS] All outputs are unique across executions");
}

#[tokio::test]
async fn test_phase1_integration_validation_errors() {
    println!("\n========================================");
    println!("PHASE 1 Integration Test: Validation Errors");
    println!("========================================");

    let service = create_integration_service();
    let crypto = CoreCryptoServiceImpl::new();

    println!("\n[Test 1] Empty secret");
    let mut request = create_valid_request(&crypto);
    request.secret = Zeroizing::new(vec![]);
    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    println!("  [PASS] Empty secret rejected");

    println!("\n[Test 2] Threshold below minimum");
    let mut request = create_valid_request(&crypto);
    request.threshold = 1;
    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    println!("  [PASS] Threshold=1 rejected");

    println!("\n[Test 3] Threshold exceeds total");
    let mut request = create_valid_request(&crypto);
    request.threshold = 10;
    request.total_shares = 5;
    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    println!("  [PASS] Threshold > total rejected");

    println!("\n[Test 4] Total shares exceeds maximum");
    let mut request = create_valid_request(&crypto);
    request.total_shares = 255;
    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    println!("  [PASS] Total shares > 20 rejected");

    println!("\n[Test 5] Empty owner process ID");
    let mut request = create_valid_request(&crypto);
    request.owner_process_id = String::new();
    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    println!("  [PASS] Empty process ID rejected");

    // Note: Invalid public key test is done in unit tests within the crate
    // since PublicKey is #[non_exhaustive] and cannot be constructed externally

    println!("\n[PASS] All validation errors handled correctly");
}
