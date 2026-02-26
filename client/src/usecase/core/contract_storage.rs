//! ContractStorage - AO contract communication service
//!
//! Handles AO Network contract interactions, separated from
//! Arweave immutable storage operations for clean SRP compliance.

use std::sync::Arc;

use async_trait::async_trait;

use crate::adapter::external::ao::{
    AOClient, Binary, ExecuteMsg, GetCFragsBySecretResponse, QueryMsg,
};
use crate::service::error::{ServiceError, ServiceResult};
use crate::usecase::core::crypto::{CFragData, KeyFragment};

/// Contract storage trait for AO Network communication
#[async_trait]
pub trait ContractStorage: Send + Sync {
    /// Send kFrags to Owner-Process via AO Network
    async fn send_kfrags(&self, kfrags: &[KeyFragment], contract_id: &str) -> ServiceResult<()>;

    /// Delegate capsule to AO contract for re-encryption triggering
    ///
    /// Associates the Arweave capsule with its kFrag so the AO contract
    /// can trigger Phase 2 re-encryption.
    ///
    /// # Arguments
    /// * `capsule_data` - Serialized capsule bytes (from Arweave CapsulePayload)
    /// * `contract_id` - Owner-Process contract ID on AO Network
    /// * `kfrag_id`    - kFrag identifier matching the corresponding DelegateKFrag call
    ///                   (format: `"kfrag-{index}"`)
    /// * `capsule_id`  - Arweave transaction ID of the stored capsule (`capsule_tx_id`)
    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()>;

    /// Retrieve cFrags for a secret from AO Network
    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        requester_process_id: &str,
    ) -> ServiceResult<Vec<CFragData>>;
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
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()> {
        let msg = ExecuteMsg::DelegateCapsule {
            kfrag_id: kfrag_id.to_string(),
            capsule_id: capsule_id.to_string(),
            capsule: Binary::from(capsule_data.to_vec()),
        };
        self.ao_client
            .execute(contract_id, msg)
            .await
            .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;
        Ok(())
    }

    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        requester_process_id: &str,
    ) -> ServiceResult<Vec<CFragData>> {
        let msg = QueryMsg::GetCFragsBySecret {
            secret_id: secret_id.to_string(),
        };
        let result = self
            .ao_client
            .query(requester_process_id, msg)
            .await
            .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;

        let response: GetCFragsBySecretResponse = serde_json::from_slice(result.as_slice())
            .map_err(|e| {
                ServiceError::ao_network_error(format!("Failed to parse cFrag response: {e}"))
            })?;

        let cfrags = response
            .cfrags
            .into_iter()
            .map(|entry| CFragData {
                cfrag_data: entry.cfrag_data.into_vec(),
                holder_id: entry.holder_id,
            })
            .collect();

        Ok(cfrags)
    }
}
