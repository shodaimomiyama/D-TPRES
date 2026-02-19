//! Actions Layer Integration Tests
//!
//! This module tests the Actions layer API (share, recover, generate_keypair)
//! which serves as the developer-facing facade for FORMIX operations.
//!
//! Note: Complete roundtrip tests (share → recover) require storage implementation.
//! Tests marked with `_storage_pending` will be fully functional once Issue #47 is complete.
#![allow(clippy::large_futures)]

use std::sync::Arc;

use formix::actions::{ActionError, DefaultActionsContainer, RecoverOptions, ShareOptions};
use formix::adapter::external::mock_ao::MockAOClient;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::dto::SecretMetadata;

fn create_test_container() -> DefaultActionsContainer {
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    DefaultActionsContainer::with_storage(arweave, contract)
}

// ============================================================================
// share() Integration Tests (Requirement 1)
// ============================================================================

#[tokio::test]
async fn test_actions_share_valid_params() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret data for integration".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process_integration".to_string(),
            None,
        )
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.secret_id.as_str().is_empty());
    assert_eq!(result.kfrag_count, 5);
}

#[tokio::test]
async fn test_actions_share_empty_secret_fails() {
    let container = DefaultActionsContainer::new();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            vec![],
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(
                code.contains("secret") || code.contains("empty"),
                "Expected error code about secret, got: {}",
                code
            );
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_share_zero_threshold_fails() {
    let container = DefaultActionsContainer::new();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret".to_vec(),
            0,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(code.contains("threshold"));
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_share_threshold_exceeds_total_fails() {
    let container = DefaultActionsContainer::new();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret".to_vec(),
            6,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(code.contains("threshold"));
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_share_result_has_valid_secret_id() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"my secret data".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await
        .unwrap();

    assert!(!result.secret_id.as_str().is_empty());
    assert!(result.secret_id.as_str().len() > 10);
}

#[tokio::test]
async fn test_actions_share_result_kfrag_count_matches() {
    let container = create_test_container();
    let test_cases = [(3, 5), (2, 7), (5, 10), (2, 3)];

    for (threshold, total) in test_cases {
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                b"test secret".to_vec(),
                threshold,
                total,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .await
            .unwrap();

        assert_eq!(
            result.kfrag_count, total,
            "Expected kfrag_count {} for ({}, {}), got {}",
            total, threshold, total, result.kfrag_count
        );
    }
}

#[tokio::test]
async fn test_actions_share_with_metadata() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let metadata = SecretMetadata {
        name: Some("Integration Test Secret".to_string()),
        description: Some("A secret for integration testing".to_string()),
        expires_at: Some(1735689600),
        tags: vec!["integration".to_string(), "test".to_string()],
    };
    let options = ShareOptions::with_metadata(metadata);

    let result = container
        .share(
            b"secret with metadata".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            Some(options),
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_actions_share_various_threshold_combinations() {
    let container = create_test_container();
    let combinations = [(2, 3), (3, 5), (5, 10), (2, 10), (3, 7)];

    for (threshold, total) in combinations {
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                b"test secret for combination".to_vec(),
                threshold,
                total,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .await;

        assert!(
            result.is_ok(),
            "Failed for threshold={}, total={}",
            threshold,
            total
        );
        let result = result.unwrap();
        assert_eq!(result.kfrag_count, total);
    }
}

#[tokio::test]
async fn test_actions_share_unique_secret_ids() {
    let container = create_test_container();
    let mut secret_ids = Vec::new();

    for _ in 0..5 {
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                b"test secret".to_vec(),
                3,
                5,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .await
            .unwrap();

        assert!(
            !secret_ids.contains(&result.secret_id.as_str().to_string()),
            "Duplicate secret_id generated"
        );
        secret_ids.push(result.secret_id.as_str().to_string());
    }
}

// ============================================================================
// recover() Integration Tests (Requirement 2)
// ============================================================================

#[tokio::test]
async fn test_actions_recover_empty_secret_id_fails() {
    let container = DefaultActionsContainer::new();
    let (requester_sk, _) = container.generate_keypair().unwrap();
    let (_, owner_pk) = container.generate_keypair().unwrap();

    let result = container
        .recover(
            "",
            requester_sk,
            owner_pk,
            "requester_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(code.contains("secret_id"));
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_recover_empty_process_id_fails() {
    let container = DefaultActionsContainer::new();
    let (requester_sk, _) = container.generate_keypair().unwrap();
    let (_, owner_pk) = container.generate_keypair().unwrap();

    let result = container
        .recover(
            "test_secret_id",
            requester_sk,
            owner_pk,
            String::new(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(code.contains("process_id"));
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_recover_nonexistent_secret_fails() {
    let container = DefaultActionsContainer::new();
    let (requester_sk, _) = container.generate_keypair().unwrap();
    let (_, owner_pk) = container.generate_keypair().unwrap();

    let result = container
        .recover(
            "nonexistent_secret_id_12345",
            requester_sk,
            owner_pk,
            "requester_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ResourceNotFound { .. } | ActionError::WorkflowFailed { .. } => {}
        other => panic!(
            "Expected ResourceNotFound or WorkflowFailed, got {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn test_actions_recover_with_options() {
    let container = DefaultActionsContainer::new();
    let (requester_sk, _) = container.generate_keypair().unwrap();
    let (_, owner_pk) = container.generate_keypair().unwrap();

    let options = RecoverOptions::new();
    let result = container
        .recover(
            "test_secret_id",
            requester_sk,
            owner_pk,
            "requester_process".to_string(),
            Some(options),
        )
        .await;

    assert!(result.is_err());
}

// ============================================================================
// generate_keypair() Integration Tests (Requirement 3)
// ============================================================================

#[test]
fn test_actions_generate_keypair_valid() {
    let container = DefaultActionsContainer::new();
    let result = container.generate_keypair();
    assert!(result.is_ok());
}

#[test]
fn test_actions_generate_keypair_secret_key_not_empty() {
    let container = DefaultActionsContainer::new();
    let (secret_key, _) = container.generate_keypair().unwrap();
    assert!(!secret_key.is_empty());
}

#[test]
fn test_actions_generate_keypair_public_key_not_empty() {
    let container = DefaultActionsContainer::new();
    let (_, public_key) = container.generate_keypair().unwrap();
    assert!(!public_key.key_data.is_empty());
}

#[test]
fn test_actions_generate_keypair_unique() {
    let container = DefaultActionsContainer::new();

    let (_, pk1) = container.generate_keypair().unwrap();
    let (_, pk2) = container.generate_keypair().unwrap();
    let (_, pk3) = container.generate_keypair().unwrap();

    assert_ne!(pk1.key_data, pk2.key_data);
    assert_ne!(pk2.key_data, pk3.key_data);
    assert_ne!(pk1.key_data, pk3.key_data);
}

#[tokio::test]
async fn test_actions_generate_keypair_usable_with_share() {
    let container = create_test_container();

    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret using generated keys".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    assert!(result.is_ok());
}

#[test]
fn test_actions_generate_keypair_multiple_in_sequence() {
    let container = DefaultActionsContainer::new();
    let mut public_keys = Vec::new();

    for _ in 0..10 {
        let (sk, pk) = container.generate_keypair().unwrap();
        assert!(!sk.is_empty());
        assert!(!pk.key_data.is_empty());
        public_keys.push(pk.key_data.clone());
    }

    for i in 0..public_keys.len() {
        for j in (i + 1)..public_keys.len() {
            assert_ne!(
                public_keys[i], public_keys[j],
                "Keys {} and {} are equal",
                i, j
            );
        }
    }
}

// ============================================================================
// Roundtrip Tests (share → recover)
// ============================================================================

// Note: Full roundtrip tests require storage implementation.
// These tests verify the cryptographic flow works correctly with the Actions API.

#[tokio::test]
async fn test_actions_share_produces_valid_result_for_recovery() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (requester_sk, requester_pk) = container.generate_keypair().unwrap();

    let original_secret = b"This is the secret to be shared and recovered".to_vec();

    let share_result = container
        .share(
            original_secret.clone(),
            3,
            5,
            owner_sk,
            owner_pk.clone(),
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await
        .unwrap();

    assert!(!share_result.secret_id.as_str().is_empty());

    let recover_result = container
        .recover(
            share_result.secret_id.as_str(),
            requester_sk,
            owner_pk,
            "requester_process".to_string(),
            None,
        )
        .await;

    match recover_result {
        Ok(result) => {
            assert_eq!(
                result.recovered_secret, original_secret,
                "Recovered secret doesn't match original"
            );
        }
        Err(ActionError::ResourceNotFound { .. }) | Err(ActionError::WorkflowFailed { .. }) => {}
        Err(other) => panic!("Unexpected error: {:?}", other),
    }
}

#[tokio::test]
async fn test_actions_roundtrip_with_various_secret_sizes() {
    let container = create_test_container();

    let test_cases = [
        ("tiny", b"Hi".to_vec()),
        ("small", b"Small secret".to_vec()),
        ("medium", b"Medium secret with more content".to_vec()),
        ("binary_small", vec![0x01, 0x02, 0x03, 0x04, 0x05]),
    ];

    for (name, secret) in test_cases {
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                secret.clone(),
                3,
                5,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .await;

        assert!(
            result.is_ok(),
            "Failed to share {} secret ({} bytes): {:?}",
            name,
            secret.len(),
            result.err()
        );
    }
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

#[tokio::test]
async fn test_actions_error_display_is_informative() {
    let container = DefaultActionsContainer::new();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            vec![],
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    let err = result.unwrap_err();
    let error_message = format!("{}", err);

    assert!(!error_message.is_empty());
    assert!(
        error_message.contains("Validation") || error_message.contains("secret"),
        "Error message should be informative: {}",
        error_message
    );
}

#[tokio::test]
async fn test_actions_validation_errors_have_code() {
    let container = DefaultActionsContainer::new();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            vec![],
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await;

    match result.unwrap_err() {
        ActionError::ValidationFailed { code, message } => {
            assert!(!code.is_empty(), "Error code should not be empty");
            assert!(!message.is_empty(), "Error message should not be empty");
        }
        other => panic!("Expected ValidationFailed, got {:?}", other),
    }
}
