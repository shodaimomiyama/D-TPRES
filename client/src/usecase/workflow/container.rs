//! WorkflowService Container - Dependency Injection Configuration
//!
//! Provides factory functions and a container for WorkflowServices with
//! properly configured dependencies.

use std::sync::Arc;

use crate::adapter::external::mock_ao::MockAOClient;
use crate::usecase::core::contract_storage::ContractStorageImpl;
use crate::usecase::core::crypto::CryptoServiceImpl as CoreCryptoServiceImpl;
use crate::usecase::core::storage::ArweaveStorageServiceImpl;
use crate::usecase::service::{
    CryptoService, CryptoServiceImpl, StorageService, StorageServiceImpl,
};

use super::secret_recovery_service::{
    SecretRecoveryWorkflowService, SecretRecoveryWorkflowServiceImpl,
};
use super::secret_sharing_service::{
    SecretSharingWorkflowService, SecretSharingWorkflowServiceImpl,
};

// ============================================================================
// WorkflowServiceContainer
// ============================================================================

/// Container for WorkflowServices
///
/// Provides pre-configured instances of SecretSharingWorkflowService and
/// SecretRecoveryWorkflowService with all dependencies properly wired.
pub struct WorkflowServiceContainer<C: CryptoService, ST: StorageService> {
    crypto_service: Arc<C>,
    storage_service: Arc<ST>,
}

impl<C: CryptoService, ST: StorageService> WorkflowServiceContainer<C, ST> {
    /// Create a new WorkflowServiceContainer
    pub fn new(crypto_service: Arc<C>, storage_service: Arc<ST>) -> Self {
        Self {
            crypto_service,
            storage_service,
        }
    }

    /// Get a reference to the CryptoService
    pub fn crypto_service(&self) -> Arc<C> {
        Arc::clone(&self.crypto_service)
    }

    /// Create a SecretSharingWorkflowService instance
    pub fn secret_sharing_service(&self) -> SecretSharingWorkflowServiceImpl<C, ST> {
        SecretSharingWorkflowServiceImpl::new(
            Arc::clone(&self.crypto_service),
            Arc::clone(&self.storage_service),
        )
    }

    /// Create a SecretRecoveryWorkflowService instance
    pub fn secret_recovery_service(&self) -> SecretRecoveryWorkflowServiceImpl<C, ST> {
        SecretRecoveryWorkflowServiceImpl::new(
            Arc::clone(&self.crypto_service),
            Arc::clone(&self.storage_service),
        )
    }
}

// ============================================================================
// Default Container with CryptoServiceImpl
// ============================================================================

type DefaultCryptoService = CryptoServiceImpl<CoreCryptoServiceImpl>;
type DefaultStorageService =
    StorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

/// Default WorkflowServiceContainer using concrete implementations
pub type DefaultWorkflowServiceContainer =
    WorkflowServiceContainer<DefaultCryptoService, DefaultStorageService>;

impl DefaultWorkflowServiceContainer {
    /// Create a new container with default implementations
    pub fn with_default_crypto() -> Self {
        let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
        let crypto_service = Arc::new(CryptoServiceImpl::new(core_crypto));

        let mock_ao = Arc::new(MockAOClient::new());
        let arweave = Arc::new(ArweaveStorageServiceImpl::default());
        let contract = Arc::new(ContractStorageImpl::new(mock_ao));
        let storage_service = Arc::new(StorageServiceImpl::new(arweave, contract));

        Self::new(crypto_service, storage_service)
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create a SecretSharingWorkflowService with default dependencies
pub fn create_secret_sharing_service() -> impl SecretSharingWorkflowService {
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto_service = Arc::new(CryptoServiceImpl::new(core_crypto));

    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage_service = Arc::new(StorageServiceImpl::new(arweave, contract));

    SecretSharingWorkflowServiceImpl::new(crypto_service, storage_service)
}

/// Create a SecretRecoveryWorkflowService with default dependencies
pub fn create_secret_recovery_service() -> impl SecretRecoveryWorkflowService {
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto_service = Arc::new(CryptoServiceImpl::new(core_crypto));

    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage_service = Arc::new(StorageServiceImpl::new(arweave, contract));

    SecretRecoveryWorkflowServiceImpl::new(crypto_service, storage_service)
}

/// Create both WorkflowServices sharing the same CryptoService
pub fn create_workflow_services() -> (
    impl SecretSharingWorkflowService,
    impl SecretRecoveryWorkflowService,
) {
    let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
    let crypto_service = Arc::new(CryptoServiceImpl::new(core_crypto));

    let mock_ao = Arc::new(MockAOClient::new());
    let arweave = Arc::new(ArweaveStorageServiceImpl::default());
    let contract = Arc::new(ContractStorageImpl::new(mock_ao));
    let storage_service = Arc::new(StorageServiceImpl::new(arweave, contract));

    let sharing = SecretSharingWorkflowServiceImpl::new(
        Arc::clone(&crypto_service),
        Arc::clone(&storage_service),
    );
    let recovery = SecretRecoveryWorkflowServiceImpl::new(crypto_service, storage_service);
    (sharing, recovery)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
