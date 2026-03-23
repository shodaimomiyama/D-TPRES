use std::sync::Arc;

use formix::adapter::external::mock_ao::MockAOClient;
use formix::domain::SecretId;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{
    CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::dto::SecretRecoveryRequest;
use formix::usecase::error::WorkflowError;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::{SecretRecoveryWorkflowService, SecretRecoveryWorkflowServiceImpl};

type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
type TestStorageService =
    ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

fn create_test_service() -> SecretRecoveryWorkflowServiceImpl<TestCryptoService, TestStorageService>
{
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));

    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

    SecretRecoveryWorkflowServiceImpl::new(crypto, storage)
}

fn create_test_request(crypto: &CoreCryptoServiceImpl) -> SecretRecoveryRequest {
    let (requester_sk, _requester_pk) = crypto.generate_keypair().unwrap();
    let (_, owner_pk) = crypto.generate_keypair().unwrap();

    SecretRecoveryRequest {
        secret_id: SecretId::generate(),
        requester_secret_key: requester_sk,
        owner_public_key: owner_pk,
        requester_process_id: "requester-process-123".to_string(),
    }
}

#[tokio::test]
async fn test_execute_recovery_fails_at_capsule_retrieval() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let request = create_test_request(&crypto);

    let result = service.execute_secret_recovery(request).await;
    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
}

#[tokio::test]
async fn test_can_recover_fails_at_capsule_retrieval() {
    let service = create_test_service();
    let secret_id = SecretId::generate();

    let result = service
        .can_recover(&secret_id, "requester-process-123")
        .await;
    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
}

#[test]
fn test_phase3_decryption_error_message() {
    let err = WorkflowError::DecryptionError {
        phase: "PRE decapsulation".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("Decryption failed"));
    assert!(msg.contains("PRE decapsulation"));
}

#[tokio::test]
async fn test_workflow_fails_at_capsule_retrieval() {
    let service = create_test_service();
    let crypto = CoreCryptoServiceImpl::new();
    let request = create_test_request(&crypto);

    let result = service.execute_secret_recovery(request).await;
    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
}

#[tokio::test]
async fn test_can_recover_fails_when_no_capsule() {
    let service = create_test_service();
    let secret_id = SecretId::generate();

    let result = service.can_recover(&secret_id, "requester-123").await;
    assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
}
