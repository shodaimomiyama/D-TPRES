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

    /// Retrieve Capsule, ciphertext, verifying_pk, threshold_k, and threshold_n from Arweave
    #[allow(clippy::type_complexity)]
    fn retrieve_capsule_with_metadata(
        &self,
        secret_id: &SecretId,
    ) -> WorkflowResult<(Capsule, Vec<u8>, Vec<u8>, u8, u8)> {
        // Ideally there is exactly one capsule per secret_id (UUID v4), but if multiple
        // transactions exist for the same secret_id (e.g. due to retries), we pick the
        // most-recently stored one by sorting descending by timestamp.
        // TODO: Verify that the Arweave GraphQL API used by ArweaveStorageServiceImpl
        //       propagates the SortBy::Timestamp ordering to the underlying query;
        //       the in-memory implementation already supports it.
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

        let threshold_n = tx
            .tags
            .iter()
            .find(|t| t.name == "threshold_n")
            .ok_or_else(|| {
                WorkflowError::not_found("threshold_n tag missing from capsule transaction")
            })?
            .value
            .parse::<u8>()
            .map_err(|e| WorkflowError::validation(format!("Invalid threshold_n value: {e}")))?;

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
            threshold_n,
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
                Ok(data) if data.is_empty() => {
                    log::warn!("decrypt_shares: share {i} decrypted to empty bytes, skipping");
                }
                Ok(data) => {
                    // shamirsecretsharing library embeds the index in the first byte
                    let index = data[0];
                    decrypted.push(ShamirShare {
                        index,
                        share_data: data,
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
    fn record_audit_trail(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let audit_record = format!(
            r#"{{"secret_id":"{secret_id}","requester_process_id":"{requester_process_id}","status":"success","timestamp_secs":{timestamp}}}"#
        );

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
        let (capsule, ciphertext, verifying_pk, threshold, threshold_n) =
            self.retrieve_capsule_with_metadata(&request.secret_id)?;

        // Step 2: Retrieve cFrags from AO using kfrag_id convention {secret_id}_{index}
        let cfrags = self
            .retrieve_cfrags(
                &request.secret_id,
                threshold_n,
                request.secret_id.as_str(),
                &request.requester_process_id,
            )
            .await?;

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

        // Step 6: Decrypt each share with AES-GCM (corrupt shares are skipped)
        let decrypted_shares = self.decrypt_shares(&encrypted_shares, &symmetric_key, threshold)?;

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
        let (_, _, _, threshold, threshold_n) = self.retrieve_capsule_with_metadata(secret_id)?;

        let cfrags = self
            .retrieve_cfrags(
                secret_id,
                threshold_n,
                secret_id.as_str(),
                requester_process_id,
            )
            .await?;

        Ok(cfrags.len() as u8 >= threshold)
    }
}

#[cfg(test)]
#[allow(
    clippy::uninlined_format_args,
    clippy::indexing_slicing,
    clippy::redundant_clone,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects
)]
mod tests {
    use super::*;
    use crate::adapter::external::ao::{AOClient, ExecuteMsg};
    use crate::adapter::external::mock_ao::MockAOClient;
    use crate::usecase::core::contract_storage::ContractStorageImpl;
    use crate::usecase::core::crypto::{
        CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
    };
    use crate::usecase::core::storage::{ArweaveStorageService, ArweaveStorageServiceImpl};
    use crate::usecase::service::{
        CryptoServiceImpl as ServiceCryptoServiceImpl,
        StorageServiceImpl as ServiceStorageServiceImpl,
    };

    type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
    type TestStorageService =
        ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

    fn create_test_service()
    -> SecretRecoveryWorkflowServiceImpl<TestCryptoService, TestStorageService> {
        let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
        let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));

        let mock_ao = Arc::new(MockAOClient::new());
        let arweave = Arc::new(ArweaveStorageServiceImpl::default());
        let contract = Arc::new(ContractStorageImpl::new(mock_ao));
        let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));

        SecretRecoveryWorkflowServiceImpl::new(crypto, storage)
    }

    fn create_test_request(crypto: &CoreCryptoServiceImpl) -> SecretRecoveryRequest {
        let (requester_sk, _requester_pk) = crypto.generate_keypair().unwrap();
        let (_, owner_pk) = crypto.generate_keypair().unwrap();

        SecretRecoveryRequest {
            secret_id: SecretId::generate(),
            requester_secret_key: requester_sk,
            owner_public_key: owner_pk,
            requester_process_id: "requester-process-123".to_string(),
        }
    }

    fn create_mock_cfrags(count: usize) -> Vec<CFragData> {
        (0..count)
            .map(|i| CFragData {
                cfrag_data: vec![i as u8; 32],
                holder_id: format!("holder-{}", i),
            })
            .collect()
    }

    struct TestComponents {
        arweave: Arc<ArweaveStorageServiceImpl>,
        mock_ao: Arc<MockAOClient>,
    }

    fn create_test_service_with_components() -> (
        SecretRecoveryWorkflowServiceImpl<TestCryptoService, TestStorageService>,
        TestComponents,
    ) {
        let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
        let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
        let mock_ao = Arc::new(MockAOClient::new());
        let arweave = Arc::new(ArweaveStorageServiceImpl::default());
        let contract = Arc::new(ContractStorageImpl::new(Arc::clone(&mock_ao)));
        let storage = Arc::new(ServiceStorageServiceImpl::new(
            Arc::clone(&arweave),
            contract,
        ));
        let service = SecretRecoveryWorkflowServiceImpl::new(crypto, storage);
        (service, TestComponents { arweave, mock_ao })
    }

    // ========================================================================
    // Validation Tests (Task 17)
    // ========================================================================

    #[test]
    fn test_validate_request_valid() {
        println!("\n=== test_validate_request_valid ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!("  Created request with secret_id: {}", request.secret_id);

        let result = service.validate_request(&request);
        println!("  Validation result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Valid request accepted");
    }

    // Note: test_phase3_invalid_accessor_key is not implemented because SecretKey
    // has private fields and cannot be created with empty/invalid data directly.
    // This is a security feature - valid SecretKeys can only be created through
    // the crypto service which generates proper cryptographic keys.

    #[test]
    fn test_validate_request_empty_process_id() {
        println!("\n=== test_validate_request_empty_process_id ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.requester_process_id = String::new();
        println!("  Testing with empty requester_process_id");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("process ID"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase3_validates_cfrag_count() {
        println!("\n=== test_phase3_validates_cfrag_count ===");
        let service = create_test_service();

        // Exactly threshold
        println!("  Testing with exactly threshold (3 cFrags, threshold=3)...");
        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());

        // Above threshold
        println!("  Testing above threshold (5 cFrags, threshold=3)...");
        let cfrags = create_mock_cfrags(5);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] cFrag count validation works correctly");
    }

    #[test]
    fn test_phase3_insufficient_cfrags() {
        println!("\n=== test_phase3_insufficient_cfrags ===");
        let service = create_test_service();

        println!("  Testing with insufficient cFrags (2 cFrags, threshold=3)...");
        let cfrags = create_mock_cfrags(2);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result);

        assert!(matches!(
            result,
            Err(WorkflowError::InsufficientCFrags {
                required: 3,
                actual: 2
            })
        ));

        // Verify error message format
        if let Err(err) = result {
            let msg = err.to_string();
            assert!(msg.contains("Insufficient cFrags"));
            assert!(msg.contains("need 3"));
            assert!(msg.contains("got 2"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[tokio::test]
    async fn test_phase3_retrieve_cfrags_empty_when_no_data() {
        println!("\n=== test_phase3_retrieve_cfrags_empty_when_no_data ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving cFrags for secret_id: {}", secret_id);

        let result = service
            .retrieve_cfrags(&secret_id, 3, secret_id.as_str(), "requester-123")
            .await;
        println!("  Result: {:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        println!("  [PASS] Empty cFrags returned when no data set");
    }

    #[tokio::test]
    async fn test_phase3_resource_not_found_capsule() {
        println!("\n=== test_phase3_resource_not_found_capsule ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving capsule for secret_id: {}", secret_id);

        let result = service.retrieve_capsule_with_metadata(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] ResourceNotFound returned (no capsule in Arweave)");
    }

    #[test]
    fn test_phase3_retrieve_encrypted_shares_empty_when_no_data() {
        println!("\n=== test_phase3_retrieve_encrypted_shares_empty_when_no_data ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving encrypted shares for secret_id: {}", secret_id);

        let result = service.retrieve_encrypted_shares(&secret_id);
        println!("  Result: {:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        println!("  [PASS] Empty shares returned when no data stored");
    }

    // ========================================================================
    // Workflow Tests (Task 18)
    // ========================================================================

    #[tokio::test]
    async fn test_execute_recovery_fails_at_capsule_retrieval() {
        println!("\n=== test_execute_recovery_fails_at_capsule_retrieval ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!("  Executing PHASE 3 recovery (should fail at capsule retrieval)...");

        let result = service.execute_secret_recovery(request).await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Correctly failed at capsule retrieval");
    }

    #[tokio::test]
    async fn test_can_recover_fails_at_capsule_retrieval() {
        println!("\n=== test_can_recover_fails_at_capsule_retrieval ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Checking can_recover for secret_id: {}", secret_id);

        let result = service
            .can_recover(&secret_id, "requester-process-123")
            .await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Correctly failed at capsule retrieval");
    }

    #[test]
    fn test_verify_threshold_success() {
        println!("\n=== test_verify_threshold_success ===");
        let service = create_test_service();

        println!("  Verifying threshold with 3 cFrags, threshold=3...");
        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Threshold verification succeeded");
    }

    #[test]
    fn test_verify_threshold_insufficient() {
        println!("\n=== test_verify_threshold_insufficient ===");
        let service = create_test_service();

        println!("  Verifying threshold with 2 cFrags, threshold=3...");
        let cfrags = create_mock_cfrags(2);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result);
        assert!(matches!(
            result,
            Err(WorkflowError::InsufficientCFrags {
                required: 3,
                actual: 2
            })
        ));
        println!("  [PASS] Correctly identified insufficient cFrags");
    }

    #[test]
    fn test_phase3_exactly_k_cfrags() {
        println!("\n=== test_phase3_exactly_k_cfrags ===");
        let service = create_test_service();

        // Exactly k (threshold) cFrags should be sufficient
        println!("  Testing exactly k=3 cFrags...");
        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Exactly k cFrags is sufficient");
    }

    #[test]
    fn test_phase3_more_than_k_cfrags() {
        println!("\n=== test_phase3_more_than_k_cfrags ===");
        let service = create_test_service();

        // More than k cFrags should also work
        println!("  Testing with 7 cFrags > threshold=3...");
        let cfrags = create_mock_cfrags(7);
        let result = service.verify_threshold(&cfrags, 3);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] More than k cFrags accepted");
    }

    #[test]
    fn test_phase3_zero_cfrags() {
        println!("\n=== test_phase3_zero_cfrags ===");
        let service = create_test_service();

        println!("  Testing with 0 cFrags, threshold=2...");
        let cfrags: Vec<CFragData> = vec![];
        let result = service.verify_threshold(&cfrags, 2);
        println!("  Result: {:?}", result);
        assert!(matches!(
            result,
            Err(WorkflowError::InsufficientCFrags {
                required: 2,
                actual: 0
            })
        ));
        println!("  [PASS] Zero cFrags correctly rejected");
    }

    #[test]
    fn test_record_audit_trail_placeholder() {
        println!("\n=== test_record_audit_trail_placeholder ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Recording audit trail for secret_id: {}", secret_id);

        let result = service.record_audit_trail(&secret_id, "requester-123");
        println!("  Result: {:?}", result);
        assert!(result.is_ok());

        // Placeholder returns a specific string
        let audit_tx_id = result.unwrap();
        println!("  audit_tx_id: {}", audit_tx_id);
        assert!(!audit_tx_id.is_empty());
        println!("  [PASS] Audit trail recorded (placeholder)");
    }

    // ========================================================================
    // Decryption Helper Tests (Task 18 continued)
    // ========================================================================

    #[test]
    fn test_decrypt_shares_with_valid_key() {
        println!("\n=== test_decrypt_shares_with_valid_key ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        // Generate a valid symmetric key
        let symmetric_key = crypto.generate_symmetric_key().unwrap();
        println!("  Generated symmetric key");

        // Encrypt some test data
        let original_data = [b"share1".to_vec(), b"share2".to_vec()];
        println!(
            "  Original data: {:?}",
            original_data
                .iter()
                .map(|d| String::from_utf8_lossy(d).to_string())
                .collect::<Vec<_>>()
        );
        let encrypted_shares: Vec<Vec<u8>> = original_data
            .iter()
            .map(|data| crypto.aes_gcm_encrypt(&symmetric_key, data).unwrap())
            .collect();
        println!("  Encrypted {} shares", encrypted_shares.len());

        // Decrypt using the service
        println!("  Decrypting shares...");
        let result = service.decrypt_shares(&encrypted_shares, &symmetric_key);
        assert!(result.is_ok());

        let decrypted = result.unwrap();
        println!("  Decrypted {} shares", decrypted.len());
        assert_eq!(decrypted.len(), 2);

        // Verify decrypted data matches original
        println!(
            "  Decrypted[0]: {:?}",
            String::from_utf8_lossy(&decrypted[0].share_data)
        );
        println!(
            "  Decrypted[1]: {:?}",
            String::from_utf8_lossy(&decrypted[1].share_data)
        );
        assert_eq!(decrypted[0].share_data, b"share1");
        assert_eq!(decrypted[1].share_data, b"share2");
        println!("  [PASS] Decryption roundtrip successful");
    }

    #[test]
    fn test_decrypt_shares_with_invalid_key() {
        println!("\n=== test_decrypt_shares_with_invalid_key ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        let correct_key = crypto.generate_symmetric_key().unwrap();
        let wrong_key = crypto.generate_symmetric_key().unwrap();
        println!("  Generated two different keys");

        // Encrypt with correct key
        let encrypted_share = crypto.aes_gcm_encrypt(&correct_key, b"test data").unwrap();
        println!("  Encrypted with correct key");

        // Try to decrypt with wrong key
        println!("  Attempting to decrypt with wrong key...");
        let result = service.decrypt_shares(&[encrypted_share], &wrong_key);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::DecryptionError { .. })));
        println!("  [PASS] Correctly failed with wrong key");
    }

    #[test]
    fn test_decrypt_shares_empty_input() {
        println!("\n=== test_decrypt_shares_empty_input ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let symmetric_key = crypto.generate_symmetric_key().unwrap();

        println!("  Testing with empty shares array...");
        let encrypted_shares: Vec<Vec<u8>> = vec![];
        let result = service.decrypt_shares(&encrypted_shares, &symmetric_key);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());

        let decrypted = result.unwrap();
        println!("  Decrypted {} shares", decrypted.len());
        assert!(decrypted.is_empty());
        println!("  [PASS] Empty input handled correctly");
    }

    // ========================================================================
    // Shamir Reconstruction Tests (Task 18 continued)
    // ========================================================================

    #[test]
    fn test_reconstruct_secret_with_valid_shares() {
        println!("\n=== test_reconstruct_secret_with_valid_shares ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        // Create original secret and split it
        let original_secret = b"Test secret!".to_vec();
        println!(
            "  Original secret: {:?}",
            String::from_utf8_lossy(&original_secret)
        );
        let shares = crypto.split_secret_shamir(&original_secret, 3, 5).unwrap();
        println!("  Split into {} shares (threshold=3)", shares.len());

        // Take exactly threshold number of shares
        let subset: Vec<ShamirShare> = shares.into_iter().take(3).collect();
        println!("  Using {} shares for reconstruction", subset.len());

        // Reconstruct
        println!("  Reconstructing secret...");
        let result = service.reconstruct_secret(&subset, 3);
        assert!(result.is_ok());

        let recovered = result.unwrap();
        println!(
            "  Recovered secret: {:?}",
            String::from_utf8_lossy(&recovered)
        );
        assert_eq!(recovered, original_secret);
        println!("  [PASS] Secret reconstructed correctly");
    }

    #[test]
    fn test_reconstruct_secret_insufficient_shares() {
        println!("\n=== test_reconstruct_secret_insufficient_shares ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        let original_secret = b"Test secret!".to_vec();
        println!(
            "  Original secret: {:?}",
            String::from_utf8_lossy(&original_secret)
        );
        let shares = crypto.split_secret_shamir(&original_secret, 3, 5).unwrap();
        println!("  Split into {} shares (threshold=3)", shares.len());

        // Take less than threshold
        let subset: Vec<ShamirShare> = shares.into_iter().take(2).collect();
        println!("  Using only {} shares (less than threshold)", subset.len());

        let result = service.reconstruct_secret(&subset, 3);
        println!("  Result: {:?}", result);
        // Reconstruction should fail due to insufficient shares
        assert!(result.is_err());
        println!("  [PASS] Correctly failed with insufficient shares");
    }

    // ========================================================================
    // Error Handling Tests (Task 19)
    // ========================================================================

    #[test]
    fn test_phase3_decryption_error_message() {
        println!("\n=== test_phase3_decryption_error_message ===");
        let err = WorkflowError::DecryptionError {
            phase: "PRE decapsulation".to_string(),
        };
        let msg = err.to_string();
        println!("  Error message: {}", msg);
        assert!(msg.contains("Decryption failed"));
        assert!(msg.contains("PRE decapsulation"));
        println!("  [PASS] Decryption error message format correct");
    }

    #[tokio::test]
    async fn test_phase3_ao_retrieval_returns_empty_when_no_data() {
        println!("\n=== test_phase3_ao_retrieval_returns_empty_when_no_data ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Testing AO retrieval for secret_id: {}", secret_id);

        let result = service
            .retrieve_cfrags(&secret_id, 3, secret_id.as_str(), "requester-123")
            .await;
        println!("  Result: {:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        println!("  [PASS] AO retrieval returns empty Vec when no data set");
    }

    #[tokio::test]
    async fn test_phase3_storage_retrieval_behavior() {
        println!("\n=== test_phase3_storage_retrieval_behavior ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Testing storage retrieval for secret_id: {}", secret_id);

        // Capsule retrieval returns ResourceNotFound (no matching transaction)
        println!("  Testing capsule retrieval...");
        let result = service.retrieve_capsule_with_metadata(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        // Encrypted shares retrieval returns empty vec (no matching transactions)
        println!("  Testing encrypted shares retrieval...");
        let result = service.retrieve_encrypted_shares(&secret_id);
        println!("  Result: {:?}", result);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
        println!("  [PASS] Storage retrieval behavior correct");
    }

    #[tokio::test]
    async fn test_workflow_fails_at_capsule_retrieval() {
        println!("\n=== test_workflow_fails_at_capsule_retrieval ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!("  Executing workflow (should fail at capsule retrieval)...");

        let result = service.execute_secret_recovery(request).await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Workflow correctly fails at capsule retrieval");
    }

    #[tokio::test]
    async fn test_can_recover_fails_when_no_capsule() {
        println!("\n=== test_can_recover_fails_when_no_capsule ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Calling can_recover for secret_id: {}", secret_id);

        let result = service.can_recover(&secret_id, "requester-123").await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] can_recover fails when no capsule in Arweave");
    }

    // ========================================================================
    // CFragData Structure Tests
    // ========================================================================

    #[test]
    fn test_cfrag_data_structure() {
        println!("\n=== test_cfrag_data_structure ===");
        let cfrag = CFragData {
            cfrag_data: vec![1, 2, 3, 4],
            holder_id: "holder-abc".to_string(),
        };
        println!(
            "  Created CFragData: holder_id={}, data_len={}",
            cfrag.holder_id,
            cfrag.cfrag_data.len()
        );

        assert_eq!(cfrag.cfrag_data, vec![1, 2, 3, 4]);
        assert_eq!(cfrag.holder_id, "holder-abc");
        println!("  [PASS] CFragData structure works correctly");
    }

    #[test]
    fn test_cfrag_data_clone() {
        println!("\n=== test_cfrag_data_clone ===");
        let cfrag = CFragData {
            cfrag_data: vec![1, 2, 3, 4],
            holder_id: "holder-abc".to_string(),
        };
        println!("  Original: holder_id={}", cfrag.holder_id);

        let cloned = cfrag.clone();
        println!("  Cloned: holder_id={}", cloned.holder_id);
        assert_eq!(cloned.cfrag_data, cfrag.cfrag_data);
        assert_eq!(cloned.holder_id, cfrag.holder_id);
        println!("  [PASS] CFragData clone works correctly");
    }

    // ========================================================================
    // Integration Tests (Step 7.3)
    // ========================================================================

    #[tokio::test]
    async fn test_retrieve_capsule_with_metadata_success() {
        let (service, components) = create_test_service_with_components();
        let secret_id = SecretId::new("test-secret-abc");
        let capsule_data = vec![10, 20, 30, 40, 50];
        let ciphertext_data = vec![60, 70, 80];
        let verifying_pk_data = vec![90, 100, 110];

        let payload = CapsulePayload {
            capsule_bytes: capsule_data.clone(),
            ciphertext: ciphertext_data.clone(),
            verifying_pk: verifying_pk_data.clone(),
        };
        let payload_bytes = bincode::serialize(&payload).unwrap();

        components
            .arweave
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
                        value: "3".to_string(),
                    },
                    Tag {
                        name: "threshold_n".to_string(),
                        value: "5".to_string(),
                    },
                ],
            )
            .unwrap();

        let result = service.retrieve_capsule_with_metadata(&secret_id);
        assert!(result.is_ok());

        let (capsule, ciphertext, verifying_pk, threshold_k, threshold_n) = result.unwrap();
        assert_eq!(capsule.capsule_bytes, capsule_data);
        assert_eq!(ciphertext, ciphertext_data);
        assert_eq!(verifying_pk, verifying_pk_data);
        assert_eq!(threshold_k, 3);
        assert_eq!(threshold_n, 5);
    }

    #[tokio::test]
    async fn test_retrieve_capsule_missing_threshold_tag() {
        let (service, components) = create_test_service_with_components();
        let secret_id = SecretId::new("test-secret-def");
        let capsule_data = vec![10, 20, 30];

        components
            .arweave
            .store_data(
                &capsule_data,
                vec![
                    Tag {
                        name: "type".to_string(),
                        value: "capsule".to_string(),
                    },
                    Tag {
                        name: "secret_id".to_string(),
                        value: secret_id.as_str().to_string(),
                    },
                ],
            )
            .unwrap();

        let result = service.retrieve_capsule_with_metadata(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[tokio::test]
    async fn test_retrieve_cfrags_from_ao() {
        let (service, components) = create_test_service_with_components();
        let secret_id = SecretId::new("test-secret-ghi");
        let process_id = "requester-process-456";
        let capsule_id = secret_id.as_str();

        // Set up cFrags via Reencrypt for kfrag_ids following {secret_id}_{index} convention
        for i in 0..2u8 {
            let kfrag_id = format!("{}_{}", secret_id, i);
            components
                .mock_ao
                .execute(
                    process_id,
                    ExecuteMsg::Reencrypt {
                        kfrag_id,
                        capsule_id: capsule_id.to_string(),
                    },
                )
                .await
                .unwrap();
        }

        let result = service
            .retrieve_cfrags(&secret_id, 2, capsule_id, process_id)
            .await;
        assert!(result.is_ok());

        let cfrags = result.unwrap();
        assert_eq!(cfrags.len(), 2);
        assert_eq!(cfrags[0].holder_id, format!("{}_0", secret_id));
        assert_eq!(cfrags[1].holder_id, format!("{}_1", secret_id));
    }

    #[tokio::test]
    async fn test_retrieve_encrypted_shares_sorted() {
        let (service, components) = create_test_service_with_components();
        let secret_id = SecretId::new("test-secret-jkl");

        for i in 0..3u8 {
            components
                .arweave
                .store_data(
                    &[100 + i; 16],
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
                .unwrap();
        }

        let result = service.retrieve_encrypted_shares(&secret_id);
        assert!(result.is_ok());

        let shares = result.unwrap();
        assert_eq!(shares.len(), 3);
    }
}
