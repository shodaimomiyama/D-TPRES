//! SecretRecoveryWorkflowService - Phase 3 Secret Recovery Workflow
//!
//! Orchestrates CryptoService and StorageService to implement the complete
//! PHASE 3 workflow: cFrag/Capsule retrieval, PRE decryption, and secret reconstruction.

use std::sync::Arc;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{CFragData, Capsule, CryptoService, SecretKey, ShamirShare};
use crate::usecase::dto::{SecretRecoveryRequest, SecretRecoveryResult};
use crate::usecase::error::{WorkflowError, WorkflowResult};

// ============================================================================
// SecretRecoveryWorkflowService Trait
// ============================================================================

/// SecretRecoveryWorkflowService trait - Phase 3 workflow interface
///
/// Defines the contract for secret recovery operations that orchestrate
/// CryptoService and StorageService to retrieve, decrypt, and reconstruct secrets.
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
    ///
    /// # Arguments
    /// * `request` - SecretRecoveryRequest containing secret_id and requester credentials
    ///
    /// # Returns
    /// * `WorkflowResult<SecretRecoveryResult>` - Result containing recovered secret
    ///
    /// # Errors
    /// * `WorkflowError::ValidationError` - Invalid input parameters
    /// * `WorkflowError::ResourceNotFound` - Secret or cFrags not found
    /// * `WorkflowError::InsufficientCFrags` - Not enough cFrags for threshold
    /// * `WorkflowError::CryptoError` - Cryptographic operation failed
    /// * `WorkflowError::DecryptionError` - Decryption failed at specific phase
    /// * `WorkflowError::StorageError` - Arweave retrieval failed
    /// * `WorkflowError::AOCommunicationError` - AO Network communication failed
    fn execute_secret_recovery(
        &self,
        request: SecretRecoveryRequest,
    ) -> WorkflowResult<SecretRecoveryResult>;

    /// Check if a secret can be recovered
    ///
    /// # Arguments
    /// * `secret_id` - The ID of the secret to check
    /// * `requester_process_id` - The requester's process ID
    ///
    /// # Returns
    /// * `WorkflowResult<bool>` - True if enough cFrags are available
    ///
    /// # Errors
    /// * `WorkflowError::ResourceNotFound` - Secret not found
    /// * `WorkflowError::AOCommunicationError` - AO Network communication failed
    fn can_recover(&self, secret_id: &SecretId, requester_process_id: &str)
    -> WorkflowResult<bool>;
}

// ============================================================================
// SecretRecoveryWorkflowServiceImpl
// ============================================================================

/// Implementation of SecretRecoveryWorkflowService
///
/// Orchestrates CryptoService and StorageService to implement
/// the complete Phase 3 secret recovery workflow.
pub struct SecretRecoveryWorkflowServiceImpl<C: CryptoService> {
    crypto_service: Arc<C>,
    // TODO: Add StorageService when AO communication is implemented (Issue #47)
    // storage_service: Arc<S>,
}

impl<C: CryptoService> SecretRecoveryWorkflowServiceImpl<C> {
    /// Create a new SecretRecoveryWorkflowServiceImpl
    pub fn new(crypto_service: Arc<C>) -> Self {
        Self { crypto_service }
    }

    /// Validate input parameters for secret recovery
    fn validate_request(&self, request: &SecretRecoveryRequest) -> WorkflowResult<()> {
        // Validate requester secret key
        if request.requester_secret_key.is_empty() {
            return Err(WorkflowError::validation(
                "Requester secret key is required",
            ));
        }

        // Validate requester process ID
        if request.requester_process_id.is_empty() {
            return Err(WorkflowError::validation(
                "Requester process ID is required",
            ));
        }

        Ok(())
    }

    /// Retrieve cFrags from AO Network
    /// TODO: Implement when StorageService AO methods are available (Issue #47)
    fn retrieve_cfrags(
        &self,
        _secret_id: &SecretId,
        _requester_process_id: &str,
    ) -> WorkflowResult<Vec<CFragData>> {
        // Placeholder: Will fetch cFrags from Requester-Process via AO
        // For now, return ResourceNotFound as storage is not implemented
        Err(WorkflowError::not_found(
            "cFrag retrieval not yet implemented (Issue #47)",
        ))
    }

    /// Retrieve Capsule from Arweave
    /// TODO: Implement when StorageService is available (Issue #47)
    fn retrieve_capsule(&self, _secret_id: &SecretId) -> WorkflowResult<Capsule> {
        // Placeholder: Will fetch Capsule from Arweave using capsule_tx_id
        Err(WorkflowError::not_found(
            "Capsule retrieval not yet implemented (Issue #47)",
        ))
    }

    /// Retrieve encrypted shares from Arweave
    /// TODO: Implement when StorageService is available (Issue #47)
    fn retrieve_encrypted_shares(&self, _secret_id: &SecretId) -> WorkflowResult<Vec<Vec<u8>>> {
        // Placeholder: Will fetch encrypted shares from Arweave
        Err(WorkflowError::not_found(
            "Encrypted shares retrieval not yet implemented (Issue #47)",
        ))
    }

    /// Retrieve threshold parameters for the secret
    /// TODO: Implement when StorageService is available (Issue #47)
    fn retrieve_threshold(&self, _secret_id: &SecretId) -> WorkflowResult<u8> {
        // Placeholder: Will fetch threshold from secret metadata
        Err(WorkflowError::not_found(
            "Threshold retrieval not yet implemented (Issue #47)",
        ))
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
    ) -> WorkflowResult<Vec<u8>> {
        self.crypto_service
            .decrypt_pre_capsule(capsule, cfrags, requester_secret_key)
            .map_err(|e| WorkflowError::DecryptionError {
                phase: format!("PRE decapsulation: {}", e),
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
                        phase: format!("AES-GCM decryption of share {}: {}", i, e),
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
                phase: format!("Shamir reconstruction: {}", e),
            })
    }

    /// Record audit trail on Arweave
    /// TODO: Implement when StorageService is available (Issue #47)
    fn record_audit_trail(
        &self,
        _secret_id: &SecretId,
        _requester_process_id: &str,
    ) -> WorkflowResult<String> {
        // Placeholder: Will record recovery event on Arweave
        // For now, return a placeholder transaction ID
        Ok("audit_not_implemented".to_string())
    }
}

impl<C: CryptoService> SecretRecoveryWorkflowService for SecretRecoveryWorkflowServiceImpl<C> {
    fn execute_secret_recovery(
        &self,
        request: SecretRecoveryRequest,
    ) -> WorkflowResult<SecretRecoveryResult> {
        // Step 0: Validate input parameters
        self.validate_request(&request)?;

        // Step 1: Retrieve cFrags from Requester-Process via AO
        let cfrags = self.retrieve_cfrags(&request.secret_id, &request.requester_process_id)?;

        // Step 2: Retrieve Capsule and encrypted shares from Arweave
        let capsule = self.retrieve_capsule(&request.secret_id)?;
        let encrypted_shares = self.retrieve_encrypted_shares(&request.secret_id)?;

        // Step 3: Retrieve threshold and verify
        let threshold = self.retrieve_threshold(&request.secret_id)?;
        self.verify_threshold(&cfrags, threshold)?;

        // Step 4-5: Decrypt Capsule and recover symmetric key
        let symmetric_key =
            self.decrypt_capsule(&capsule, &cfrags, &request.requester_secret_key)?;

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

    fn can_recover(
        &self,
        secret_id: &SecretId,
        requester_process_id: &str,
    ) -> WorkflowResult<bool> {
        // Retrieve threshold required
        let threshold = self.retrieve_threshold(secret_id)?;

        // Retrieve available cFrags
        let cfrags = self.retrieve_cfrags(secret_id, requester_process_id)?;

        // Check if we have enough
        Ok(cfrags.len() as u8 >= threshold)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::redundant_clone)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::CryptoServiceImpl;

    fn create_test_service() -> SecretRecoveryWorkflowServiceImpl<CryptoServiceImpl> {
        let crypto = Arc::new(CryptoServiceImpl::new());
        SecretRecoveryWorkflowServiceImpl::new(crypto)
    }

    fn create_test_request(crypto: &CryptoServiceImpl) -> SecretRecoveryRequest {
        let (requester_sk, _requester_pk) = crypto.generate_keypair().unwrap();

        SecretRecoveryRequest {
            secret_id: SecretId::generate(),
            requester_secret_key: requester_sk,
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

    // ========================================================================
    // Validation Tests (Task 17)
    // ========================================================================

    #[test]
    fn test_validate_request_valid() {
        println!("\n=== test_validate_request_valid ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();
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

    #[test]
    fn test_phase3_resource_not_found_cfrags() {
        println!("\n=== test_phase3_resource_not_found_cfrags ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving cFrags for secret_id: {}", secret_id);

        let result = service.retrieve_cfrags(&secret_id, "requester-123");
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] ResourceNotFound returned (storage not implemented)");
    }

    #[test]
    fn test_phase3_resource_not_found_capsule() {
        println!("\n=== test_phase3_resource_not_found_capsule ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving capsule for secret_id: {}", secret_id);

        let result = service.retrieve_capsule(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] ResourceNotFound returned (storage not implemented)");
    }

    #[test]
    fn test_phase3_resource_not_found_encrypted_shares() {
        println!("\n=== test_phase3_resource_not_found_encrypted_shares ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Retrieving encrypted shares for secret_id: {}", secret_id);

        let result = service.retrieve_encrypted_shares(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] ResourceNotFound returned (storage not implemented)");
    }

    // ========================================================================
    // Workflow Tests (Task 18)
    // ========================================================================

    #[test]
    fn test_execute_recovery_storage_not_implemented() {
        println!("\n=== test_execute_recovery_storage_not_implemented ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!("  Executing PHASE 3 recovery (should fail - storage not implemented)...");

        // Should fail at cFrag retrieval since storage is not implemented
        let result = service.execute_secret_recovery(request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Correctly failed at storage retrieval");
    }

    #[test]
    fn test_can_recover_storage_not_implemented() {
        println!("\n=== test_can_recover_storage_not_implemented ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Checking can_recover for secret_id: {}", secret_id);

        // Should fail because storage is not implemented
        let result = service.can_recover(&secret_id, "requester-process-123");
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Correctly failed at storage retrieval");
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
        let crypto = CryptoServiceImpl::new();

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
        let crypto = CryptoServiceImpl::new();

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
        let crypto = CryptoServiceImpl::new();
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
        let crypto = CryptoServiceImpl::new();

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
        let crypto = CryptoServiceImpl::new();

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

    #[test]
    fn test_phase3_ao_retrieval_error() {
        println!("\n=== test_phase3_ao_retrieval_error ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Testing AO retrieval error for secret_id: {}", secret_id);

        // This tests that cFrag retrieval returns proper error
        let result = service.retrieve_cfrags(&secret_id, "requester-123");
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        if let Err(WorkflowError::ResourceNotFound(msg)) = result {
            assert!(msg.contains("not yet implemented") || msg.contains("Issue #47"));
            println!("  [PASS] AO retrieval returns proper error: {}", msg);
        }
    }

    #[test]
    fn test_phase3_storage_retrieval_error() {
        println!("\n=== test_phase3_storage_retrieval_error ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!(
            "  Testing storage retrieval errors for secret_id: {}",
            secret_id
        );

        // Test capsule retrieval error
        println!("  Testing capsule retrieval...");
        let result = service.retrieve_capsule(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        // Test encrypted shares retrieval error
        println!("  Testing encrypted shares retrieval...");
        let result = service.retrieve_encrypted_shares(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        // Test threshold retrieval error
        println!("  Testing threshold retrieval...");
        let result = service.retrieve_threshold(&secret_id);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] All storage retrieval errors handled correctly");
    }

    #[test]
    fn test_workflow_fails_at_first_storage_operation() {
        println!("\n=== test_workflow_fails_at_first_storage_operation ===");
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);
        println!("  Executing workflow (should fail at first storage operation)...");

        // execute_secret_recovery should fail at the first storage operation (cFrag retrieval)
        let result = service.execute_secret_recovery(request);
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] Workflow correctly fails at first storage operation");
    }

    #[test]
    fn test_can_recover_fails_at_threshold_retrieval() {
        println!("\n=== test_can_recover_fails_at_threshold_retrieval ===");
        let service = create_test_service();
        let secret_id = SecretId::generate();
        println!("  Calling can_recover for secret_id: {}", secret_id);

        // can_recover first tries to get threshold, which fails
        let result = service.can_recover(&secret_id, "requester-123");
        println!("  Result: {:?}", result);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
        println!("  [PASS] can_recover fails at threshold retrieval");
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
}
