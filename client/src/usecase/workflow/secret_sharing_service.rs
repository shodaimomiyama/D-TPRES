//! SecretSharingWorkflowService - Phase 1 Secret Sharing Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 1 workflow: secret splitting, encryption, kFrag generation, and storage.

use std::sync::Arc;

use zeroize::Zeroizing;

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

impl<C: CryptoService> SecretSharingWorkflowService for SecretSharingWorkflowServiceImpl<C> {
    fn execute_secret_sharing(
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
        println!("\n=== test_validate_request_valid ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = vec![];
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();

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

    #[test]
    fn test_phase1_returns_complete_result() {
        println!("\n=== test_phase1_returns_complete_result ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        let owner_pk = request.owner_public_key.clone();
        println!("  Executing complete PHASE 1 workflow...");

        let result = service.execute_secret_sharing(request);
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

    #[test]
    fn test_phase1_complete_flow() {
        println!("\n=== test_phase1_complete_flow ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let original_secret = b"This is a very important secret message!".to_vec();
        println!(
            "  Original secret: {:?}",
            String::from_utf8_lossy(&original_secret)
        );

        let mut request = create_test_request(&crypto);
        request.secret = original_secret.clone();
        request.threshold = 3;
        request.total_shares = 5;
        println!("  Parameters: threshold=3, total_shares=5");

        println!("  Executing PHASE 1 workflow...");
        let result = service.execute_secret_sharing(request);
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
            assert!(tx_id.contains("share_tx_"));
        }
        assert!(result.capsule_tx_id.contains("capsule_tx_"));
        println!("  [PASS] Complete PHASE 1 flow executed successfully");
    }

    #[test]
    fn test_phase1_minimum_threshold() {
        println!("\n=== test_phase1_minimum_threshold ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.threshold = constants::MIN_THRESHOLD;
        request.total_shares = constants::MIN_THRESHOLD;
        println!(
            "  Testing minimum threshold: k=n={}",
            constants::MIN_THRESHOLD
        );

        let result = service.execute_secret_sharing(request);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Minimum threshold accepted");
    }

    #[test]
    fn test_phase1_max_secret_size() {
        println!("\n=== test_phase1_max_secret_size ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);

        // Test with max supported secret size (63 bytes - DATA_SIZE is 64, 1 byte for length prefix)
        request.secret = vec![0xAB; 63];
        println!("  Testing max secret size: {} bytes", request.secret.len());

        let result = service.execute_secret_sharing(request);
        println!("  Result: {:?}", result.is_ok());
        assert!(result.is_ok());
        println!("  [PASS] Max secret size (63 bytes) accepted");
    }

    // ========================================================================
    // Error Handling & Status Tests (Task 16)
    // ========================================================================

    #[test]
    fn test_execute_secret_sharing_validation_fails() {
        println!("\n=== test_execute_secret_sharing_validation_fails ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.secret = vec![]; // Invalid
        println!("  Testing with empty secret (should fail validation)");

        let result = service.execute_secret_sharing(request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));
        println!("  [PASS] Validation correctly failed for empty secret");
    }

    #[test]
    fn test_get_secret_status_not_implemented() {
        println!("\n=== test_get_secret_status_not_implemented ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Checking status for secret_id: {}", secret_id);

        let result = service.get_secret_status(&secret_id);
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

    #[test]
    fn test_execute_with_different_threshold_total_combinations() {
        println!("\n=== test_execute_with_different_threshold_total_combinations ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        // Test various valid combinations
        let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

        for (threshold, total) in combinations {
            println!("  Testing (k={}, n={})...", threshold, total);
            let mut request = create_test_request(&crypto);
            request.threshold = threshold;
            request.total_shares = total;

            let result = service.execute_secret_sharing(request);
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

    #[test]
    fn test_result_secret_id_is_unique() {
        println!("\n=== test_result_secret_id_is_unique ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        let request1 = create_test_request(&crypto);
        let request2 = create_test_request(&crypto);

        println!("  Executing two workflows to verify unique IDs...");
        let result1 = service.execute_secret_sharing(request1).unwrap();
        let result2 = service.execute_secret_sharing(request2).unwrap();

        println!("  secret_id_1: {}", result1.secret_id);
        println!("  secret_id_2: {}", result2.secret_id);

        // Each execution should generate a unique secret ID
        assert_ne!(result1.secret_id, result2.secret_id);
        println!("  [PASS] Each execution generates unique secret_id");
    }
}
