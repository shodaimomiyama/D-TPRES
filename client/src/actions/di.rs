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

#[cfg(feature = "production-ao")]
use crate::adapter::external::ao::ProductionAOClient;
#[cfg(feature = "production-ao")]
use crate::adapter::external::arweave::{ArweaveClientImpl, ProductionArweaveStorageService};

#[cfg(feature = "production-ao")]
pub type ProductionStorageService = ServiceStorageServiceImpl<
    ProductionArweaveStorageService<ArweaveClientImpl>,
    ContractStorageImpl<ProductionAOClient>,
>;

#[cfg(feature = "production-ao")]
pub type ProductionActionsContainer =
    ActionsContainer<CoreCryptoServiceImpl, ProductionStorageService>;

#[cfg(feature = "production-ao")]
impl ProductionActionsContainer {
    pub fn with_production_ao(
        ao_client: Arc<ProductionAOClient>,
        arweave_client: Arc<ArweaveClientImpl>,
    ) -> Self {
        let crypto_service = Arc::new(CoreCryptoServiceImpl::new());
        let service_crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&crypto_service)));
        let arweave = Arc::new(ProductionArweaveStorageService::new(arweave_client));
        let contract = Arc::new(ContractStorageImpl::new(ao_client));
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
