//! SecretRecoveryWorkflowService - Phase 3 Secret Recovery Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 3 workflow: cFrag/Capsule retrieval, PRE decryption, and secret reconstruction.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{
    CFragData, Capsule, CapsulePayload, PublicKey, SecretKey, ShamirShare,
};
use crate::usecase::core::storage::{QueryParams, Tag};
use crate::usecase::dto::{SecretRecoveryRequest, SecretRecoveryResult};
use crate::usecase::error::{WorkflowError, WorkflowResult};
use crate::usecase::service::{CryptoService, StorageService};

// ============================================================================
// SecretRecoveryWorkflowService Trait
// ============================================================================

/// SecretRecoveryWorkflowService trait - Phase 3 workflow interface
///
/// Defines the contract for secret recovery operations that orchestrate
/// CryptoService and StorageService to retrieve, decrypt, and reconstruct secrets.
#[async_trait]
pub trait SecretRecoveryWorkflowService: Send + Sync {
    /// Execute the complete Phase 3 secret recovery workflow
    ///
    /// # Steps
    /// 1. Retrieve cFrags from Requester-Process via AO Network
    /// 2. Retrieve Capsule and encrypted shares from Arweave
    /// 3. Verify threshold is met (have enough cFrags)
    /// 4. Decrypt Capsule using cFrags (PRE decapsulation)
    /// 5. Recover symmetric key kₒ
    /// 6. Decrypt each share with AES-GCM using kₒ
    /// 7. Reconstruct secret using Shamir Secret Sharing
    /// 8. Record audit trail on Arweave
    async fn execute_secret_recovery(
        &self,
        request: SecretRecoveryRequest,
    ) -> WorkflowResult<SecretRecoveryResult>;

    /// Check if a secret can be recovered (enough cFrags available)
    async fn can_recover(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<bool>;
}

// ============================================================================
// SecretRecoveryWorkflowServiceImpl
// ============================================================================

/// Implementation of SecretRecoveryWorkflowService
///
/// Orchestrates CryptoService and StorageService to implement
/// the complete Phase 3 secret recovery workflow.
pub struct SecretRecoveryWorkflowServiceImpl<C: CryptoService, ST: StorageService> {
    crypto_service: Arc<C>,
    storage_service: Arc<ST>,
}

#[allow(clippy::cast_possible_truncation, clippy::indexing_slicing)]
impl<C: CryptoService, ST: StorageService> SecretRecoveryWorkflowServiceImpl<C, ST> {
    /// Create a new SecretRecoveryWorkflowServiceImpl
    pub fn new(crypto_service: Arc<C>, storage_service: Arc<ST>) -> Self {
        Self {
            crypto_service,
            storage_service,
        }
    }

    /// Validate input parameters for secret recovery
    fn validate_request(&self, request: &SecretRecoveryRequest) -> WorkflowResult<()> {
        if request.requester_secret_key.is_empty() {
            return Err(WorkflowError::validation(
                "Requester secret key is required",
            ));
        }

        if request.requester_process_id.is_empty() {
            return Err(WorkflowError::validation(
                "Requester process ID is required",
            ));
        }

        if request.owner_public_key.key_data.is_empty() {
            return Err(WorkflowError::validation("Owner public key is required"));
        }

        Ok(())
    }

    /// Retrieve cFrags from AO Network via StorageService
    async fn retrieve_cfrags(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<Vec<CFragData>> {
        self.storage_service
            .retrieve_cfrags(secret_id.as_str(), requester_process_id)
            .await
            .map_err(WorkflowError::from)
    }

    /// Retrieve Capsule, ciphertext, verifying_pk, and threshold from Arweave
    fn retrieve_capsule_with_metadata(
        &self,
        secret_id: &SecretId,
    ) -> WorkflowResult<(Capsule, Vec<u8>, Vec<u8>, u8)> {
        // Phase 1 stores exactly one capsule per secret_id (UUID v4), so limit=1
        // with no sort is safe. See CapsuleRepository.find_by_secret_id -> Option<Capsule>.
        let params = QueryParams {
            tags: vec![
                Tag {
                    name: "type".to_string(),
                    value: "capsule".to_string(),
                },
                Tag {
                    name: "secret_id".to_string(),
                    value: secret_id.as_str().to_string(),
                },
            ],
            limit: Some(1),
            sort_by: None,
        };

        let transactions = self
            .storage_service
            .query_by_tags(params)
            .map_err(WorkflowError::from)?;

        let tx = transactions.into_iter().next().ok_or_else(|| {
            WorkflowError::not_found(format!("Capsule not found for secret {secret_id}"))
        })?;

        let threshold_k = tx
            .tags
            .iter()
            .find(|t| t.name == "threshold_k")
            .ok_or_else(|| {
                WorkflowError::not_found("threshold_k tag missing from capsule transaction")
            })?
            .value
            .parse::<u8>()
            .map_err(|e| WorkflowError::validation(format!("Invalid threshold_k value: {e}")))?;

        let payload: CapsulePayload = bincode::deserialize(&tx.data)
            .map_err(|_| WorkflowError::crypto("Failed to deserialize CapsulePayload"))?;

        let capsule = Capsule {
            capsule_bytes: payload.capsule_bytes,
        };

        Ok((
            capsule,
            payload.ciphertext,
            payload.verifying_pk,
            threshold_k,
        ))
    }

    /// Retrieve encrypted shares from Arweave via StorageService
    fn retrieve_encrypted_shares(&self, secret_id: &SecretId) -> WorkflowResult<Vec<Vec<u8>>> {
        self.storage_service
            .retrieve_encrypted_shares(secret_id.as_str())
            .map_err(WorkflowError::from)
    }

    /// Verify threshold requirement is met
    fn verify_threshold(&self, cfrags: &[CFragData], required_threshold: u8) -> WorkflowResult<()> {
        let available = cfrags.len() as u8;
        if available < required_threshold {
            return Err(WorkflowError::insufficient_cfrags(
                required_threshold,
                available,
            ));
        }
        Ok(())
    }

    /// Decrypt Capsule using cFrags and recover symmetric key
    fn decrypt_capsule(
        &self,
        capsule: &Capsule,
        cfrags: &[CFragData],
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        ciphertext: &[u8],
        verifying_pk: &[u8],
    ) -> WorkflowResult<Vec<u8>> {
        self.crypto_service
            .decrypt_pre_capsule(
                capsule,
                cfrags,
                requester_secret_key,
                owner_public_key,
                ciphertext,
                verifying_pk,
            )
            .map_err(|e| WorkflowError::DecryptionError {
                phase: format!("PRE decapsulation: {e}"),
            })
    }

    /// Decrypt shares with AES-GCM
    fn decrypt_shares(
        &self,
        encrypted_shares: &[Vec<u8>],
        symmetric_key: &[u8],
    ) -> WorkflowResult<Vec<ShamirShare>> {
        encrypted_shares
            .iter()
            .enumerate()
            .map(|(i, encrypted_share)| {
                let decrypted_data = self
                    .crypto_service
                    .aes_gcm_decrypt(symmetric_key, encrypted_share)
                    .map_err(|e| WorkflowError::DecryptionError {
                        phase: format!("AES-GCM decryption of share {i}: {e}"),
                    })?;

                // shamirsecretsharing library embeds the index in the first byte of share data
                if decrypted_data.is_empty() {
                    return Err(WorkflowError::DecryptionError {
                        phase: "Share data is empty after decryption".to_string(),
                    });
                }

                let index = decrypted_data[0];

                Ok(ShamirShare {
                    index,
                    share_data: decrypted_data,
                })
            })
            .collect()
    }

    /// Reconstruct secret using Shamir Secret Sharing
    fn reconstruct_secret(&self, shares: &[ShamirShare], threshold: u8) -> WorkflowResult<Vec<u8>> {
        self.crypto_service
            .reconstruct_secret_shamir(shares, threshold)
            .map_err(|e| WorkflowError::DecryptionError {
                phase: format!("Shamir reconstruction: {e}"),
            })
    }

    /// Record audit trail on Arweave (placeholder)
    fn record_audit_trail(
        &self,
        _secret_id: &SecretId,
        _requester_process_id: &str,
    ) -> WorkflowResult<String> {
        Ok("audit_not_implemented".to_string())
    }
}

#[allow(clippy::cast_possible_truncation)]
#[async_trait]
impl<C: CryptoService, ST: StorageService> SecretRecoveryWorkflowService
    for SecretRecoveryWorkflowServiceImpl<C, ST>
{
    async fn execute_secret_recovery(
        &self,
        request: SecretRecoveryRequest,
    ) -> WorkflowResult<SecretRecoveryResult> {
        // Step 0: Validate input parameters
        self.validate_request(&request)?;

        // Step 1: Retrieve cFrags from Requester-Process via AO
        let cfrags = self
            .retrieve_cfrags(&request.secret_id, &request.requester_process_id)
            .await?;

        // Step 2: Retrieve CapsulePayload (capsule + ciphertext + verifying_pk) from Arweave
        let (capsule, ciphertext, verifying_pk, threshold) =
            self.retrieve_capsule_with_metadata(&request.secret_id)?;

        // Step 3: Retrieve encrypted shares and verify threshold
        let encrypted_shares = self.retrieve_encrypted_shares(&request.secret_id)?;
        self.verify_threshold(&cfrags, threshold)?;

        // Step 4-5: Decrypt Capsule using PRE and recover symmetric key
        let symmetric_key = self.decrypt_capsule(
            &capsule,
            &cfrags,
            &request.requester_secret_key,
            &request.owner_public_key,
            &ciphertext,
            &verifying_pk,
        )?;

        // Step 6: Decrypt each share with AES-GCM
        let decrypted_shares = self.decrypt_shares(&encrypted_shares, &symmetric_key)?;

        // Step 7: Reconstruct secret using Shamir
        let recovered_secret = self.reconstruct_secret(&decrypted_shares, threshold)?;

        // Step 8: Record audit trail
        let audit_tx_id =
            self.record_audit_trail(&request.secret_id, &request.requester_process_id)?;

        Ok(SecretRecoveryResult {
            recovered_secret,
            audit_tx_id,
        })
    }

    async fn can_recover(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<bool> {
        let (_, _, _, threshold) = self.retrieve_capsule_with_metadata(secret_id)?;

        // Retrieve available cFrags
        let cfrags = self
            .retrieve_cfrags(secret_id, requester_process_id)
            .await?;

        // Check if we have enough
        Ok(cfrags.len() as u8 >= threshold)
    }
}
