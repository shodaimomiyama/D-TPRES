//! PHASE 1 → PHASE 3 Roundtrip Integration Tests
//!
//! This module tests the complete workflow from secret sharing (PHASE 1)
//! to secret recovery (PHASE 3), verifying end-to-end correctness.
//!
//! Note: Storage operations are mocked since Issue #47 (AO Network communication)
//! is not yet implemented. The tests focus on cryptographic correctness using
//! only public APIs.
#![allow(clippy::large_futures)]

use std::sync::Arc;

use zeroize::Zeroizing;

use formix::adapter::external::mock_ao::MockAOClient;
use formix::usecase::SecretSharingRequest;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{
    CFragData, CryptoService, CryptoServiceImpl as CoreCryptoServiceImpl, ShamirShare,
};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{SecretSharingWorkflowService, SecretSharingWorkflowServiceImpl};

// ============================================================================
// PHASE 1 → PHASE 3 Roundtrip Tests (Task 22)
// ============================================================================

#[test]
fn test_roundtrip_shamir_reconstruction() {
    println!("\n========================================");
    println!("Roundtrip Test: Shamir Secret Reconstruction");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let original_secret = b"Top secret message for roundtrip test!";
    let secret_data: Vec<u8> = original_secret.to_vec();
    println!(
        "\n[Step 1] Original secret: {:?}",
        String::from_utf8_lossy(&secret_data)
    );
    println!("  Secret length: {} bytes", secret_data.len());

    println!("\n[Step 2] PHASE 1 - Split secret using Shamir (k=3, n=5)");
    let shares = crypto.split_secret_shamir(&secret_data, 3, 5).unwrap();
    println!("  Generated {} Shamir shares", shares.len());
    for share in &shares {
        println!(
            "  Share index {}: {} bytes",
            share.index,
            share.share_data.len()
        );
    }

    println!("\n[Step 3] PHASE 3 - Reconstruct with exactly k=3 shares");
    let subset: Vec<ShamirShare> = shares.clone().into_iter().take(3).collect();
    println!(
        "  Using shares with indices: {:?}",
        subset.iter().map(|s| s.index).collect::<Vec<_>>()
    );

    let reconstructed = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();
    assert_eq!(reconstructed, secret_data);
    println!(
        "  Reconstructed: {:?}",
        String::from_utf8_lossy(&reconstructed)
    );

    println!("\n[Step 4] Verify with different share combinations");
    // Try with different 3-share combinations
    let combinations = [vec![0, 1, 2], vec![0, 2, 4], vec![1, 3, 4], vec![2, 3, 4]];

    for indices in combinations {
        let subset: Vec<ShamirShare> = indices.iter().map(|&i| shares[i].clone()).collect();
        let result = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();
        assert_eq!(result, secret_data);
        println!("  [PASS] Combination {:?} reconstructed correctly", indices);
    }

    println!("\n[PASS] Shamir roundtrip successful with all combinations");
}

#[test]
fn test_roundtrip_aes_encryption_decryption() {
    println!("\n========================================");
    println!("Roundtrip Test: AES-GCM Encryption/Decryption");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let test_data = [
        b"Short message".to_vec(),
        b"This is a longer message for testing AES-GCM encryption".to_vec(),
        vec![0xAB; 100], // Binary data
        vec![0x00; 32],  // Zero-filled data (edge case)
    ];

    println!("\n[Step 1] Generate symmetric key");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    println!("  Generated {} byte AES-256 key", symmetric_key.len());

    for (i, original) in test_data.iter().enumerate() {
        println!("\n[Test {}] Data size: {} bytes", i + 1, original.len());

        let encrypted = crypto.aes_gcm_encrypt(&symmetric_key, original).unwrap();
        println!("  Encrypted size: {} bytes", encrypted.len());

        let decrypted = crypto.aes_gcm_decrypt(&symmetric_key, &encrypted).unwrap();
        assert_eq!(&decrypted, original);
        println!("  [PASS] Roundtrip successful");
    }

    println!("\n[PASS] All AES-GCM roundtrip tests passed");
}

#[test]
fn test_roundtrip_complete_phase1_to_phase3_crypto_flow() {
    println!("\n========================================");
    println!("Roundtrip Test: Complete PHASE 1 → PHASE 3 Crypto Flow");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let original_secret = b"Complete workflow test secret!";
    let secret_data: Vec<u8> = original_secret.to_vec();
    println!(
        "\n[Original Secret] {:?}",
        String::from_utf8_lossy(&secret_data)
    );

    // ========================================
    // PHASE 1: Secret Sharing
    // ========================================
    println!("\n--- PHASE 1: Secret Sharing ---");

    println!("\n[P1-1] Generate keypairs");
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();
    println!(
        "  Owner keypair generated: {} bytes public key",
        owner_pk.key_data.len()
    );
    println!(
        "  Requester keypair generated: {} bytes public key",
        requester_pk.key_data.len()
    );

    println!("\n[P1-2] Generate symmetric key and encrypt secret");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    let encrypted_secret = crypto
        .aes_gcm_encrypt(&symmetric_key, &secret_data)
        .unwrap();
    println!("  Symmetric key: {} bytes", symmetric_key.len());
    println!("  Encrypted secret: {} bytes", encrypted_secret.len());

    println!("\n[P1-3] Create PRE capsule");
    let (capsule, capsule_ciphertext) = crypto
        .create_pre_capsule(&owner_pk, &symmetric_key)
        .unwrap();
    println!("  Capsule created: {} bytes", capsule.capsule_bytes.len());
    println!("  Capsule ciphertext: {} bytes", capsule_ciphertext.len());

    println!("\n[P1-4] Split symmetric key with Shamir (k=3, n=5)");
    let key_shares = crypto.split_secret_shamir(&symmetric_key, 3, 5).unwrap();
    println!("  Generated {} key shares", key_shares.len());

    println!("\n[P1-5] Generate reencryption key and kFrags for PRE");
    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &requester_pk)
        .unwrap();
    let kfrags = crypto.create_kfrags(&reenc_key, 3, 5).unwrap();
    println!("  Generated {} kFrags", kfrags.len());

    // ========================================
    // PHASE 3: Secret Recovery (Simulated)
    // ========================================
    println!("\n--- PHASE 3: Secret Recovery ---");

    println!("\n[P3-1] Collect Shamir key shares (simulating k=3 collection)");
    let collected_shares: Vec<ShamirShare> = key_shares.into_iter().take(3).collect();
    println!(
        "  Collected {} shares with indices: {:?}",
        collected_shares.len(),
        collected_shares.iter().map(|s| s.index).collect::<Vec<_>>()
    );

    println!("\n[P3-2] Reconstruct symmetric key from shares");
    let recovered_key = crypto
        .reconstruct_secret_shamir(&collected_shares, 3)
        .unwrap();
    assert_eq!(recovered_key, symmetric_key);
    println!("  [PASS] Symmetric key reconstructed correctly");

    println!("\n[P3-3] Decrypt the original secret");
    let recovered_secret = crypto
        .aes_gcm_decrypt(&recovered_key, &encrypted_secret)
        .unwrap();
    assert_eq!(recovered_secret, secret_data);
    println!(
        "  [PASS] Secret recovered: {:?}",
        String::from_utf8_lossy(&recovered_secret)
    );

    println!("\n[PASS] Complete PHASE 1 → PHASE 3 roundtrip successful!");
}

#[test]
fn test_roundtrip_various_threshold_combinations() {
    println!("\n========================================");
    println!("Roundtrip Test: Various Threshold Combinations");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let test_cases = [
        (2, 2, "minimum k=n=2"),
        (2, 5, "k=2, n=5"),
        (3, 5, "standard 3-of-5"),
        (5, 10, "k=5, n=10"),
        (3, 10, "k=3, n=10"),
        (10, 20, "maximum supported"),
    ];

    let original_secret = b"Threshold test secret data";

    for (threshold, total, description) in test_cases {
        println!("\n[Test] {} (k={}, n={})", description, threshold, total);

        // PHASE 1: Split
        let shares = crypto
            .split_secret_shamir(original_secret, threshold, total)
            .unwrap();
        println!("  Split: {} shares created", shares.len());

        // PHASE 3: Reconstruct with exactly threshold shares
        let subset: Vec<ShamirShare> = shares.into_iter().take(threshold as usize).collect();
        let reconstructed = crypto
            .reconstruct_secret_shamir(&subset, threshold)
            .unwrap();

        assert_eq!(reconstructed, original_secret.to_vec());
        println!("  [PASS] Reconstructed with {} shares", threshold);
    }

    println!("\n[PASS] All threshold combinations work correctly");
}

#[test]
fn test_roundtrip_secret_size_variations() {
    println!("\n========================================");
    println!("Roundtrip Test: Secret Size Variations");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let test_sizes = [
        (1, "minimum 1 byte"),
        (16, "16 bytes (AES block)"),
        (32, "32 bytes (AES-256 key)"),
        (48, "48 bytes"),
        (63, "maximum 63 bytes"),
    ];

    for (size, description) in test_sizes {
        println!("\n[Test] {} ({} bytes)", description, size);

        let original_secret: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

        // PHASE 1: Split with Shamir (k=3, n=5)
        let shares = crypto.split_secret_shamir(&original_secret, 3, 5).unwrap();
        println!("  Split into {} shares", shares.len());

        // PHASE 3: Reconstruct
        let subset: Vec<ShamirShare> = shares.into_iter().take(3).collect();
        let reconstructed = crypto.reconstruct_secret_shamir(&subset, 3).unwrap();

        assert_eq!(reconstructed, original_secret);
        println!("  [PASS] {} byte secret roundtrip successful", size);
    }

    println!("\n[PASS] All secret sizes handled correctly");
}

#[test]
fn test_roundtrip_multiple_secrets_isolated() {
    println!("\n========================================");
    println!("Roundtrip Test: Multiple Secrets Isolation");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let secrets = [
        b"First secret message".to_vec(),
        b"Second secret message".to_vec(),
        b"Third secret message".to_vec(),
    ];

    println!("\n[Step 1] Split all secrets independently");
    let mut all_shares: Vec<Vec<ShamirShare>> = Vec::new();

    for (i, secret) in secrets.iter().enumerate() {
        let shares = crypto.split_secret_shamir(secret, 2, 3).unwrap();
        println!(
            "  Secret {}: {:?} -> {} shares",
            i + 1,
            String::from_utf8_lossy(secret),
            shares.len()
        );
        all_shares.push(shares);
    }

    println!("\n[Step 2] Reconstruct each secret with its own shares");
    for (i, (secret, shares)) in secrets.iter().zip(all_shares.iter()).enumerate() {
        let subset: Vec<ShamirShare> = shares.clone().into_iter().take(2).collect();
        let reconstructed = crypto.reconstruct_secret_shamir(&subset, 2).unwrap();

        assert_eq!(&reconstructed, secret);
        println!("  Secret {}: [PASS] Reconstructed correctly", i + 1);
    }

    println!("\n[Step 3] Verify cross-contamination doesn't occur");
    // Mix shares from different secrets - should NOT reconstruct correctly
    let mixed_shares = vec![all_shares[0][0].clone(), all_shares[1][1].clone()];
    let mixed_result = crypto.reconstruct_secret_shamir(&mixed_shares, 2);
    // The result will be garbage, not matching any original secret
    if let Ok(result) = mixed_result {
        assert_ne!(result, secrets[0]);
        assert_ne!(result, secrets[1]);
        println!("  [PASS] Mixed shares produce incorrect result (expected)");
    } else {
        println!("  [PASS] Mixed shares reconstruction failed (expected)");
    }

    println!("\n[PASS] Multiple secrets remain isolated");
}

#[test]
fn test_roundtrip_keypair_generation_consistency() {
    println!("\n========================================");
    println!("Roundtrip Test: Keypair Generation Consistency");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    println!("\n[Step 1] Generate multiple keypairs");
    let mut keypairs = Vec::new();
    for i in 0..5 {
        let (sk, pk) = crypto.generate_keypair().unwrap();
        println!(
            "  Keypair {}: pk={} bytes, sk exists",
            i + 1,
            pk.key_data.len()
        );
        keypairs.push((sk, pk));
    }

    println!("\n[Step 2] Verify all keypairs are unique");
    for i in 0..keypairs.len() {
        for j in (i + 1)..keypairs.len() {
            assert_ne!(keypairs[i].1.key_data, keypairs[j].1.key_data);
        }
    }
    println!("  [PASS] All {} public keys are unique", keypairs.len());

    println!("\n[Step 3] Verify keypair operations work independently");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();

    for (i, (owner_sk, owner_pk)) in keypairs.iter().enumerate() {
        let (_, requester_pk) = crypto.generate_keypair().unwrap();

        // Create capsule with this owner's key
        let (capsule, _) = crypto.create_pre_capsule(owner_pk, &symmetric_key).unwrap();
        assert!(!capsule.capsule_bytes.is_empty());

        // Generate kFrags
        let reenc_key = crypto
            .generate_reencryption_key(owner_sk, &requester_pk)
            .unwrap();
        let kfrags = crypto.create_kfrags(&reenc_key, 2, 3).unwrap();
        assert_eq!(kfrags.len(), 3);

        println!("  Keypair {}: [PASS] Capsule and kFrags generated", i + 1);
    }

    println!("\n[PASS] All keypairs work correctly and independently");
}

#[tokio::test]
async fn test_roundtrip_workflow_services_integration() {
    println!("\n========================================");
    println!("Roundtrip Test: Workflow Services Integration");
    println!("========================================");

    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    let sharing_service = SecretSharingWorkflowServiceImpl::new(service_crypto, storage);

    let (owner_sk, owner_pk) = core_crypto.generate_keypair().unwrap();
    let (_requester_sk, requester_pk) = core_crypto.generate_keypair().unwrap();

    let original_secret = b"Integration test secret!";
    let secret_data: Vec<u8> = original_secret.to_vec();

    println!("\n[Step 1] Create SecretSharingRequest");
    let request = SecretSharingRequest {
        secret: Zeroizing::new(secret_data.clone()),
        owner_secret_key: owner_sk,
        owner_public_key: owner_pk.clone(),
        requester_public_key: requester_pk,
        threshold: 3,
        total_shares: 5,
        owner_process_id: "integration-test-owner".to_string(),
        metadata: None,
    };
    println!(
        "  Request created with threshold={}, total_shares={}",
        request.threshold, request.total_shares
    );

    println!("\n[Step 2] Execute PHASE 1 workflow");
    let phase1_result = sharing_service
        .execute_secret_sharing(request)
        .await
        .unwrap();
    println!("  Secret ID: {}", phase1_result.secret_id);
    println!("  Capsule TX: {}", phase1_result.capsule_tx_id);
    println!("  kFrag count: {}", phase1_result.kfrag_count);
    println!("  Share TX count: {}", phase1_result.share_tx_ids.len());

    println!("\n[Step 3] Verify PHASE 1 outputs");
    assert!(!phase1_result.secret_id.as_str().is_empty());
    assert_eq!(phase1_result.kfrag_count, 5);
    assert_eq!(phase1_result.share_tx_ids.len(), 5);
    println!("  [PASS] PHASE 1 outputs valid");

    println!("\n[Step 4] Test crypto operations that PHASE 3 would use");

    // Test Shamir reconstruction (simulate key shares)
    let key_shares = core_crypto
        .split_secret_shamir(&core_crypto.generate_symmetric_key().unwrap(), 3, 5)
        .unwrap();
    let key_subset: Vec<ShamirShare> = key_shares.into_iter().take(3).collect();
    let _ = core_crypto
        .reconstruct_secret_shamir(&key_subset, 3)
        .unwrap();
    println!("  [PASS] Shamir reconstruction works");

    // Test AES decryption
    let sym_key = core_crypto.generate_symmetric_key().unwrap();
    let encrypted = core_crypto.aes_gcm_encrypt(&sym_key, &secret_data).unwrap();
    let decrypted = core_crypto.aes_gcm_decrypt(&sym_key, &encrypted).unwrap();
    assert_eq!(decrypted, secret_data);
    println!("  [PASS] AES decryption works");

    println!("\n[PASS] Workflow services integration successful");
}

/// Full PRE roundtrip: Phase 1 → Phase 2 → Phase 3
///
/// Verifies the complete Proxy Re-Encryption flow including Shamir + AES layers.
/// Uses a single CryptoServiceImpl instance so that signer/verifying_key are consistent
/// across kFrag generation (Phase 1) and cFrag verification (Phase 2/3).
#[test]
fn test_full_pre_roundtrip_phase1_phase2_phase3() {
    println!("\n========================================");
    println!("Full PRE Roundtrip: Phase 1 → Phase 2 → Phase 3");
    println!("========================================");

    let crypto = Arc::new(CoreCryptoServiceImpl::new());

    let original_secret = b"End-to-end PRE roundtrip test secret!";
    let secret_data: Vec<u8> = original_secret.to_vec();
    let threshold: u8 = 3;
    let total: u8 = 5;

    // ========================================
    // PHASE 1: Secret Sharing & PRE Setup
    // ========================================
    println!("\n--- PHASE 1: Secret Sharing & PRE Setup ---");

    println!("\n[P1-1] Generate keypairs");
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (requester_sk, requester_pk) = crypto.generate_keypair().unwrap();

    println!("\n[P1-2] Generate symmetric key and encrypt original secret with AES");
    let symmetric_key = crypto.generate_symmetric_key().unwrap();
    let encrypted_secret = crypto
        .aes_gcm_encrypt(&symmetric_key, &secret_data)
        .unwrap();
    println!("  Symmetric key: {} bytes", symmetric_key.len());
    println!("  Encrypted secret: {} bytes", encrypted_secret.len());

    println!(
        "\n[P1-3] Split original secret via Shamir (k={}, n={})",
        threshold, total
    );
    let shares = crypto
        .split_secret_shamir(&secret_data, threshold, total)
        .unwrap();
    println!("  Generated {} Shamir shares", shares.len());

    println!("\n[P1-4] AES-encrypt each Shamir share with the symmetric key");
    let encrypted_shares: Vec<Vec<u8>> = shares
        .iter()
        .map(|share| {
            crypto
                .aes_gcm_encrypt(&symmetric_key, &share.share_data)
                .unwrap()
        })
        .collect();
    println!("  Encrypted {} shares", encrypted_shares.len());

    println!("\n[P1-5] Create PRE capsule (encrypts symmetric key via Umbral)");
    let (capsule, capsule_ciphertext) = crypto
        .create_pre_capsule(&owner_pk, &symmetric_key)
        .unwrap();
    println!("  Capsule: {} bytes", capsule.capsule_bytes.len());
    println!("  Capsule ciphertext: {} bytes", capsule_ciphertext.len());

    println!("\n[P1-6] Generate reencryption key and kFrags");
    let reenc_key = crypto
        .generate_reencryption_key(&owner_sk, &requester_pk)
        .unwrap();
    let kfrags = crypto.create_kfrags(&reenc_key, threshold, total).unwrap();
    println!(
        "  Generated {} kFrags (threshold={})",
        kfrags.len(),
        threshold
    );

    // ========================================
    // PHASE 2: Proxy Re-Encryption
    // ========================================
    println!("\n--- PHASE 2: Proxy Re-Encryption ---");

    println!(
        "\n[P2-1] Proxy re-encrypt with threshold ({}) kFrags",
        threshold
    );
    let cipher_fragments: Vec<_> = kfrags
        .iter()
        .take(threshold as usize)
        .enumerate()
        .map(|(i, kfrag)| {
            let cfrag = crypto.proxy_reencrypt(kfrag, &capsule).unwrap();
            println!(
                "  cFrag {} (id={}): capsule_fragment={} bytes",
                i,
                cfrag.fragment_id,
                cfrag.capsule_fragment.len()
            );
            cfrag
        })
        .collect();
    println!("  Generated {} cFrags", cipher_fragments.len());

    // ========================================
    // PHASE 3: Secret Recovery via PRE
    // ========================================
    println!("\n--- PHASE 3: Secret Recovery via PRE ---");

    println!("\n[P3-1] Convert CipherFragment → CFragData");
    let cfrag_data_list: Vec<CFragData> = cipher_fragments
        .iter()
        .enumerate()
        .map(|(i, cf)| CFragData::new(cf.capsule_fragment.clone(), format!("holder-{}", i)))
        .collect();
    println!("  Prepared {} CFragData entries", cfrag_data_list.len());

    println!("\n[P3-2] Get verifying key bytes");
    let verifying_pk_bytes = crypto.verifying_key_bytes().unwrap();
    println!("  Verifying key: {} bytes", verifying_pk_bytes.len());

    println!("\n[P3-3] Decrypt PRE capsule → recover symmetric key");
    let recovered_symmetric_key = crypto
        .decrypt_pre_capsule(
            &capsule,
            &cfrag_data_list,
            &requester_sk,
            &owner_pk,
            &capsule_ciphertext,
            &verifying_pk_bytes,
        )
        .unwrap();
    assert_eq!(
        recovered_symmetric_key, symmetric_key,
        "Recovered symmetric key must match original"
    );
    println!(
        "  [PASS] Symmetric key recovered via PRE ({} bytes)",
        recovered_symmetric_key.len()
    );

    println!("\n[P3-4] AES-decrypt Shamir shares and verify integrity");
    for (i, enc_share) in encrypted_shares.iter().enumerate() {
        let decrypted = crypto
            .aes_gcm_decrypt(&recovered_symmetric_key, enc_share)
            .unwrap();
        assert_eq!(
            decrypted, shares[i].share_data,
            "Decrypted share {} must match original share data",
            i
        );
    }
    println!(
        "  [PASS] All {} decrypted shares match originals",
        encrypted_shares.len()
    );

    println!(
        "\n[P3-5] Reconstruct original secret via Shamir (using {} of {} shares)",
        threshold, total
    );
    let collected: Vec<ShamirShare> = shares.into_iter().take(threshold as usize).collect();
    let recovered_secret = crypto
        .reconstruct_secret_shamir(&collected, threshold)
        .unwrap();
    assert_eq!(
        recovered_secret, secret_data,
        "Recovered secret must match original"
    );
    println!(
        "  [PASS] Secret recovered: {:?}",
        String::from_utf8_lossy(&recovered_secret)
    );

    // ========================================
    // Additional: Verify AES path also works
    // ========================================
    println!("\n--- Additional: Verify AES decryption path ---");
    let aes_recovered = crypto
        .aes_gcm_decrypt(&recovered_symmetric_key, &encrypted_secret)
        .unwrap();
    assert_eq!(aes_recovered, secret_data);
    println!("  [PASS] AES-encrypted secret also decrypted correctly");

    println!("\n[PASS] Full PRE roundtrip Phase 1 → Phase 2 → Phase 3 successful!");
}
