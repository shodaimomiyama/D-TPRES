use std::sync::Arc;

use formix::usecase::workflow::{
    create_secret_recovery_service, create_secret_sharing_service, create_workflow_services,
    DefaultWorkflowServiceContainer,
};

#[test]
fn test_container_creates_services() {
    let container = DefaultWorkflowServiceContainer::with_default_crypto();

    let _sharing = container.secret_sharing_service();
    let _recovery = container.secret_recovery_service();
}

#[test]
fn test_factory_functions() {
    let _sharing = create_secret_sharing_service();
    let _recovery = create_secret_recovery_service();
    let (_sharing2, _recovery2) = create_workflow_services();
}

#[test]
fn test_container_shares_crypto_service() {
    let container = DefaultWorkflowServiceContainer::with_default_crypto();

    let crypto1 = container.crypto_service();
    let crypto2 = container.crypto_service();

    assert!(Arc::ptr_eq(&crypto1, &crypto2));
}
