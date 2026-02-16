//! Actions layer dependency injection container
//!
//! Provides centralized access to all components needed by Actions layer functions.

use std::sync::Arc;

use crate::adapter::external::mock_ao::MockAOClient;
use crate::controller::di::ControllerContainer;
use crate::usecase::core::contract_storage::ContractStorageImpl;
use crate::usecase::core::crypto::{
    CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
};
use crate::usecase::core::storage::ArweaveStorageServiceImpl;
use crate::usecase::service::{
    CryptoServiceImpl as ServiceCryptoServiceImpl, StorageService,
    StorageServiceImpl as ServiceStorageServiceImpl,
};
use crate::usecase::workflow::container::WorkflowServiceContainer;

/// Container for Actions layer dependencies
///
/// Aggregates Controller layer, WorkflowService layer, and CryptoService
/// for simplified access from Actions functions (share, recover, generateKeyPair).
///
/// `C` is bounded by `core::CryptoService` for controller compatibility.
/// The workflow layer receives `ServiceCryptoServiceImpl<C>` wrapping `C`.
pub struct ActionsContainer<C: CoreCryptoService, ST: StorageService> {
    controller: ControllerContainer<C>,
    workflow_services: WorkflowServiceContainer<ServiceCryptoServiceImpl<C>, ST>,
    crypto_service: Arc<C>,
}

impl<C: CoreCryptoService, ST: StorageService> ActionsContainer<C, ST> {
    /// Create a new ActionsContainer with custom dependencies
    pub fn with_dependencies(
        controller: ControllerContainer<C>,
        workflow_services: WorkflowServiceContainer<ServiceCryptoServiceImpl<C>, ST>,
        crypto_service: Arc<C>,
    ) -> Self {
        Self {
            controller,
            workflow_services,
            crypto_service,
        }
    }

    /// Get reference to ControllerContainer
    pub const fn controller(&self) -> &ControllerContainer<C> {
        &self.controller
    }

    /// Get reference to WorkflowServiceContainer
    pub const fn workflow_services(
        &self,
    ) -> &WorkflowServiceContainer<ServiceCryptoServiceImpl<C>, ST> {
        &self.workflow_services
    }

    /// Get reference to CryptoService
    #[allow(clippy::missing_const_for_fn)]
    pub fn crypto_service(&self) -> &C {
        &self.crypto_service
    }

    /// Get Arc clone of CryptoService
    pub fn crypto_service_arc(&self) -> Arc<C> {
        Arc::clone(&self.crypto_service)
    }
}

pub type DefaultStorageService =
    ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

/// Default ActionsContainer using concrete implementations
pub type DefaultActionsContainer = ActionsContainer<CoreCryptoServiceImpl, DefaultStorageService>;

impl DefaultActionsContainer {
    /// Create a new ActionsContainer with default components
    pub fn new() -> Self {
        let crypto_service = Arc::new(CoreCryptoServiceImpl::new());
        let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&crypto_service)));

        let mock_ao = Arc::new(MockAOClient::new());
        let arweave = Arc::new(ArweaveStorageServiceImpl::default());
        let contract = Arc::new(ContractStorageImpl::new(mock_ao));
        let storage_service = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

        let controller = ControllerContainer::new(Arc::clone(&crypto_service));
        let workflow_services = WorkflowServiceContainer::new(service_crypto, storage_service);

        Self {
            controller,
            workflow_services,
            crypto_service,
        }
    }

    /// Create with pre-configured arweave and contract storage services
    pub fn with_storage(
        arweave: Arc<ArweaveStorageServiceImpl>,
        contract: Arc<ContractStorageImpl<MockAOClient>>,
    ) -> Self {
        let crypto_service = Arc::new(CoreCryptoServiceImpl::new());
        let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&crypto_service)));
        let storage_service = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

        let controller = ControllerContainer::new(Arc::clone(&crypto_service));
        let workflow_services = WorkflowServiceContainer::new(service_crypto, storage_service);

        Self {
            controller,
            workflow_services,
            crypto_service,
        }
    }
}

impl Default for DefaultActionsContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::CryptoService as CoreCryptoService;

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

        let result = controller.share_validator().validate(
            b"secret",
            3,
            5,
            &owner_sk,
            &owner_pk,
            &requester_pk,
        );
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
        let contract = Arc::new(ContractStorageImpl::new(mock_ao));
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

        let secret_id = crate::domain::value_objects::SecretId::new("test");
        let request = container.controller().recover_extractor().extract(
            secret_id,
            requester_sk,
            "process_id".to_string(),
        );
        assert_eq!(request.requester_process_id, "process_id");

        let _sharing = container.workflow_services().secret_sharing_service();
        let _recovery = container.workflow_services().secret_recovery_service();
    }
}
