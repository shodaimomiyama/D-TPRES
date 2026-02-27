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
use crate::usecase::core::storage::{QueryParams, SortBy, SortOrder, Tag};
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
    ///
    /// Generates kfrag_ids from `{secret_id}_{0..total_shares}` convention,
    /// then queries each kfrag's cFrag individually via GetCFrag.
    ///
    /// # Architecture Note: Single-Contract Design
    ///
    /// The `process_id` parameter is the ID of the **single deployed AO contract**
    /// that handles all roles (Owner / Holder / Requester) in one instance.
    /// The same contract is used in Phase 1 (`owner_process_id`) and Phase 3
    /// (`requester_process_id`). Passing `requester_process_id` here is correct
    /// because it refers to the same contract.
    ///
    /// # Known Issue: capsule_id
    ///
    /// Currently `capsule_id` is set to `secret_id`, but the AO contract stores
    /// cFrags keyed by the Arweave `capsule_tx_id`. This will be resolved once
    /// `DelegateCapsule` is implemented in Phase 1 (see Issue #70).
    async fn retrieve_cfrags(
        &self,
        secret_id: &SecretId,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> WorkflowResult<Vec<CFragData>> {
        self.storage_service
            .retrieve_cfrags(secret_id.as_str(), total_shares, capsule_id, process_id)
            .await
            .map_err(WorkflowError::from)
    }

    /// Retrieve Capsule, ciphertext, verifying_pk, thresholds, and capsule_tx_id from Arweave
    ///
    /// Returns `(capsule, ciphertext, verifying_pk, threshold_k, threshold_n, capsule_tx_id)`.
    /// - `threshold_k` – minimum kFrags required for reconstruction
    /// - `threshold_n` – total kFrags generated (stored for audit / future use)
    /// - `capsule_tx_id` – Arweave transaction ID of the stored capsule (= `capsule_id` for AO)
    #[allow(clippy::type_complexity)]
    async fn retrieve_capsule_with_metadata(
        &self,
        secret_id: &SecretId,
    ) -> WorkflowResult<(Capsule, Vec<u8>, Vec<u8>, u8, u8, String)> {
        // Ideally there is exactly one capsule per secret_id (UUID v4), but if multiple
        // transactions exist for the same secret_id (e.g. due to retries), we pick the
        // most-recently stored one by sorting descending by timestamp.
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
            sort_by: Some(SortBy::Timestamp(SortOrder::Descending)),
        };

        let transactions = self
            .storage_service
            .query_by_tags(params)
            .await
            .map_err(WorkflowError::from)?;

        let tx = transactions.into_iter().next().ok_or_else(|| {
            WorkflowError::not_found(format!("Capsule not found for secret {secret_id}"))
        })?;

        let capsule_tx_id = tx.id.clone();

        // Deserialize CapsulePayload first — threshold values are now embedded in the
        // payload (fail-safe), so we no longer depend on Arweave tags being populated.
        let payload: CapsulePayload = bincode::deserialize(&tx.data)
            .map_err(|_| WorkflowError::crypto("Failed to deserialize CapsulePayload"))?;

        let threshold_k = payload.threshold_k;
        let threshold_n = payload.threshold_n;

        // Validate threshold values immediately after retrieval
        const MIN_THRESHOLD: u8 = 2;
        if threshold_k < MIN_THRESHOLD {
            return Err(WorkflowError::validation(format!(
                "threshold_k ({threshold_k}) must be >= {MIN_THRESHOLD}"
            )));
        }
        if threshold_n == 0 {
            return Err(WorkflowError::validation("threshold_n must be > 0"));
        }
        if threshold_k > threshold_n {
            return Err(WorkflowError::validation(format!(
                "threshold_k ({threshold_k}) must be <= threshold_n ({threshold_n})"
            )));
        }

        let capsule = Capsule {
            capsule_bytes: payload.capsule_bytes,
        };

        Ok((
            capsule,
            payload.ciphertext,
            payload.verifying_pk,
            threshold_k,
            threshold_n,
            capsule_tx_id,
        ))
    }

    /// Retrieve encrypted shares from Arweave via StorageService
    async fn retrieve_encrypted_shares(
        &self,
        secret_id: &SecretId,
    ) -> WorkflowResult<Vec<Vec<u8>>> {
        self.storage_service
            .retrieve_encrypted_shares(secret_id.as_str())
            .await
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

    /// Decrypt shares with AES-GCM, skipping corrupted/invalid shares.
    ///
    /// If a share fails to decrypt it is logged and skipped rather than aborting
    /// the entire recovery.  The caller (`execute_secret_recovery`) checks that at
    /// least `threshold` shares were successfully decrypted before proceeding to
    /// Shamir reconstruction.
    fn decrypt_shares(
        &self,
        encrypted_shares: &[Vec<u8>],
        symmetric_key: &[u8],
        threshold: u8,
    ) -> WorkflowResult<Vec<ShamirShare>> {
        let mut decrypted = Vec::with_capacity(encrypted_shares.len());

        for (i, encrypted_share) in encrypted_shares.iter().enumerate() {
            match self
                .crypto_service
                .aes_gcm_decrypt(symmetric_key, encrypted_share)
            {
                Err(e) => {
                    // Skip this share and continue — a corrupted/missing share must not
                    // prevent recovery when enough valid shares remain.
                    log::warn!(
                        "decrypt_shares: share {i} failed AES-GCM decryption, skipping: {e}"
                    );
                }
                Ok(plaintext) if plaintext.is_empty() => {
                    log::warn!("decrypt_shares: share {i} decrypted to empty bytes, skipping");
                }
                Ok(plaintext) => {
                    // shamirsecretsharing library embeds the index in the first byte
                    let index = plaintext[0];
                    decrypted.push(ShamirShare {
                        index,
                        share_data: plaintext,
                    });
                }
            }
        }

        // Verify we still have enough valid shares to meet the threshold
        let available = decrypted.len() as u8;
        if available < threshold {
            return Err(WorkflowError::insufficient_cfrags(threshold, available));
        }

        Ok(decrypted)
    }

    /// Reconstruct secret using Shamir Secret Sharing
    fn reconstruct_secret(&self, shares: &[ShamirShare], threshold: u8) -> WorkflowResult<Vec<u8>> {
        self.crypto_service
            .reconstruct_secret_shamir(shares, threshold)
            .map_err(|e| WorkflowError::DecryptionError {
                phase: format!("Shamir reconstruction: {e}"),
            })
    }

    /// Record an audit trail entry on Arweave.
    ///
    /// Stores a minimal JSON record containing:
    /// - `secret_id` — which secret was accessed
    /// - `requester_process_id` — who requested access
    /// - `status` — outcome (`"success"` when called from the happy path)
    /// - `timestamp_secs` — Unix timestamp of the event
    ///
    /// TODO: also call this from error paths in `execute_secret_recovery` so that
    /// failed recovery attempts are recorded as well.
    async fn record_audit_trail(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let audit_record = serde_json::json!({
            "secret_id": secret_id.as_str(),
            "requester_process_id": requester_process_id,
            "status": "success",
            "timestamp_secs": timestamp,
        })
        .to_string();

        self.storage_service
            .store_data(
                audit_record.as_bytes(),
                vec![
                    Tag {
                        name: "type".to_string(),
                        value: "audit_trail".to_string(),
                    },
                    Tag {
                        name: "secret_id".to_string(),
                        value: secret_id.as_str().to_string(),
                    },
                    Tag {
                        name: "requester_process_id".to_string(),
                        value: requester_process_id.to_string(),
                    },
                    Tag {
                        name: "status".to_string(),
                        value: "success".to_string(),
                    },
                    Tag {
                        name: "timestamp_secs".to_string(),
                        value: timestamp.to_string(),
                    },
                ],
            )
            .await
            .map_err(WorkflowError::from)
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

        // Step 1: Retrieve CapsulePayload (capsule + ciphertext + verifying_pk + thresholds) from Arweave
        let (capsule, ciphertext, verifying_pk, threshold, threshold_n, capsule_tx_id) = self
            .retrieve_capsule_with_metadata(&request.secret_id)
            .await?;

        // Step 2: Retrieve cFrags from AO using kfrag_id convention {secret_id}_{index}
        let cfrags = self
            .retrieve_cfrags(
                &request.secret_id,
                threshold_n,
                &capsule_tx_id,
                &request.requester_process_id,
            )
            .await?;

        // Step 3: Retrieve encrypted shares and verify threshold
        let encrypted_shares = self.retrieve_encrypted_shares(&request.secret_id).await?;
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

        // Step 6: Decrypt each share with AES-GCM (corrupt shares are skipped)
        let decrypted_shares = self.decrypt_shares(&encrypted_shares, &symmetric_key, threshold)?;

        // Step 7: Reconstruct secret using Shamir
        let recovered_secret = self.reconstruct_secret(&decrypted_shares, threshold)?;

        // Step 8: Record audit trail
        let audit_tx_id = self
            .record_audit_trail(&request.secret_id, &request.requester_process_id)
            .await?;

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
        let (_, _, _, threshold, threshold_n, capsule_tx_id) =
            self.retrieve_capsule_with_metadata(secret_id).await?;

        let cfrags = self
            .retrieve_cfrags(secret_id, threshold_n, &capsule_tx_id, requester_process_id)
            .await?;

        Ok(cfrags.len() as u8 >= threshold)
    }
}
