//! SecretSharingWorkflowService - Phase 1 Secret Sharing Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 1 workflow: secret splitting, encryption, kFrag generation, and storage.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{
    constants, CapsulePayload, KeyFragment, PublicKey, ShamirShare,
};
use crate::usecase::core::storage::{QueryParams, Tag};
use crate::usecase::dto::{SecretSharingRequest, SecretSharingResult, SecretStatus};
use crate::usecase::error::{WorkflowError, WorkflowResult};
use crate::usecase::service::{CryptoService, StorageService};

// ============================================================================
// DelegateCapsule Retry Config
// ============================================================================

/// Maximum number of retries for each DelegateCapsule call (excluding the initial attempt).
/// AO contract uses IDEM_FLAGS to ensure idempotency on success; retries on failure are safe.
const DELEGATE_CAPSULE_MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (doubles each retry: 100ms → 200ms → 400ms).
const DELEGATE_CAPSULE_RETRY_BASE_DELAY_MS: u64 = 100;

// ============================================================================
// SecretSharingWorkflowService Trait
// ============================================================================

/// SecretSharingWorkflowService trait - Phase 1 workflow interface
///
/// Defines the contract for secret sharing operations that orchestrate
/// CryptoService and StorageService to split, encrypt, and distribute secrets.
#[async_trait]
pub trait SecretSharingWorkflowService: Send + Sync {
    /// Execute the complete Phase 1 secret sharing workflow
    ///
    /// # Steps
    /// 1. Generate symmetric key kₒ
    /// 2. Split secret using Shamir Secret Sharing
    /// 3. Encrypt each share with AES-GCM using kₒ
    /// 4. Create PRE Capsule for kₒ
    /// 5. Generate re-encryption key from owner to requester
    /// 6. Create kFrags from re-encryption key
    /// 7. Send kFrags to Owner-Process via AO
    /// 8. Store Capsule and encrypted shares on Arweave
    ///
    /// # Arguments
    /// * `request` - SecretSharingRequest containing secret, keys, and parameters
    ///
    /// # Returns
    /// * `WorkflowResult<SecretSharingResult>` - Result containing secret ID and tx IDs
    ///
    /// # Errors
    /// * `WorkflowError::ValidationError` - Invalid input parameters
    /// * `WorkflowError::CryptoError` - Cryptographic operation failed
    /// * `WorkflowError::StorageError` - Arweave storage failed
    /// * `WorkflowError::AOCommunicationError` - AO Network communication failed
    async fn execute_secret_sharing(
        &self,
        request: SecretSharingRequest,
    ) -> WorkflowResult<SecretSharingResult>;

    /// Get the current status of a secret
    ///
    /// # Arguments
    /// * `secret_id` - The ID of the secret to query
    ///
    /// # Returns
    /// * `WorkflowResult<SecretStatus>` - Current status of the secret
    ///
    /// # Errors
    /// * `WorkflowError::ResourceNotFound` - Secret not found
    /// * `WorkflowError::StorageError` - Storage query failed
    async fn get_secret_status(&self, secret_id: &SecretId) -> WorkflowResult<SecretStatus>;
}

// ============================================================================
// SecretSharingWorkflowServiceImpl
// ============================================================================

/// Implementation of SecretSharingWorkflowService
///
/// Orchestrates CryptoService and StorageService to implement
/// the complete Phase 1 secret sharing workflow.
pub struct SecretSharingWorkflowServiceImpl<C: CryptoService, ST: StorageService> {
    crypto_service: Arc<C>,
    storage_service: Arc<ST>,
}

impl<C: CryptoService, ST: StorageService> SecretSharingWorkflowServiceImpl<C, ST> {
    /// Create a new SecretSharingWorkflowServiceImpl
    pub fn new(crypto_service: Arc<C>, storage_service: Arc<ST>) -> Self {
        Self {
            crypto_service,
            storage_service,
        }
    }

    /// Validate input parameters for secret sharing
    fn validate_request(&self, request: &SecretSharingRequest) -> WorkflowResult<()> {
        // Validate threshold
        if request.threshold < constants::MIN_THRESHOLD {
            return Err(WorkflowError::validation(format!(
                "Threshold must be at least {} (got {})",
                constants::MIN_THRESHOLD,
                request.threshold
            )));
        }

        // Validate total_shares
        if request.total_shares > constants::MAX_SHARES {
            return Err(WorkflowError::validation(format!(
                "Total shares cannot exceed {} (got {})",
                constants::MAX_SHARES,
                request.total_shares
            )));
        }

        // Validate threshold <= total_shares
        if request.threshold > request.total_shares {
            return Err(WorkflowError::validation(format!(
                "Threshold ({}) cannot exceed total shares ({})",
                request.threshold, request.total_shares
            )));
        }

        // Validate secret is not empty
        if request.secret.is_empty() {
            return Err(WorkflowError::validation("Secret cannot be empty"));
        }

        // Validate owner public key
        if request.owner_public_key.key_data.is_empty() {
            return Err(WorkflowError::validation("Owner public key is required"));
        }

        // Validate requester public key
        if request.requester_public_key.key_data.is_empty() {
            return Err(WorkflowError::validation(
                "Requester public key is required",
            ));
        }

        // Validate owner process ID
        if request.owner_process_id.is_empty() {
            return Err(WorkflowError::validation("Owner process ID is required"));
        }

        Ok(())
    }

    /// Generate symmetric key for share encryption
    fn generate_symmetric_key(&self) -> WorkflowResult<Vec<u8>> {
        self.crypto_service
            .generate_symmetric_key()
            .map_err(WorkflowError::from)
    }

    /// Split secret using Shamir Secret Sharing
    fn split_secret(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> WorkflowResult<Vec<ShamirShare>> {
        self.crypto_service
            .split_secret_shamir(secret, threshold, total_shares)
            .map_err(WorkflowError::from)
    }

    /// Encrypt shares with AES-GCM
    fn encrypt_shares(
        &self,
        shares: &[ShamirShare],
        symmetric_key: &[u8],
    ) -> WorkflowResult<Vec<Vec<u8>>> {
        shares
            .iter()
            .map(|share| {
                self.crypto_service
                    .aes_gcm_encrypt(symmetric_key, &share.share_data)
                    .map_err(WorkflowError::from)
            })
            .collect()
    }

    /// Create PRE Capsule for the symmetric key
    fn create_capsule(
        &self,
        owner_public_key: &PublicKey,
        symmetric_key: &[u8],
    ) -> WorkflowResult<(crate::usecase::core::crypto::Capsule, Vec<u8>)> {
        self.crypto_service
            .create_pre_capsule(owner_public_key, symmetric_key)
            .map_err(WorkflowError::from)
    }

    /// Generate kFrags for re-encryption
    fn generate_kfrags(&self, request: &SecretSharingRequest) -> WorkflowResult<Vec<KeyFragment>> {
        // Generate re-encryption key
        let rekey = self
            .crypto_service
            .generate_reencryption_key(&request.owner_secret_key, &request.requester_public_key)
            .map_err(WorkflowError::from)?;

        // Create kFrags
        self.crypto_service
            .create_kfrags(&rekey, request.threshold, request.total_shares)
            .map_err(WorkflowError::from)
    }
}

#[async_trait]
impl<C: CryptoService, ST: StorageService> SecretSharingWorkflowService
    for SecretSharingWorkflowServiceImpl<C, ST>
{
    async fn execute_secret_sharing(
        &self,
        request: SecretSharingRequest,
    ) -> WorkflowResult<SecretSharingResult> {
        // Step 0: Validate input parameters
        self.validate_request(&request)?;

        // Step 1: Generate symmetric key kₒ (wrapped with Zeroizing for automatic cleanup)
        let symmetric_key: Zeroizing<Vec<u8>> = Zeroizing::new(self.generate_symmetric_key()?);

        // Step 2: Split secret using Shamir Secret Sharing
        let shares = self.split_secret(&request.secret, request.threshold, request.total_shares)?;

        // Step 3: Encrypt each share with AES-GCM using kₒ
        let encrypted_shares = self.encrypt_shares(&shares, &symmetric_key)?;

        // Step 4: Create PRE Capsule for kₒ
        let (capsule, ciphertext) =
            self.create_capsule(&request.owner_public_key, &symmetric_key)?;

        // Step 4b: Get verifying_pk for CapsulePayload
        let verifying_pk_bytes = self
            .crypto_service
            .verifying_key_bytes()
            .map_err(WorkflowError::from)?;

        // Step 5: Generate secret_id before kFrag sending (binds kFrags to this secret)
        let secret_id = SecretId::generate();

        // Step 5-6: Generate re-encryption key and create kFrags
        let kfrags = self.generate_kfrags(&request)?;
        let kfrag_count = kfrags.len() as u8;

        // Step 7: Send kFrags to Owner-Process via AO
        // kfrag_id format: {secret_id}_{index}
        self.storage_service
            .send_kfrags_to_contract(&kfrags, &request.owner_process_id, secret_id.as_str())
            .await
            .map_err(WorkflowError::from)?;

        // Step 8: Store CapsulePayload (capsule + ciphertext + verifying_pk) on Arweave

        let payload = CapsulePayload {
            capsule_bytes: capsule.capsule_bytes.clone(),
            ciphertext,
            verifying_pk: verifying_pk_bytes,
            threshold_k: request.threshold,
            threshold_n: request.total_shares,
        };
        let payload_bytes = bincode::serialize(&payload)
            .map_err(|_| WorkflowError::crypto("Failed to serialize CapsulePayload"))?;

        let capsule_tx_id = self
            .storage_service
            .store_data(
                &payload_bytes,
                vec![
                    Tag {
                        name: "type".to_string(),
                        value: "capsule".to_string(),
                    },
                    Tag {
                        name: "secret_id".to_string(),
                        value: secret_id.as_str().to_string(),
                    },
                    Tag {
                        name: "threshold_k".to_string(),
                        value: request.threshold.to_string(),
                    },
                    Tag {
                        name: "threshold_n".to_string(),
                        value: request.total_shares.to_string(),
                    },
                ],
            )
            .await
            .map_err(WorkflowError::from)?;

        // Step 9: Store encrypted shares on Arweave
        //
        // Shares are stored BEFORE DelegateCapsule (Step 9b) to ensure all
        // Arweave data (capsule + shares) is durable before the AO contract is
        // notified. If AO triggers re-encryption immediately on DelegateCapsule,
        // the shares must already be available for Phase 3 retrieval.
        let share_items: Vec<(Vec<u8>, Vec<Tag>)> = encrypted_shares
            .into_iter()
            .enumerate()
            .map(|(i, encrypted_share)| {
                (
                    encrypted_share,
                    vec![
                        Tag {
                            name: "type".to_string(),
                            value: "encrypted_share".to_string(),
                        },
                        Tag {
                            name: "secret_id".to_string(),
                            value: secret_id.as_str().to_string(),
                        },
                        Tag {
                            name: "index".to_string(),
                            value: i.to_string(),
                        },
                    ],
                )
            })
            .collect();

        let batch_result = self
            .storage_service
            .batch_store(share_items)
            .await
            .map_err(WorkflowError::from)?;
        if !batch_result.failed.is_empty() {
            let failed_count = batch_result.failed.len();
            let total_count = failed_count + batch_result.successful.len();
            return Err(WorkflowError::PartialStorageFailure {
                capsule_tx_id,
                successful_share_tx_ids: batch_result.successful,
                failed_shares: batch_result.failed,
                failed_count,
                total_count,
            });
        }
        let share_tx_ids = batch_result.successful;

        // Step 9b: DelegateCapsule to AO for each kFrag
        //
        // Called AFTER both capsule and shares are safely on Arweave (Steps 8 & 9).
        // This prevents AO from triggering re-encryption before shares are available.
        //
        // Retry with exponential backoff per kFrag (safe because the AO contract
        // implements IDEM_FLAGS: successful calls return NoOp on re-submission,
        // failed calls re-process cleanly from scratch).
        // share_tx_ids is included in any error so callers know Arweave is intact.
        let mut delegate_successful_ids: Vec<String> = Vec::with_capacity(kfrags.len());
        let mut delegate_failed_ids: Vec<(String, String)> = Vec::new();

        for kfrag in &kfrags {
            // Must match the format used in send_kfrags: {secret_id}_{index}
            let kfrag_id = format!("{}_{}", secret_id.as_str(), kfrag.id);
            let mut last_err: Option<String> = None;

            for attempt in 0..=DELEGATE_CAPSULE_MAX_RETRIES {
                if attempt > 0 {
                    // Exponential backoff: 100ms, 200ms, 400ms
                    let delay_ms = DELEGATE_CAPSULE_RETRY_BASE_DELAY_MS * (1u64 << (attempt - 1));
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }

                match self
                    .storage_service
                    .delegate_capsule(
                        &capsule.capsule_bytes,
                        &request.owner_process_id,
                        &kfrag_id,
                        &capsule_tx_id,
                    )
                    .await
                {
                    Ok(()) => {
                        last_err = None;
                        break; // success — stop retrying this kFrag
                    }
                    Err(e) => {
                        last_err = Some(e.to_string());
                        // continue to next attempt
                    }
                }
            }

            match last_err {
                None => delegate_successful_ids.push(kfrag_id),
                Some(err) => delegate_failed_ids.push((kfrag_id, err)),
            }
        }

        if !delegate_failed_ids.is_empty() {
            return Err(WorkflowError::partial_delegate_capsule_failure(
                &capsule_tx_id,
                share_tx_ids,
                delegate_successful_ids,
                delegate_failed_ids,
            ));
        }

        Ok(SecretSharingResult {
            secret_id,
            capsule_tx_id,
            share_tx_ids,
            kfrag_count,
            owner_public_key: request.owner_public_key.clone(),
        })
    }

    async fn get_secret_status(&self, secret_id: &SecretId) -> WorkflowResult<SecretStatus> {
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

        let results = self
            .storage_service
            .query_by_tags(params)
            .await
            .map_err(WorkflowError::from)?;

        if results.is_empty() {
            return Err(WorkflowError::not_found(format!(
                "Secret {} not found",
                secret_id
            )));
        }

        Ok(SecretStatus::Created)
    }
}
