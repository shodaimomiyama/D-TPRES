//! ContractStorage - AO contract communication service (HyperBEAM-native)
//!
//! Updated for new `ao/` module:
//! - `ExecuteMsg::*` enum variants → `AOExecuteMsg::*` builder methods
//! - `QueryMsg::*` → `AOQueryMsg::*` builder methods
//! - `AOResponse` → `AONativeResponse`
//! - `GetCFragResponse` parses `{ kfrag_id, capsule_id, cfrag: [u8] }` format

use std::sync::Arc;

use async_trait::async_trait;

use crate::adapter::external::ao::{AOClient, AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary, GetCFragResponse};
use crate::service::error::{ServiceError, ServiceResult};
use crate::usecase::core::crypto::{CFragData, KeyFragment};

/// Contract storage trait for AO Network communication.
#[async_trait]
pub trait ContractStorage: Send + Sync {
    /// Send kFrags to Owner-Process via AO Network.
    async fn send_kfrags(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()>;

    /// Delegate capsule to AO contract for re-encryption triggering.
    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()>;

    /// Retrieve cFrags for a secret from AO Network.
    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>>;
}

/// ContractStorage implementation using AOClient (HyperBEAM-native).
pub struct ContractStorageImpl<A: AOClient> {
    ao_client: Arc<A>,
    /// Default holder process ID for DelegateCapsule.
    /// In HyperBEAM, Owner-Process delegates to a Holder-Process via AO message.
    /// Set to the same process_id for single-process setups.
    holder_process_id: String,
}

impl<A: AOClient> ContractStorageImpl<A> {
    pub fn new(ao_client: Arc<A>, holder_process_id: impl Into<String>) -> Self {
        Self { ao_client, holder_process_id: holder_process_id.into() }
    }

    /// Convenience constructor for single-process setups (holder = owner).
    pub fn new_single_process(ao_client: Arc<A>) -> Self {
        Self { ao_client, holder_process_id: String::new() }
    }
}

#[async_trait]
impl<A: AOClient> ContractStorage for ContractStorageImpl<A> {
    async fn send_kfrags(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()> {
        for kfrag in kfrags {
            let kfrag_id = format!("{secret_id}_{}", kfrag.id);
            let msg = AOExecuteMsg::delegate_kfrag(&kfrag_id, kfrag.key_data.clone());
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
        // holder_process_id defaults to contract_id for single-process setups
        let holder = if self.holder_process_id.is_empty() {
            contract_id.to_string()
        } else {
            self.holder_process_id.clone()
        };

        let msg = AOExecuteMsg::delegate_capsule(kfrag_id, capsule_id, capsule_data.to_vec(), holder);
        self.ao_client
            .execute(contract_id, msg)
            .await
            .map_err(|e| ServiceError::ao_network_error(e.to_string()))?;
        Ok(())
    }

    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>> {
        let mut cfrags = Vec::new();

        for i in 0..total_shares {
            let kfrag_id = format!("{secret_id}_{i}");
            let msg = AOQueryMsg::get_cfrag(&kfrag_id, capsule_id);

            match self.ao_client.query(process_id, msg).await {
                Ok(result) => {
                    let response: GetCFragResponse = serde_json::from_slice(result.as_slice())
                        .map_err(|e| ServiceError::ao_network_error(
                            format!("Failed to parse cFrag for {kfrag_id}: {e}")
                        ))?;
                    cfrags.push(CFragData {
                        cfrag_data: response.cfrag,
                        holder_id: kfrag_id,
                    });
                }
                Err(_) => {
                    // cFrag not yet ready for this kfrag — skip
                }
            }
        }

        Ok(cfrags)
    }
}
