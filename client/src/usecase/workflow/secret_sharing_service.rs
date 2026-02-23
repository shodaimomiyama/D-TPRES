//! SecretSharingWorkflowService - Phase 1 Secret Sharing Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 1 workflow: secret splitting, encryption, kFrag generation, and storage.

use std::sync::Arc;

use async_trait::async_trait;
use zeroize::Zeroizing;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{
    CapsulePayload, KeyFragment, PublicKey, ShamirShare, constants,
};
use crate::usecase::core::storage::{QueryParams, Tag};
use crate::usecase::dto::{SecretSharingRequest, SecretSharingResult, SecretStatus};
use crate::usecase::error::{WorkflowError, WorkflowResult};
use crate::usecase::service::{CryptoService, StorageService};

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

#[cfg(test)]
#[allow(
    dead_code,
    clippy::uninlined_format_args,
    clippy::indexing_slicing,
    clippy::redundant_clone,
    clippy::cast_possible_truncation,
    clippy::arithmetic_side_effects,
    clippy::unwrap_used,
    clippy::type_complexity
)]
mod tests {
    use super::*;
    use crate::adapter::external::mock_ao::MockAOClient;
    use crate::service::error::ServiceResult;
    use crate::usecase::core::contract_storage::ContractStorageImpl;
    use crate::usecase::core::crypto::{
        CryptoService as CoreCryptoService, CryptoServiceImpl as CoreCryptoServiceImpl,
    };
    use crate::usecase::core::storage::{
        ArweaveStorageServiceImpl, ArweaveTransaction, BatchResult, QueryParams, Tag,
        TransactionStatus,
    };
    use crate::usecase::service::{
        CryptoServiceImpl as ServiceCryptoServiceImpl,
        StorageServiceImpl as ServiceStorageServiceImpl,
    };
    use std::sync::RwLock;

    type TestCryptoService = ServiceCryptoServiceImpl<CoreCryptoServiceImpl>;
    type TestStorageService =
        ServiceStorageServiceImpl<ArweaveStorageServiceImpl, ContractStorageImpl<MockAOClient>>;

    // ========================================================================
    // MockStorageService (Task 8)
    // ========================================================================

    /// Mock implementation of StorageService for testing
    struct MockStorageService {
        /// Stores all data passed to store_data()
        stored_data: Arc<RwLock<Vec<StoredItem>>>,
        /// Stores all batch items passed to batch_store()
        batch_items: Arc<RwLock<Vec<(Vec<u8>, Vec<Tag>)>>>,
        /// Stores kFrag calls
        kfrag_calls: Arc<RwLock<Vec<KFragCall>>>,
        /// Stores query calls
        query_results: Arc<RwLock<Vec<ArweaveTransaction>>>,
        /// Transaction counter
        tx_counter: Arc<RwLock<u64>>,
        /// Controls whether send_kfrag should fail
        should_fail_kfrag: Arc<RwLock<bool>>,
        /// Controls whether store_data should fail
        should_fail_store: Arc<RwLock<bool>>,
        /// Controls whether batch_store should partially fail (odd indices fail)
        should_fail_batch_partial: Arc<RwLock<bool>>,
    }

    #[derive(Debug, Clone)]
    struct StoredItem {
        data: Vec<u8>,
        tags: Vec<Tag>,
        tx_id: String,
    }

    #[derive(Debug, Clone)]
    struct KFragCall {
        kfrag_count: usize,
        owner_process_id: String,
    }

    impl MockStorageService {
        fn new() -> Self {
            Self {
                stored_data: Arc::new(RwLock::new(Vec::new())),
                batch_items: Arc::new(RwLock::new(Vec::new())),
                kfrag_calls: Arc::new(RwLock::new(Vec::new())),
                query_results: Arc::new(RwLock::new(Vec::new())),
                tx_counter: Arc::new(RwLock::new(0)),
                should_fail_kfrag: Arc::new(RwLock::new(false)),
                should_fail_store: Arc::new(RwLock::new(false)),
                should_fail_batch_partial: Arc::new(RwLock::new(false)),
            }
        }

        fn get_stored_data(&self) -> Vec<StoredItem> {
            self.stored_data.read().unwrap().clone()
        }

        fn get_batch_items(&self) -> Vec<(Vec<u8>, Vec<Tag>)> {
            self.batch_items.read().unwrap().clone()
        }

        fn get_kfrag_calls(&self) -> Vec<KFragCall> {
            self.kfrag_calls.read().unwrap().clone()
        }

        fn set_query_results(&self, results: Vec<ArweaveTransaction>) {
            *self.query_results.write().unwrap() = results;
        }

        fn set_should_fail_kfrag(&self, should_fail: bool) {
            *self.should_fail_kfrag.write().unwrap() = should_fail;
        }

        fn set_should_fail_store(&self, should_fail: bool) {
            *self.should_fail_store.write().unwrap() = should_fail;
        }

        fn set_should_fail_batch_partial(&self, should_fail: bool) {
            *self.should_fail_batch_partial.write().unwrap() = should_fail;
        }

        fn build_partial_failure_result(&self, item_count: usize) -> BatchResult {
            let mut successful = Vec::new();
            let mut failed = Vec::new();
            for index in 0..item_count {
                if index % 2 == 1 {
                    failed.push((index.to_string(), "Mock partial failure".to_string()));
                } else {
                    successful.push(self.generate_tx_id());
                }
            }
            BatchResult { successful, failed }
        }

        fn generate_tx_id(&self) -> String {
            let mut counter = self.tx_counter.write().unwrap();
            *counter += 1;
            format!("mock_tx_{:016x}", *counter)
        }
    }

    #[async_trait]
    impl StorageService for MockStorageService {
        async fn store_data(&self, data: &[u8], tags: Vec<Tag>) -> ServiceResult<String> {
            if *self.should_fail_store.read().unwrap() {
                return Err(crate::service::error::ServiceError::storage_error(
                    "Mock storage failure",
                ));
            }

            let tx_id = self.generate_tx_id();
            self.stored_data.write().unwrap().push(StoredItem {
                data: data.to_vec(),
                tags,
                tx_id: tx_id.clone(),
            });
            Ok(tx_id)
        }

        async fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction> {
            let stored = self.stored_data.read().unwrap();
            stored
                .iter()
                .find(|item| item.tx_id == transaction_id)
                .map(|item| ArweaveTransaction {
                    id: item.tx_id.clone(),
                    data: item.data.clone(),
                    tags: item.tags.clone(),
                    timestamp: 0,
                })
                .ok_or_else(|| {
                    crate::service::error::ServiceError::not_found(format!(
                        "Transaction {} not found",
                        transaction_id
                    ))
                })
        }

        async fn query_by_tags(
            &self,
            _params: QueryParams,
        ) -> ServiceResult<Vec<ArweaveTransaction>> {
            Ok(self.query_results.read().unwrap().clone())
        }

        async fn check_transaction_status(
            &self,
            _transaction_id: &str,
        ) -> ServiceResult<TransactionStatus> {
            Ok(TransactionStatus::Confirmed)
        }

        async fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult> {
            if *self.should_fail_store.read().unwrap() {
                return Err(crate::service::error::ServiceError::storage_error(
                    "Mock batch storage failure",
                ));
            }

            self.batch_items.write().unwrap().extend(items.clone());

            if *self.should_fail_batch_partial.read().unwrap() {
                return Ok(self.build_partial_failure_result(items.len()));
            }

            let successful: Vec<String> = items.iter().map(|_| self.generate_tx_id()).collect();
            Ok(BatchResult {
                successful,
                failed: Vec::new(),
            })
        }

        async fn exists(&self, transaction_id: &str) -> ServiceResult<bool> {
            let stored = self.stored_data.read().unwrap();
            Ok(stored.iter().any(|item| item.tx_id == transaction_id))
        }

        async fn update_tags(
            &self,
            transaction_id: &str,
            new_tags: Vec<Tag>,
        ) -> ServiceResult<String> {
            let data = self.retrieve_data(transaction_id).await?.data;
            self.store_data(&data, new_tags).await
        }

        async fn send_kfrags_to_contract(
            &self,
            kfrags: &[KeyFragment],
            contract_id: &str,
            _secret_id: &str,
        ) -> ServiceResult<()> {
            if *self.should_fail_kfrag.read().unwrap() {
                return Err(crate::service::error::ServiceError::ao_network_error(
                    "Mock kFrag send failure",
                ));
            }

            self.kfrag_calls.write().unwrap().push(KFragCall {
                kfrag_count: kfrags.len(),
                owner_process_id: contract_id.to_string(),
            });
            Ok(())
        }

        async fn delegate_capsule(
            &self,
            _capsule_data: &[u8],
            _contract_id: &str,
        ) -> ServiceResult<()> {
            Err(crate::service::error::ServiceError::System(
                crate::service::error::SystemException::Internal("Not implemented".to_string()),
            ))
        }

        async fn retrieve_cfrags(
            &self,
            _secret_id: &str,
            _total_shares: u8,
            _capsule_id: &str,
            _process_id: &str,
        ) -> ServiceResult<Vec<crate::usecase::core::crypto::CFragData>> {
            Err(crate::service::error::ServiceError::System(
                crate::service::error::SystemException::Internal("Not implemented".to_string()),
            ))
        }

        async fn retrieve_encrypted_shares(&self, _secret_id: &str) -> ServiceResult<Vec<Vec<u8>>> {
            Err(crate::service::error::ServiceError::System(
                crate::service::error::SystemException::Internal("Not implemented".to_string()),
            ))
        }
    }

    // ========================================================================
    // Test Helper Functions
    // ========================================================================

    fn create_test_service()
    -> SecretSharingWorkflowServiceImpl<TestCryptoService, TestStorageService> {
        let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
        let crypto = Arc::new(ServiceCryptoServiceImpl::new(Arc::clone(&core_crypto)));
        let mock_ao = Arc::new(MockAOClient::new());
        let arweave = Arc::new(ArweaveStorageServiceImpl::default());
        let contract = Arc::new(ContractStorageImpl::new(mock_ao));
        let storage = Arc::new(ServiceStorageServiceImpl::new(arweave, contract));
        SecretSharingWorkflowServiceImpl::new(crypto, storage)
    }

    fn create_mock_service() -> (
        SecretSharingWorkflowServiceImpl<TestCryptoService, MockStorageService>,
        Arc<MockStorageService>,
    ) {
        let core_crypto = Arc::new(CoreCryptoServiceImpl::new());
        let crypto = Arc::new(ServiceCryptoServiceImpl::new(core_crypto));
        let storage = Arc::new(MockStorageService::new());
        let service = SecretSharingWorkflowServiceImpl::new(crypto, storage.clone());
        (service, storage)
    }

    fn create_test_request(crypto: &CoreCryptoServiceImpl) -> SecretSharingRequest {
        let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
        let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();

        SecretSharingRequest {
            secret: Zeroizing::new(b"Test secret data for sharing".to_vec()),
            owner_secret_key: owner_sk,
            owner_public_key: owner_pk,
            requester_public_key: requester_pk,
            threshold: 3,
            total_shares: 5,
            owner_process_id: "owner-process-123".to_string(),
            metadata: None,
        }
    }

    // ========================================================================
    // Validation Tests (Task 14)
    // ========================================================================

    #[test]
    fn test_validate_request_valid() {
        println!("\n=== test_validate_request_valid ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!(
            "  Created request: threshold={}, total_shares={}, secret_len={}",
            request.threshold,
            request.total_shares,
            request.secret.len()
        );

        let result = service.validate_request(&request);
        println!("  Validation result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Valid request accepted");
    }

    #[test]
    fn test_phase1_invalid_threshold_below_min() {
        println!("\n=== test_phase1_invalid_threshold_below_min ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = 1; // Below MIN_THRESHOLD
        println!("  Testing threshold=1 (below MIN_THRESHOLD=2)");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("at least"));
            println!("  [PASS] Correctly rejected with message: {}", msg);
        }
    }

    #[test]
    fn test_phase1_invalid_threshold_exceeds_total() {
        println!("\n=== test_phase1_invalid_threshold_exceeds_total ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = 6;
        request.total_shares = 5;
        println!("  Testing threshold=6 > total_shares=5");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("cannot exceed"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase1_invalid_total_shares_exceeds_max() {
        println!("\n=== test_phase1_invalid_total_shares_exceeds_max ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.total_shares = 255; // Exceeds MAX_SHARES
        request.threshold = 3;
        println!("  Testing total_shares=255 (exceeds MAX)");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("cannot exceed") || msg.contains("255"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase1_empty_secret() {
        println!("\n=== test_phase1_empty_secret ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = Zeroizing::new(vec![]);
        println!("  Testing empty secret");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("empty"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase1_invalid_owner_public_key() {
        println!("\n=== test_phase1_invalid_owner_public_key ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.owner_public_key = PublicKey { key_data: vec![] };
        println!("  Testing empty owner public key");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("Owner public key"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase1_invalid_requester_public_key() {
        println!("\n=== test_phase1_invalid_requester_public_key ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.requester_public_key = PublicKey { key_data: vec![] };
        println!("  Testing empty requester public key");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("Requester public key"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    #[test]
    fn test_phase1_empty_owner_process_id() {
        println!("\n=== test_phase1_empty_owner_process_id ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.owner_process_id = String::new();
        println!("  Testing empty owner process ID");

        let result = service.validate_request(&request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("process ID"));
            println!("  [PASS] Correctly rejected: {}", msg);
        }
    }

    // ========================================================================
    // Workflow Tests (Task 15)
    // ========================================================================

    #[test]
    fn test_phase1_generates_symmetric_key() {
        println!("\n=== test_phase1_generates_symmetric_key ===");
        let service = create_test_service();

        println!("  Generating symmetric key...");
        let result = service.generate_symmetric_key();
        assert!(result.is_ok());

        let key = result.unwrap();
        println!("  Key length: {} bytes", key.len());
        assert_eq!(key.len(), 32); // AES-256 key
        println!("  [PASS] Generated valid AES-256 key (32 bytes)");
    }

    #[test]
    fn test_phase1_generates_symmetric_key_randomness() {
        println!("\n=== test_phase1_generates_symmetric_key_randomness ===");
        let service = create_test_service();

        println!("  Generating two keys to verify randomness...");
        let key1 = service.generate_symmetric_key().unwrap();
        let key2 = service.generate_symmetric_key().unwrap();

        println!("  Key1: {:02x?}...", &key1[..8]);
        println!("  Key2: {:02x?}...", &key2[..8]);
        // Keys should be different (random)
        assert_ne!(key1, key2);
        println!("  [PASS] Keys are different (cryptographically random)");
    }

    #[test]
    fn test_phase1_splits_secret_shamir() {
        println!("\n=== test_phase1_splits_secret_shamir ===");
        let service = create_test_service();
        let secret = b"Test secret data";
        println!(
            "  Secret: {:?} ({} bytes)",
            String::from_utf8_lossy(secret),
            secret.len()
        );

        println!("  Splitting with threshold=3, total_shares=5...");
        let result = service.split_secret(secret, 3, 5);
        assert!(result.is_ok());

        let shares = result.unwrap();
        println!("  Generated {} shares", shares.len());
        assert_eq!(shares.len(), 5);

        // Each share should have a unique index
        let indices: Vec<u8> = shares.iter().map(|s| s.index).collect();
        println!("  Share indices: {:?}", indices);
        let mut unique_indices = indices.clone();
        unique_indices.sort_unstable();
        unique_indices.dedup();
        assert_eq!(indices.len(), unique_indices.len());
        println!("  [PASS] All shares have unique indices");
    }

    #[test]
    fn test_phase1_encrypts_shares_aes_gcm() {
        println!("\n=== test_phase1_encrypts_shares_aes_gcm ===");
        let service = create_test_service();
        let symmetric_key = service.generate_symmetric_key().unwrap();
        println!("  Generated symmetric key");

        let shares = vec![
            ShamirShare {
                index: 1,
                share_data: b"share data 1".to_vec(),
            },
            ShamirShare {
                index: 2,
                share_data: b"share data 2".to_vec(),
            },
        ];
        println!("  Encrypting {} shares with AES-GCM...", shares.len());

        let result = service.encrypt_shares(&shares, &symmetric_key);
        assert!(result.is_ok());

        let encrypted = result.unwrap();
        assert_eq!(encrypted.len(), 2);

        // Encrypted data should be larger than original (includes nonce + tag)
        for (i, enc) in encrypted.iter().enumerate() {
            println!(
                "  Share {}: {} bytes -> {} bytes (encrypted)",
                i,
                shares[i].share_data.len(),
                enc.len()
            );
            assert!(enc.len() > shares[i].share_data.len());
        }
        println!("  [PASS] All shares encrypted successfully");
    }

    #[test]
    fn test_phase1_creates_capsule_for_symmetric_key() {
        println!("\n=== test_phase1_creates_capsule_for_symmetric_key ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let (_owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
        let symmetric_key = service.generate_symmetric_key().unwrap();
        println!("  Generated owner keypair and symmetric key");

        println!("  Creating PRE capsule...");
        let result = service.create_capsule(&owner_pk, &symmetric_key);
        assert!(result.is_ok());

        let (capsule, ciphertext) = result.unwrap();
        println!("  Capsule size: {} bytes", capsule.capsule_bytes.len());
        println!("  Ciphertext size: {} bytes", ciphertext.len());
        assert!(!capsule.capsule_bytes.is_empty());
        assert!(!ciphertext.is_empty());
        println!("  [PASS] PRE capsule created successfully");
    }

    #[test]
    fn test_phase1_generates_kfrags() {
        println!("\n=== test_phase1_generates_kfrags ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!(
            "  threshold={}, total_shares={}",
            request.threshold, request.total_shares
        );

        println!("  Generating kFrags...");
        let result = service.generate_kfrags(&request);
        assert!(result.is_ok());

        let kfrags = result.unwrap();
        println!("  Generated {} kFrags", kfrags.len());
        assert_eq!(kfrags.len(), 5); // total_shares

        // Each kFrag should have data
        for (i, kfrag) in kfrags.iter().enumerate() {
            println!("  kFrag[{}]: {} bytes", i, kfrag.key_data.len());
            assert!(!kfrag.key_data.is_empty());
        }
        println!("  [PASS] All kFrags generated with valid data");
    }

    #[test]
    fn test_phase1_kfrags_count_matches_n() {
        println!("\n=== test_phase1_kfrags_count_matches_n ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        // Test with different n values
        for n in [3, 5, 7, 10] {
            let mut request = create_test_request(&crypto);
            request.total_shares = n;
            request.threshold = 2;

            println!("  Testing n={}: generating kFrags...", n);
            let kfrags = service.generate_kfrags(&request).unwrap();
            println!("  Generated {} kFrags for n={}", kfrags.len(), n);
            assert_eq!(
                kfrags.len() as u8,
                n,
                "kFrag count should match total_shares"
            );
        }
        println!("  [PASS] kFrag count matches n for all test values");
    }

    #[tokio::test]
    async fn test_phase1_returns_complete_result() {
        println!("\n=== test_phase1_returns_complete_result ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        let owner_pk = request.owner_public_key.clone();
        println!("  Executing complete PHASE 1 workflow...");

        let result = service.execute_secret_sharing(request).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        println!("  Result:");
        println!("    secret_id: {}", result.secret_id);
        println!("    capsule_tx_id: {}", result.capsule_tx_id);
        println!("    share_tx_ids: {} items", result.share_tx_ids.len());
        println!("    kfrag_count: {}", result.kfrag_count);

        // Verify all fields are populated
        assert!(!result.secret_id.as_str().is_empty());
        assert!(!result.capsule_tx_id.is_empty());
        assert_eq!(result.share_tx_ids.len(), 5);
        assert_eq!(result.kfrag_count, 5);
        assert_eq!(result.owner_public_key.key_data, owner_pk.key_data);
        println!("  [PASS] Complete result returned with all fields populated");
    }

    #[tokio::test]
    async fn test_phase1_complete_flow() {
        println!("\n=== test_phase1_complete_flow ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let original_secret = b"This is a very important secret message!".to_vec();
        println!(
            "  Original secret: {:?}",
            String::from_utf8_lossy(&original_secret)
        );

        let mut request = create_test_request(&crypto);
        request.secret = Zeroizing::new(original_secret.clone());
        request.threshold = 3;
        request.total_shares = 5;
        println!("  Parameters: threshold=3, total_shares=5");

        println!("  Executing PHASE 1 workflow...");
        let result = service.execute_secret_sharing(request).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        println!("  Result:");
        println!("    kfrag_count: {}", result.kfrag_count);
        println!("    share_tx_ids: {:?}", result.share_tx_ids);
        println!("    capsule_tx_id: {}", result.capsule_tx_id);
        assert_eq!(result.kfrag_count, 5);
        assert_eq!(result.share_tx_ids.len(), 5);

        // Verify transaction IDs have correct format
        for tx_id in &result.share_tx_ids {
            assert!(tx_id.starts_with("tx_"));
        }
        assert!(result.capsule_tx_id.starts_with("tx_"));
        println!("  [PASS] Complete PHASE 1 flow executed successfully");
    }

    #[tokio::test]
    async fn test_phase1_minimum_threshold() {
        println!("\n=== test_phase1_minimum_threshold ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = constants::MIN_THRESHOLD;
        request.total_shares = constants::MIN_THRESHOLD;
        println!(
            "  Testing minimum threshold: k=n={}",
            constants::MIN_THRESHOLD
        );

        let result = service.execute_secret_sharing(request).await;
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Minimum threshold accepted");
    }

    #[tokio::test]
    async fn test_phase1_max_secret_size() {
        println!("\n=== test_phase1_max_secret_size ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);

        // Test with max supported secret size (63 bytes - DATA_SIZE is 64, 1 byte for length prefix)
        request.secret = Zeroizing::new(vec![0xAB; 63]);
        println!("  Testing max secret size: {} bytes", request.secret.len());

        let result = service.execute_secret_sharing(request).await;
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Max secret size (63 bytes) accepted");
    }

    // ========================================================================
    // Error Handling & Status Tests (Task 16)
    // ========================================================================

    #[tokio::test]
    async fn test_execute_secret_sharing_validation_fails() {
        println!("\n=== test_execute_secret_sharing_validation_fails ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = Zeroizing::new(vec![]); // Invalid
        println!("  Testing with empty secret (should fail validation)");

        let result = service.execute_secret_sharing(request).await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
        println!("  [PASS] Validation correctly failed for empty secret");
    }

    #[tokio::test]
    async fn test_get_secret_status_not_implemented() {
        println!("\n=== test_get_secret_status_not_implemented ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Checking status for secret_id: {}", secret_id);

        let result = service.get_secret_status(&secret_id).await;
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        if let Err(WorkflowError::ResourceNotFound(msg)) = result {
            assert!(msg.contains("not found") || msg.contains("not yet implemented"));
            println!(
                "  [PASS] ResourceNotFound returned (storage not implemented): {}",
                msg
            );
        }
    }

    #[tokio::test]
    async fn test_execute_with_different_threshold_total_combinations() {
        println!("\n=== test_execute_with_different_threshold_total_combinations ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        // Test various valid combinations
        let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

        for (threshold, total) in combinations {
            println!("  Testing (k={}, n={})...", threshold, total);
            let mut request = create_test_request(&crypto);
            request.threshold = threshold;
            request.total_shares = total;

            let result = service.execute_secret_sharing(request).await;
            assert!(
                result.is_ok(),
                "Failed for threshold={threshold}, total={total}"
            );

            let result = result.unwrap();
            println!(
                "    kfrag_count={}, share_tx_ids.len()={}",
                result.kfrag_count,
                result.share_tx_ids.len()
            );
            assert_eq!(result.kfrag_count, total);
            assert_eq!(result.share_tx_ids.len(), total as usize);
        }
        println!("  [PASS] All threshold/total combinations work correctly");
    }

    #[tokio::test]
    async fn test_result_secret_id_is_unique() {
        println!("\n=== test_result_secret_id_is_unique ===");
        let service = create_test_service();
        let crypto = CoreCryptoServiceImpl::new();

        let request1 = create_test_request(&crypto);
        let request2 = create_test_request(&crypto);

        println!("  Executing two workflows to verify unique IDs...");
        let result1 = service.execute_secret_sharing(request1).await.unwrap();
        let result2 = service.execute_secret_sharing(request2).await.unwrap();

        println!("  secret_id_1: {}", result1.secret_id);
        println!("  secret_id_2: {}", result2.secret_id);

        // Each execution should generate a unique secret ID
        assert_ne!(result1.secret_id, result2.secret_id);
        println!("  [PASS] Each execution generates unique secret_id");
    }

    // ========================================================================
    // Partial Batch Failure Tests
    // ========================================================================

    #[tokio::test]
    async fn test_execute_secret_sharing_partial_batch_failure() {
        println!("\n=== test_execute_secret_sharing_partial_batch_failure ===");
        let (service, mock_storage) = create_mock_service();
        let crypto = CoreCryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        mock_storage.set_should_fail_batch_partial(true);
        println!("  Executing with partial batch failure enabled...");

        let result = service.execute_secret_sharing(request).await;
        assert!(result.is_err());

        let err = result.unwrap_err();
        println!("  Error: {:?}", err);

        match err {
            WorkflowError::PartialStorageFailure {
                ref capsule_tx_id,
                ref successful_share_tx_ids,
                ref failed_shares,
                failed_count,
                total_count,
            } => {
                assert!(!capsule_tx_id.is_empty(), "capsule_tx_id should be present");
                assert!(
                    !successful_share_tx_ids.is_empty(),
                    "some shares should have succeeded"
                );
                assert!(!failed_shares.is_empty(), "some shares should have failed");
                assert_eq!(failed_count, failed_shares.len());
                assert_eq!(
                    total_count,
                    successful_share_tx_ids.len() + failed_shares.len()
                );

                for (index_str, error_msg) in failed_shares {
                    assert!(
                        !index_str.is_empty(),
                        "failed share index should not be empty"
                    );
                    let _index: usize = index_str.parse().expect("index should be a valid number");
                    assert!(
                        !error_msg.is_empty(),
                        "failed share error message should not be empty"
                    );
                }
                println!("  capsule_tx_id: {}", capsule_tx_id);
                println!(
                    "  successful_share_tx_ids: {} items",
                    successful_share_tx_ids.len()
                );
                println!("  failed_shares: {} items", failed_shares.len());
                println!("  [PASS] PartialStorageFailure contains all expected info");
            }
            other => panic!("Expected PartialStorageFailure, got: {:?}", other),
        }

        assert!(!err.is_recoverable());
        println!("  [PASS] PartialStorageFailure is not recoverable");
    }
}
