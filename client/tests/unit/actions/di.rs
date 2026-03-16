use std::sync::Arc;

use formix::actions::{ActionsContainer, DefaultActionsContainer};
use formix::adapter::external::mock_ao::MockAOClient;
use formix::controller::ControllerContainer;
use formix::usecase::core::contract_storage::ContractStorageImpl;
use formix::usecase::core::crypto::{
    CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
};
use formix::usecase::core::storage::ArweaveStorageServiceImpl;
use formix::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageServiceImpl as ServiceStorageServiceImpl,
};
use formix::usecase::workflow::WorkflowServiceContainer;

#[test]
fn test_container_new() {
    let container = DefaultActionsContainer::new();
    let _ = container.controller();
    let _ = container.workflow_services();
    let _ = container.crypto_service();
}

#[test]
fn test_container_default() {
    let container: DefaultActionsContainer = Default::default();
    let _ = container.controller();
    let _ = container.workflow_services();
    let _ = container.crypto_service();
}

#[test]
fn test_container_provides_controller() {
    let container = DefaultActionsContainer::new();
    let controller = container.controller();

    let (owner_sk, owner_pk) = container.crypto_service().generate_keypair().unwrap();
    let (_, requester_pk) = container.crypto_service().generate_keypair().unwrap();

    let result =
        controller
            .share_validator()
            .validate(b"secret", 3, 5, &owner_sk, &owner_pk, &requester_pk);
    assert!(result.is_ok());
}

#[test]
fn test_container_provides_workflow_services() {
    let container = DefaultActionsContainer::new();
    let workflow_services = container.workflow_services();

    let _ = workflow_services.secret_sharing_service();
    let _ = workflow_services.secret_recovery_service();
}

#[test]
fn test_container_provides_crypto_service() {
    let container = DefaultActionsContainer::new();

    let result = container.crypto_service().generate_keypair();
    assert!(result.is_ok());
}

#[test]
fn test_container_crypto_service_arc() {
    let container = DefaultActionsContainer::new();
    let arc1 = container.crypto_service_arc();
    let arc2 = container.crypto_service_arc();

    assert!(Arc::ptr_eq(&arc1, &arc2));
}

#[test]
fn test_container_with_dependencies() {
    let crypto_service = Arc::new(CoreCryptoServiceImpl::new());
    let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&crypto_service)));

    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new_single_process(mock_ao));
    let storage_service = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

    let controller = ControllerContainer::new(Arc::clone(&crypto_service));
    let workflow_services = WorkflowServiceContainer::new(service_crypto, storage_service);

    let container =
        ActionsContainer::with_dependencies(controller, workflow_services, crypto_service);

    let _ = container.controller();
    let _ = container.workflow_services();
    let result = container.crypto_service().generate_keypair();
    assert!(result.is_ok());
}

#[test]
fn test_container_initializes_all_dependencies() {
    let container = DefaultActionsContainer::new();

    let (owner_sk, owner_pk) = container.crypto_service().generate_keypair().unwrap();
    let (requester_sk, requester_pk) = container.crypto_service().generate_keypair().unwrap();

    let validate_result = container.controller().share_validator().validate(
        b"secret",
        3,
        5,
        &owner_sk,
        &owner_pk,
        &requester_pk,
    );
    assert!(validate_result.is_ok());

    let secret_id = formix::domain::SecretId::new("test");
    let (_, owner_pk) = container.crypto_service().generate_keypair().unwrap();
    let request = container.controller().recover_extractor().extract(
        secret_id,
        requester_sk,
        owner_pk,
        "process_id".to_string(),
    );
    assert_eq!(request.requester_process_id, "process_id");

    let _sharing = container.workflow_services().secret_sharing_service();
    let _recovery = container.workflow_services().secret_recovery_service();
}
