use std::sync::Arc;

use formix::adapter::external::mock_ao::MockAOClient;
use formix::domain::SecretId;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{
    constants, CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::dto::SecretSharingRequest;
use formix::usecase::error::WorkflowError;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{SecretSharingWorkflowService, SecretSharingWorkflowServiceImpl};
use zeroize::Zeroizing;

type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
type TestStorageService =
    ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

fn create_test_service() -> SecretSharingWorkflowServiceImpl<TestCryptoService, TestStorageService>
{
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
    SecretSharingWorkflowServiceImpl::new(crypto, storage)
}

fn create_test_request(crypto: &CoreCryptoServiceImpl) -> SecretSharingRequest {
    let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
    let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();

    SecretSharingRequest {
        secret: Zeroizing::new(b"Test secret data for sharing".to_vec()),
        owner_secret_key: owner_sk,
        owner_public_key: owner_pk,
        requester_public_key: requester_pk,
        threshold: 3,
        total_shares: 5,
        owner_process_id: "owner-process-123".to_string(),
        metadata: None,
    }
}

#[tokio::test]
async fn test_phase1_returns_complete_result() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let request = create_test_request(&crypto);
    let owner_pk = request.owner_public_key.clone();

    let result = service.execute_secret_sharing(request).await;
    assert!(result.is_ok());

    let result = result.unwrap();

    assert!(!result.secret_id.as_str().is_empty());
    assert!(!result.capsule_tx_id.is_empty());
    assert_eq!(result.share_tx_ids.len(), 5);
    assert_eq!(result.kfrag_count, 5);
    assert_eq!(result.owner_public_key.key_data, owner_pk.key_data);
}

#[tokio::test]
async fn test_phase1_complete_flow() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let original_secret = b"This is a very important secret message!".to_vec();

    let mut request = create_test_request(&crypto);
    request.secret = Zeroizing::new(original_secret.clone());
    request.threshold = 3;
    request.total_shares = 5;

    let result = service.execute_secret_sharing(request).await;
    assert!(result.is_ok());

    let result = result.unwrap();
    assert_eq!(result.kfrag_count, 5);
    assert_eq!(result.share_tx_ids.len(), 5);

    for tx_id in &result.share_tx_ids {
        assert!(tx_id.starts_with("tx_"));
    }
    assert!(result.capsule_tx_id.starts_with("tx_"));
}

#[tokio::test]
async fn test_phase1_minimum_threshold() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let mut request = create_test_request(&crypto);
    request.threshold = constants::MIN_THRESHOLD;
    request.total_shares = constants::MIN_THRESHOLD;

    let result = service.execute_secret_sharing(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_phase1_max_secret_size() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let mut request = create_test_request(&crypto);

    // 63 bytes - DATA_SIZE is 64, 1 byte for length prefix
    request.secret = Zeroizing::new(vec![0xAB; 63]);

    let result = service.execute_secret_sharing(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_secret_sharing_validation_fails() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let mut request = create_test_request(&crypto);
    request.secret = Zeroizing::new(vec![]);

    let result = service.execute_secret_sharing(request).await;
    assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
}

#[tokio::test]
async fn test_get_secret_status_not_implemented() {
    let service = create_test_service();
    let secret_id = SecretId::generate();

    let result = service.get_secret_status(&secret_id).await;
    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

    if let Err(WorkflowError::ResourceNotFound(msg)) = result {
        assert!(msg.contains("not found") || msg.contains("not yet implemented"));
    }
}

#[tokio::test]
async fn test_execute_with_different_threshold_total_combinations() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();

    let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

    for (threshold, total) in combinations {
        let mut request = create_test_request(&crypto);
        request.threshold = threshold;
        request.total_shares = total;

        let result = service.execute_secret_sharing(request).await;
        assert!(
            result.is_ok(),
            "Failed for threshold={threshold}, total={total}"
        );

        let result = result.unwrap();
        assert_eq!(result.kfrag_count, total);
        assert_eq!(result.share_tx_ids.len(), total as usize);
    }
}

#[tokio::test]
async fn test_result_secret_id_is_unique() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();

    let request1 = create_test_request(&crypto);
    let request2 = create_test_request(&crypto);

    let result1 = service.execute_secret_sharing(request1).await.unwrap();
    let result2 = service.execute_secret_sharing(request2).await.unwrap();

    assert_ne!(result1.secret_id, result2.secret_id);
}
