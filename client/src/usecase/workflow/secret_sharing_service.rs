//! SecretSharingWorkflowService - Phase 1 Secret Sharing Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 1 workflow: secret splitting, encryption, kFrag generation, and storage.

use std::sync::Arc;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{CryptoService, KeyFragment, PublicKey, ShamirShare, constants};
use crate::usecase::dto::{SecretSharingRequest, SecretSharingResult, SecretStatus};
use crate::usecase::error::{WorkflowError, WorkflowResult};

// ============================================================================
// SecretSharingWorkflowService Trait
// ============================================================================

/// SecretSharingWorkflowService trait - Phase 1 workflow interface
///
/// Defines the contract for secret sharing operations that orchestrate
/// CryptoService and StorageService to split, encrypt, and distribute secrets.
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
    fn execute_secret_sharing(
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
    fn get_secret_status(&self, secret_id: &SecretId) -> WorkflowResult<SecretStatus>;
}

// ============================================================================
// SecretSharingWorkflowServiceImpl
// ============================================================================

/// Implementation of SecretSharingWorkflowService
///
/// Orchestrates CryptoService and StorageService to implement
/// the complete Phase 1 secret sharing workflow.
pub struct SecretSharingWorkflowServiceImpl<C: CryptoService> {
    crypto_service: Arc<C>,
    // TODO: Add StorageService when AO communication is implemented (Issue #47)
    // storage_service: Arc<S>,
}

impl<C: CryptoService> SecretSharingWorkflowServiceImpl<C> {
    /// Create a new SecretSharingWorkflowServiceImpl
    pub fn new(crypto_service: Arc<C>) -> Self {
        Self { crypto_service }
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
                    .aes_gcm_encrypt(symmetric_key, &share.data)
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

impl<C: CryptoService> SecretSharingWorkflowService for SecretSharingWorkflowServiceImpl<C> {
    fn execute_secret_sharing(
        &self,
        request: SecretSharingRequest,
    ) -> WorkflowResult<SecretSharingResult> {
        // Step 0: Validate input parameters
        self.validate_request(&request)?;

        // Step 1: Generate symmetric key kₒ
        let symmetric_key = self.generate_symmetric_key()?;

        // Step 2: Split secret using Shamir Secret Sharing
        let shares = self.split_secret(&request.secret, request.threshold, request.total_shares)?;

        // Step 3: Encrypt each share with AES-GCM using kₒ
        let encrypted_shares = self.encrypt_shares(&shares, &symmetric_key)?;

        // Step 4: Create PRE Capsule for kₒ
        let (capsule, _ciphertext) =
            self.create_capsule(&request.owner_public_key, &symmetric_key)?;

        // Step 5-6: Generate re-encryption key and create kFrags
        let kfrags = self.generate_kfrags(&request)?;
        let kfrag_count = kfrags.len() as u8;

        // Step 7: Send kFrags to Owner-Process via AO
        // TODO: Implement when StorageService AO methods are available (Issue #47)
        // self.storage_service.send_kfrag_to_owner_process(&kfrags, &request.owner_process_id)?;

        // Step 8: Store Capsule and encrypted shares on Arweave
        // TODO: Implement when StorageService is available (Issue #47)
        // For now, generate placeholder transaction IDs
        let secret_id = SecretId::generate();
        let capsule_tx_id = format!("capsule_tx_{}", secret_id.as_str());
        let share_tx_ids: Vec<String> = encrypted_shares
            .iter()
            .enumerate()
            .map(|(i, _)| format!("share_tx_{}_{}", secret_id.as_str(), i))
            .collect();

        // Suppress warnings for now - will be used when storage is implemented
        let _ = capsule;
        let _ = kfrags;

        Ok(SecretSharingResult {
            secret_id,
            capsule_tx_id,
            share_tx_ids,
            kfrag_count,
            owner_public_key: request.owner_public_key.clone(),
        })
    }

    fn get_secret_status(&self, secret_id: &SecretId) -> WorkflowResult<SecretStatus> {
        // TODO: Implement when StorageService query methods are available (Issue #47)
        // For now, return ResourceNotFound as placeholder
        Err(WorkflowError::not_found(format!(
            "Secret {} not found (storage not yet implemented)",
            secret_id
        )))
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::redundant_clone)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::CryptoServiceImpl;

    fn create_test_service() -> SecretSharingWorkflowServiceImpl<CryptoServiceImpl> {
        let crypto = Arc::new(CryptoServiceImpl::new());
        SecretSharingWorkflowServiceImpl::new(crypto)
    }

    fn create_test_request(crypto: &CryptoServiceImpl) -> SecretSharingRequest {
        let (owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
        let (_requester_sk, requester_pk) = crypto.generate_keypair().unwrap();

        SecretSharingRequest {
            secret: b"Test secret data for sharing".to_vec(),
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
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        let result = service.validate_request(&request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_phase1_invalid_threshold_below_min() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = 1; // Below MIN_THRESHOLD

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("at least"));
        }
    }

    #[test]
    fn test_phase1_invalid_threshold_exceeds_total() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = 6;
        request.total_shares = 5;

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("cannot exceed"));
        }
    }

    #[test]
    fn test_phase1_invalid_total_shares_exceeds_max() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.total_shares = 255; // Exceeds MAX_SHARES
        request.threshold = 3;

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("cannot exceed") || msg.contains("255"));
        }
    }

    #[test]
    fn test_phase1_empty_secret() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = vec![];

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("empty"));
        }
    }

    #[test]
    fn test_phase1_invalid_owner_public_key() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.owner_public_key = PublicKey { key_data: vec![] };

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("Owner public key"));
        }
    }

    #[test]
    fn test_phase1_invalid_requester_public_key() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.requester_public_key = PublicKey { key_data: vec![] };

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("Requester public key"));
        }
    }

    #[test]
    fn test_phase1_empty_owner_process_id() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.owner_process_id = String::new();

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("process ID"));
        }
    }

    // ========================================================================
    // Workflow Tests (Task 15)
    // ========================================================================

    #[test]
    fn test_phase1_generates_symmetric_key() {
        let service = create_test_service();

        let result = service.generate_symmetric_key();
        assert!(result.is_ok());

        let key = result.unwrap();
        assert_eq!(key.len(), 32); // AES-256 key
    }

    #[test]
    fn test_phase1_generates_symmetric_key_randomness() {
        let service = create_test_service();

        let key1 = service.generate_symmetric_key().unwrap();
        let key2 = service.generate_symmetric_key().unwrap();

        // Keys should be different (random)
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_phase1_splits_secret_shamir() {
        let service = create_test_service();
        let secret = b"Test secret data";

        let result = service.split_secret(secret, 3, 5);
        assert!(result.is_ok());

        let shares = result.unwrap();
        assert_eq!(shares.len(), 5);

        // Each share should have a unique index
        let indices: Vec<u8> = shares.iter().map(|s| s.index).collect();
        let mut unique_indices = indices.clone();
        unique_indices.sort_unstable();
        unique_indices.dedup();
        assert_eq!(indices.len(), unique_indices.len());
    }

    #[test]
    fn test_phase1_encrypts_shares_aes_gcm() {
        let service = create_test_service();
        let symmetric_key = service.generate_symmetric_key().unwrap();

        let shares = vec![
            ShamirShare {
                index: 1,
                data: b"share data 1".to_vec(),
            },
            ShamirShare {
                index: 2,
                data: b"share data 2".to_vec(),
            },
        ];

        let result = service.encrypt_shares(&shares, &symmetric_key);
        assert!(result.is_ok());

        let encrypted = result.unwrap();
        assert_eq!(encrypted.len(), 2);

        // Encrypted data should be larger than original (includes nonce + tag)
        for (i, enc) in encrypted.iter().enumerate() {
            assert!(enc.len() > shares[i].data.len());
        }
    }

    #[test]
    fn test_phase1_creates_capsule_for_symmetric_key() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let (_owner_sk, owner_pk) = crypto.generate_keypair().unwrap();
        let symmetric_key = service.generate_symmetric_key().unwrap();

        let result = service.create_capsule(&owner_pk, &symmetric_key);
        assert!(result.is_ok());

        let (capsule, ciphertext) = result.unwrap();
        assert!(!capsule.data.is_empty());
        assert!(!ciphertext.is_empty());
    }

    #[test]
    fn test_phase1_generates_kfrags() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        let result = service.generate_kfrags(&request);
        assert!(result.is_ok());

        let kfrags = result.unwrap();
        assert_eq!(kfrags.len(), 5); // total_shares

        // Each kFrag should have data
        for kfrag in &kfrags {
            assert!(!kfrag.key_data.is_empty());
        }
    }

    #[test]
    fn test_phase1_kfrags_count_matches_n() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        // Test with different n values
        for n in [3, 5, 7, 10] {
            let mut request = create_test_request(&crypto);
            request.total_shares = n;
            request.threshold = 2;

            let kfrags = service.generate_kfrags(&request).unwrap();
            assert_eq!(
                kfrags.len() as u8,
                n,
                "kFrag count should match total_shares"
            );
        }
    }

    #[test]
    fn test_phase1_returns_complete_result() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        let owner_pk = request.owner_public_key.clone();

        let result = service.execute_secret_sharing(request);
        assert!(result.is_ok());

        let result = result.unwrap();

        // Verify all fields are populated
        assert!(!result.secret_id.as_str().is_empty());
        assert!(!result.capsule_tx_id.is_empty());
        assert_eq!(result.share_tx_ids.len(), 5);
        assert_eq!(result.kfrag_count, 5);
        assert_eq!(result.owner_public_key.key_data, owner_pk.key_data);
    }

    #[test]
    fn test_phase1_complete_flow() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let original_secret = b"This is a very important secret message!".to_vec();

        let mut request = create_test_request(&crypto);
        request.secret = original_secret.clone();
        request.threshold = 3;
        request.total_shares = 5;

        let result = service.execute_secret_sharing(request);
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.kfrag_count, 5);
        assert_eq!(result.share_tx_ids.len(), 5);

        // Verify transaction IDs have correct format
        for tx_id in &result.share_tx_ids {
            assert!(tx_id.contains("share_tx_"));
        }
        assert!(result.capsule_tx_id.contains("capsule_tx_"));
    }

    #[test]
    fn test_phase1_minimum_threshold() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = constants::MIN_THRESHOLD;
        request.total_shares = constants::MIN_THRESHOLD;

        let result = service.execute_secret_sharing(request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_phase1_max_secret_size() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);

        // Test with max supported secret size (63 bytes - DATA_SIZE is 64, 1 byte for length prefix)
        request.secret = vec![0xAB; 63];

        let result = service.execute_secret_sharing(request);
        assert!(result.is_ok());
    }

    // ========================================================================
    // Error Handling & Status Tests (Task 16)
    // ========================================================================

    #[test]
    fn test_execute_secret_sharing_validation_fails() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = vec![]; // Invalid

        let result = service.execute_secret_sharing(request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
    }

    #[test]
    fn test_get_secret_status_not_implemented() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        let result = service.get_secret_status(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        if let Err(WorkflowError::ResourceNotFound(msg)) = result {
            assert!(msg.contains("not found") || msg.contains("not yet implemented"));
        }
    }

    #[test]
    fn test_execute_with_different_threshold_total_combinations() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        // Test various valid combinations
        let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

        for (threshold, total) in combinations {
            let mut request = create_test_request(&crypto);
            request.threshold = threshold;
            request.total_shares = total;

            let result = service.execute_secret_sharing(request);
            assert!(result.is_ok(), "Failed for threshold={threshold}, total={total}");

            let result = result.unwrap();
            assert_eq!(result.kfrag_count, total);
            assert_eq!(result.share_tx_ids.len(), total as usize);
        }
    }

    #[test]
    fn test_result_secret_id_is_unique() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        let request1 = create_test_request(&crypto);
        let request2 = create_test_request(&crypto);

        let result1 = service.execute_secret_sharing(request1).unwrap();
        let result2 = service.execute_secret_sharing(request2).unwrap();

        // Each execution should generate a unique secret ID
        assert_ne!(result1.secret_id, result2.secret_id);
    }
}
