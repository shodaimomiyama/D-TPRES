//! WorkflowService Container - Dependency Injection Configuration
//!
//! Provides factory functions and a container for WorkflowServices with
//! properly configured dependencies.

use std::sync::Arc;

use crate::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

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
pub struct WorkflowServiceContainer<C: CryptoService> {
    crypto_service: Arc<C>,
}

impl<C: CryptoService> WorkflowServiceContainer<C> {
    /// Create a new WorkflowServiceContainer with the given CryptoService
    pub const fn new(crypto_service: Arc<C>) -> Self {
        Self { crypto_service }
    }

    /// Get a reference to the CryptoService
    pub fn crypto_service(&self) -> Arc<C> {
        Arc::clone(&self.crypto_service)
    }

    /// Create a SecretSharingWorkflowService instance
    pub fn secret_sharing_service(&self) -> SecretSharingWorkflowServiceImpl<C> {
        SecretSharingWorkflowServiceImpl::new(Arc::clone(&self.crypto_service))
    }

    /// Create a SecretRecoveryWorkflowService instance
    pub fn secret_recovery_service(&self) -> SecretRecoveryWorkflowServiceImpl<C> {
        SecretRecoveryWorkflowServiceImpl::new(Arc::clone(&self.crypto_service))
    }
}

// ============================================================================
// Default Container with CryptoServiceImpl
// ============================================================================

/// Default WorkflowServiceContainer using CryptoServiceImpl
pub type DefaultWorkflowServiceContainer = WorkflowServiceContainer<CryptoServiceImpl>;

impl DefaultWorkflowServiceContainer {
    /// Create a new container with default CryptoService implementation
    pub fn with_default_crypto() -> Self {
        let crypto_service = Arc::new(CryptoServiceImpl::new());
        Self::new(crypto_service)
    }
}

// ============================================================================
// Factory Functions
// ============================================================================

/// Create a SecretSharingWorkflowService with default dependencies
pub fn create_secret_sharing_service() -> impl SecretSharingWorkflowService {
    let crypto_service = Arc::new(CryptoServiceImpl::new());
    SecretSharingWorkflowServiceImpl::new(crypto_service)
}

/// Create a SecretRecoveryWorkflowService with default dependencies
pub fn create_secret_recovery_service() -> impl SecretRecoveryWorkflowService {
    let crypto_service = Arc::new(CryptoServiceImpl::new());
    SecretRecoveryWorkflowServiceImpl::new(crypto_service)
}

/// Create both WorkflowServices sharing the same CryptoService
pub fn create_workflow_services() -> (
    impl SecretSharingWorkflowService,
    impl SecretRecoveryWorkflowService,
) {
    let crypto_service = Arc::new(CryptoServiceImpl::new());
    let sharing = SecretSharingWorkflowServiceImpl::new(Arc::clone(&crypto_service));
    let recovery = SecretRecoveryWorkflowServiceImpl::new(crypto_service);
    (sharing, recovery)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_creates_services() {
        let container = DefaultWorkflowServiceContainer::with_default_crypto();

        // Verify services can be created
        let _sharing = container.secret_sharing_service();
        let _recovery = container.secret_recovery_service();
    }

    #[test]
    fn test_factory_functions() {
        // Verify factory functions work
        let _sharing = create_secret_sharing_service();
        let _recovery = create_secret_recovery_service();
        let (_sharing2, _recovery2) = create_workflow_services();
    }

    #[test]
    fn test_container_shares_crypto_service() {
        let container = DefaultWorkflowServiceContainer::with_default_crypto();

        // Get crypto service reference
        let crypto1 = container.crypto_service();
        let crypto2 = container.crypto_service();

        // Verify they point to the same instance (Arc::ptr_eq)
        assert!(Arc::ptr_eq(&crypto1, &crypto2));
    }
}
