//! PHASE 3 Integration Tests - Secret Recovery Workflow
//!
//! This module tests the PHASE 3 workflow components using real CryptoService.
//! Storage operations return ResourceNotFound since Issue #47 (AO Network communication)
//! is not yet implemented.
//!
//! These tests focus on:
//! 1. CryptoService operations (public API) for decryption and reconstruction
//! 2. WorkflowService public API behavior

use std::sync::Arc;

use formix::adapter::external::mock_ao::MockAOClient;
use formix::domain::SecretId;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{
    CryptoService, CryptoServiceImpl as CoreCryptoServiceImpl, ShamirShare,
};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{SecretRecoveryWorkflowService, SecretRecoveryWorkflowServiceImpl};
use formix::usecase::{SecretRecoveryRequest, WorkflowError};

type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
type TestStorageService =
    ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

/// Helper to create test service with real CryptoService
fn create_integration_service(
) -> SecretRecoveryWorkflowServiceImpl<TestCryptoService, TestStorageService> {
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    SecretRecoveryWorkflowServiceImpl::new(crypto, storage)
}

// ============================================================================
// PHASE 3 Integration Tests (Task 21)
// ============================================================================

#[test]
fn test_phase3_integration_aes_decryption_roundtrip() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: AES Decryption Roundtrip");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Step 1] Generate symmetric key and encrypt shares");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    println!("  Generated {} byte symmetric key", symmetric_key.len());

    let original_shares = [
        b"share_data_1_for_testing".to_vec(),
        b"share_data_2_for_testing".to_vec(),
        b"share_data_3_for_testing".to_vec(),
    ];

    let encrypted_shares: Vec<Vec<u8>> = original_shares
        .iter()
        .map(|share| crypto.aes_gcm_encrypt(&symmetric_key, share).unwrap())
        .collect();

    println!("  Encrypted {} shares", encrypted_shares.len());
    for (i, enc) in encrypted_shares.iter().enumerate() {
        println!(
            "  Share[{}]: {} bytes -> {} bytes (encrypted)",
            i,
            original_shares[i].len(),
            enc.len()
        );
    }

    println!("\n[Step 2] Decrypt shares using CryptoService");
    for (i, encrypted) in encrypted_shares.iter().enumerate() {
        let decrypted = crypto.aes_gcm_decrypt(&symmetric_key, encrypted).unwrap();
        assert_eq!(decrypted, original_shares[i]);
        println!(
            "  Decrypted share[{}]: {:?}",
            i,
            String::from_utf8_lossy(&decrypted)
        );
    }

    println!("\n[PASS] AES decryption roundtrip successful");
}

#[test]
fn test_phase3_integration_aes_decryption_wrong_key() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: AES Decryption Wrong Key");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Step 1] Encrypt with correct key");
    let correct_key = crypto.generate_symmetric_key().unwrap();
    let wrong_key = crypto.generate_symmetric_key().unwrap();

    let encrypted = crypto
        .aes_gcm_encrypt(&correct_key, b"secret data")
        .unwrap();
    println!("  Encrypted data: {} bytes", encrypted.len());

    println!("\n[Step 2] Attempt decryption with wrong key");
    let result = crypto.aes_gcm_decrypt(&wrong_key, &encrypted);

    assert!(result.is_err());
    println!("  [PASS] Wrong key correctly rejected");

    println!("\n[PASS] Wrong key correctly rejected with DecryptionError");
}

#[test]
fn test_phase3_integration_shamir_reconstruction() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: Shamir Reconstruction");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let original_secret = b"Top secret message for Shamir!";
    println!(
        "\n[Step 1] Original secret: {:?}",
        String::from_utf8_lossy(original_secret)
    );

    println!("\n[Step 2] Split secret (k=3, n=5)");
    let shares = crypto.split_secret_shamir(original_secret, 3, 5).unwrap();
    println!("  Generated {} shares", shares.len());
    for share in &shares {
        println!(
            "  Share index {}: {} bytes",
            share.index,
            share.share_data.len()
        );
    }

    println!("\n[Step 3] Reconstruct with exactly k=3 shares");
    let subset: Vec<ShamirShare> = shares.clone().into_iter().take(3).collect();
    println!(
        "  Using shares with indices: {:?}",
        subset.iter().map(|s| s.index).collect::<Vec<_>>()
    );

    let reconstructed = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();
    assert_eq!(reconstructed, original_secret);
    println!(
        "  Reconstructed: {:?}",
        String::from_utf8_lossy(&reconstructed)
    );

    println!("\n[Step 4] Reconstruct with more than k shares (k+2=5)");
    let subset: Vec<ShamirShare> = shares.clone().into_iter().take(5).collect();
    let reconstructed = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();
    assert_eq!(reconstructed, original_secret);
    println!("  [PASS] Reconstruction with 5 shares also works");

    println!("\n[Step 5] Attempt reconstruction with insufficient shares (k-1=2)");
    let subset: Vec<ShamirShare> = shares.into_iter().take(2).collect();
    let result = crypto.reconstruct_secret_shamir(&subset, 3);
    assert!(result.is_err());
    println!("  [PASS] Insufficient shares correctly rejected");

    println!("\n[PASS] Shamir reconstruction works correctly");
}

#[test]
fn test_phase3_integration_shamir_various_thresholds() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: Various Shamir Thresholds");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let test_cases = [
        (2, 2, "minimum k=n=2"),
        (2, 5, "k=2, n=5"),
        (3, 5, "standard 3-of-5"),
        (5, 10, "k=5, n=10"),
        (2, 10, "k=2, n=10"),
    ];

    for (threshold, total, description) in test_cases {
        println!("\n[Test] {} (k={}, n={})", description, threshold, total);

        let original_secret = b"Test secret data";
        let shares = crypto
            .split_secret_shamir(original_secret, threshold, total)
            .unwrap();
        println!("  Split into {} shares", shares.len());

        // Reconstruct with exactly threshold
        let subset: Vec<ShamirShare> = shares.into_iter().take(threshold as usize).collect();
        let reconstructed = crypto
            .reconstruct_secret_shamir(&subset, threshold)
            .unwrap();

        assert_eq!(reconstructed, original_secret);
        println!("  [PASS] Reconstructed with {} shares", threshold);
    }

    println!("\n[PASS] All threshold combinations work correctly");
}

#[tokio::test]
async fn test_phase3_integration_execute_fails_at_storage() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: Execute Fails at Storage");
    println!("========================================");

    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    let service = SecretRecoveryWorkflowServiceImpl::new(service_crypto, storage);

    let (requester_sk, _requester_pk) = core_crypto.generate_keypair().unwrap();
    let (_, owner_pk) = core_crypto.generate_keypair().unwrap();
    let request = SecretRecoveryRequest {
        secret_id: SecretId::generate(),
        requester_secret_key: requester_sk,
        owner_public_key: owner_pk,
        requester_process_id: "requester-process-123".to_string(),
    };

    println!("  Request:");
    println!("    secret_id: {}", request.secret_id);
    println!("    requester_process_id: {}", request.requester_process_id);

    println!("\n  Executing PHASE 3 workflow...");
    let result = service.execute_secret_recovery(request).await;

    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    if let Err(WorkflowError::ResourceNotFound(msg)) = result {
        println!("  [EXPECTED] Failed at capsule retrieval: {}", msg);
    }

    println!("\n[PASS] Workflow correctly fails at capsule retrieval");
}

#[tokio::test]
async fn test_phase3_integration_can_recover_fails_at_storage() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: can_recover Fails at Storage");
    println!("========================================");

    let service = create_integration_service();
    let secret_id = SecretId::generate();

    println!("  Checking can_recover for secret_id: {}", secret_id);
    let result = service.can_recover(&secret_id, "requester-123").await;

    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    if let Err(WorkflowError::ResourceNotFound(msg)) = result {
        println!("  [EXPECTED] Failed at capsule retrieval: {}", msg);
    }

    println!("\n[PASS] can_recover correctly fails at capsule retrieval");
}

#[tokio::test]
async fn test_phase3_integration_validation_errors() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: Validation Errors");
    println!("========================================");

    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    let service = SecretRecoveryWorkflowServiceImpl::new(service_crypto, storage);

    println!("\n[Test 1] Empty requester process ID");
    let (requester_sk, _) = core_crypto.generate_keypair().unwrap();
    let (_, owner_pk) = core_crypto.generate_keypair().unwrap();
    let request = SecretRecoveryRequest {
        secret_id: SecretId::generate(),
        requester_secret_key: requester_sk,
        owner_public_key: owner_pk,
        requester_process_id: String::new(),
    };

    let result = service.execute_secret_recovery(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    if let Err(WorkflowError::ValidationError(msg)) = result {
        println!("  [PASS] Validation error: {}", msg);
    }

    println!("\n[PASS] Validation errors handled correctly");
}

#[test]
fn test_phase3_integration_crypto_service_keypair_generation() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: Keypair Generation");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Test] Generate multiple keypairs and verify uniqueness");
    let mut public_keys = Vec::new();

    for i in 0..5 {
        let (sk, pk) = crypto.generate_keypair().unwrap();
        println!(
            "  Keypair {}: sk exists, pk={} bytes",
            i + 1,
            pk.key_data.len()
        );
        assert!(!sk.is_empty());
        assert!(!pk.key_data.is_empty());
        public_keys.push(pk.key_data.clone());
    }

    // Verify all public keys are unique
    for i in 0..public_keys.len() {
        for j in (i + 1)..public_keys.len() {
            assert_ne!(
                public_keys[i], public_keys[j],
                "Public keys should be unique"
            );
        }
    }
    println!("  [PASS] All {} public keys are unique", public_keys.len());

    println!("\n[PASS] Keypair generation works correctly");
}

#[test]
fn test_phase3_integration_pre_capsule_creation() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: PRE Capsule Creation");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Step 1] Generate owner keypair");
    let (_owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    println!("  Owner public key: {} bytes", owner_pk.key_data.len());

    println!("\n[Step 2] Generate symmetric key");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    println!("  Symmetric key: {} bytes", symmetric_key.len());

    println!("\n[Step 3] Create PRE capsule");
    let (capsule, ciphertext) = crypto
        .create_pre_capsule(&owner_pk, &symmetric_key)
        .unwrap();
    println!("  Capsule: {} bytes", capsule.capsule_bytes.len());
    println!("  Ciphertext: {} bytes", ciphertext.len());

    assert!(!capsule.capsule_bytes.is_empty());
    assert!(!ciphertext.is_empty());

    println!("\n[PASS] PRE capsule creation works correctly");
}

#[test]
fn test_phase3_integration_kfrag_generation() {
    println!("\n========================================");
    println!("PHASE 3 Integration Test: kFrag Generation");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Step 1] Generate owner and requester keypairs");
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();
    println!("  Owner public key: {} bytes", owner_pk.key_data.len());
    println!(
        "  Requester public key: {} bytes",
        requester_pk.key_data.len()
    );

    println!("\n[Step 2] Generate reencryption key");
    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &requester_pk)
        .unwrap();
    println!("  Reencryption key generated");

    println!("\n[Step 3] Create kFrags (k=3, n=5)");
    let kfrags = crypto.create_kfrags(&reenc_key, 3, 5).unwrap();
    println!("  Generated {} kFrags", kfrags.len());

    assert_eq!(kfrags.len(), 5);
    for (i, kfrag) in kfrags.iter().enumerate() {
        assert!(!kfrag.key_data.is_empty());
        println!(
            "  kFrag[{}]: id={}, {} bytes",
            i,
            kfrag.id,
            kfrag.key_data.len()
        );
    }

    println!("\n[PASS] kFrag generation works correctly");
}
