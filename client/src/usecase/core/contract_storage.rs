//! ContractStorage - AO contract communication service
//!
//! Handles AO Network contract interactions, separated from
//! Arweave immutable storage operations for clean SRP compliance.

use std::sync::Arc;

use async_trait::async_trait;

use crate::adapter::external::ao::{AOClient, Binary, ExecuteMsg};
use crate::service::error::{ServiceError, ServiceResult};
use crate::usecase::core::crypto::KeyFragment;

/// Contract storage trait for AO Network communication
#[async_trait]
pub trait ContractStorage: Send + Sync {
    /// Send kFrags to Owner-Process via AO Network
    async fn send_kfrags(&self, kfrags: &[KeyFragment], contract_id: &str) -> ServiceResult<()>;

    /// Delegate capsule to contract (future use)
    async fn delegate_capsule(&self, capsule_data: &[u8], contract_id: &str) -> ServiceResult<()>;

    /// Retrieve cFrags from contract (future use)
    async fn retrieve_cfrags(
        &self,
        capsule_id: &str,
        contract_id: &str,
    ) -> ServiceResult<Vec<Vec<u8>>>;

    /// Retrieve threshold from contract (future use)
    async fn retrieve_threshold(&self, contract_id: &str) -> ServiceResult<u8>;
}

/// ContractStorage implementation using AOClient
pub struct ContractStorageImpl<A: AOClient> {
    ao_client: Arc<A>,
}

impl<A: AOClient> ContractStorageImpl<A> {
    pub fn new(ao_client: Arc<A>) -> Self {
        Self { ao_client }
    }
}

#[async_trait]
impl<A: AOClient> ContractStorage for ContractStorageImpl<A> {
    async fn send_kfrags(&self, kfrags: &[KeyFragment], contract_id: &str) -> ServiceResult<()> {
        for kfrag in kfrags {
            let msg = ExecuteMsg::DelegateKFrag {
                kfrag_id: format!("kfrag-{}", kfrag.id),
                kfrag: Binary::from(kfrag.key_data.clone()),
            };
            self.ao_client
                .execute(contract_id, msg)
                .await
                .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;
        }

        Ok(())
    }

    async fn delegate_capsule(
        &self,
        _capsule_data: &[u8],
        _contract_id: &str,
    ) -> ServiceResult<()> {
        Err(ServiceError::System(
            crate::service::error::SystemException::Internal(
                "Not implemented: delegate_capsule".to_string(),
            ),
        ))
    }

    async fn retrieve_cfrags(
        &self,
        _capsule_id: &str,
        _contract_id: &str,
    ) -> ServiceResult<Vec<Vec<u8>>> {
        Err(ServiceError::System(
            crate::service::error::SystemException::Internal(
                "Not implemented: retrieve_cfrags".to_string(),
            ),
        ))
    }

    async fn retrieve_threshold(&self, _contract_id: &str) -> ServiceResult<u8> {
        Err(ServiceError::System(
            crate::service::error::SystemException::Internal(
                "Not implemented: retrieve_threshold".to_string(),
            ),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::external::mock_ao::MockAOClient;

    fn create_test_contract_storage() -> ContractStorageImpl<MockAOClient> {
        let mock_ao = Arc::new(MockAOClient::new());
        ContractStorageImpl::new(mock_ao)
    }

    #[tokio::test]
    async fn test_send_kfrags() {
        let storage = create_test_contract_storage();

        let kfrags = vec![
            KeyFragment {
                id: 0,
                key_data: vec![1, 2, 3],
                verification_data: vec![],
                precursor: vec![],
            },
            KeyFragment {
                id: 1,
                key_data: vec![4, 5, 6],
                verification_data: vec![],
                precursor: vec![],
            },
        ];

        let result = storage.send_kfrags(&kfrags, "test-process-id").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delegate_capsule_not_implemented() {
        let storage = create_test_contract_storage();
        let result = storage.delegate_capsule(&[1, 2, 3], "test-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retrieve_cfrags_not_implemented() {
        let storage = create_test_contract_storage();
        let result = storage.retrieve_cfrags("capsule-id", "test-id").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retrieve_threshold_not_implemented() {
        let storage = create_test_contract_storage();
        let result = storage.retrieve_threshold("test-id").await;
        assert!(result.is_err());
    }
}
