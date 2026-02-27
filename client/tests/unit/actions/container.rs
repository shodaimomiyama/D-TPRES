use std::sync::Arc;

use formix::actions::error::ActionError;
use formix::actions::{DefaultActionsContainer, ShareOptions};
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

#[tokio::test]
async fn test_share_valid_params() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret data".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process_123".to_string(),
            None,
        )
        .await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert!(!result.secret_id.as_str().is_empty());
    assert_eq!(result.kfrag_count, 5);
}

#[tokio::test]
async fn test_share_empty_secret() {
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
            "owner_process_123".to_string(),
            None,
        )
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ActionError::ValidationFailed { code, .. } => {
            assert!(code.contains("secret") || code.contains("empty"));
        }
        _ => panic!("Expected ValidationFailed"),
    }
}

#[tokio::test]
async fn test_share_zero_threshold() {
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
        _ => panic!("Expected ValidationFailed"),
    }
}

#[tokio::test]
async fn test_share_threshold_exceeds_total() {
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
        _ => panic!("Expected ValidationFailed"),
    }
}

#[tokio::test]
async fn test_share_result_has_valid_secret_id() {
    let container = create_test_container();
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

    assert!(!result.secret_id.as_str().is_empty());
}

#[tokio::test]
async fn test_share_result_kfrag_count_matches() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let result = container
        .share(
            b"test secret".to_vec(),
            3,
            7,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.kfrag_count, 7);
}

#[tokio::test]
async fn test_share_with_metadata() {
    let container = create_test_container();
    let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
    let (_, requester_pk) = container.generate_keypair().unwrap();

    let metadata = SecretMetadata {
        name: Some("test secret".to_string()),
        description: Some("A test secret".to_string()),
        expires_at: Some(1_735_689_600),
        tags: vec!["test".to_string()],
    };
    let options = ShareOptions::with_metadata(metadata);

    let result = container
        .share(
            b"test secret".to_vec(),
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
async fn test_share_various_threshold_combinations() {
    let container = create_test_container();

    let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

    for (threshold, total) in combinations {
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
            .await;

        assert!(
            result.is_ok(),
            "Failed for threshold={}, total={}",
            threshold,
            total
        );
        assert_eq!(result.unwrap().kfrag_count, total);
    }
}

#[tokio::test]
async fn test_recover_empty_secret_id() {
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
        _ => panic!("Expected ValidationFailed"),
    }
}

#[tokio::test]
async fn test_recover_empty_process_id() {
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
        _ => panic!("Expected ValidationFailed"),
    }
}

#[tokio::test]
async fn test_recover_nonexistent_secret() {
    let container = DefaultActionsContainer::new();
    let (requester_sk, _) = container.generate_keypair().unwrap();
    let (_, owner_pk) = container.generate_keypair().unwrap();

    let result = container
        .recover(
            "nonexistent_secret_id",
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

#[test]
fn test_generate_keypair_valid() {
    let container = DefaultActionsContainer::new();
    let result = container.generate_keypair();
    assert!(result.is_ok());
}

#[test]
fn test_generate_keypair_secret_key_not_empty() {
    let container = DefaultActionsContainer::new();
    let (secret_key, _) = container.generate_keypair().unwrap();
    assert!(!secret_key.is_empty());
}

#[test]
fn test_generate_keypair_public_key_not_empty() {
    let container = DefaultActionsContainer::new();
    let (_, public_key) = container.generate_keypair().unwrap();
    assert!(!public_key.key_data.is_empty());
}

#[test]
fn test_generate_keypair_unique() {
    let container = DefaultActionsContainer::new();

    let (_, pk1) = container.generate_keypair().unwrap();
    let (_, pk2) = container.generate_keypair().unwrap();

    assert_ne!(pk1.key_data, pk2.key_data);
}

#[tokio::test]
async fn test_generate_keypair_usable_with_share() {
    let container = create_test_container();

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
        .await;

    assert!(result.is_ok());
}
