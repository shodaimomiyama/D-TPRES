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

                Ok(ShamirShare {
                    index: (i + 1) as u8,
                    data: decrypted_data,
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
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        let result = service.validate_request(&request);
        assert!(result.is_ok());
    }

    // Note: test_phase3_invalid_accessor_key is not implemented because SecretKey
    // has private fields and cannot be created with empty/invalid data directly.
    // This is a security feature - valid SecretKeys can only be created through
    // the crypto service which generates proper cryptographic keys.

    #[test]
    fn test_validate_request_empty_process_id() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let mut request = create_test_request(&crypto);
        request.requester_process_id = String::new();

        let result = service.validate_request(&request);
        assert!(matches!(result, Err(WorkflowError::ValidationError(_))));

        if let Err(WorkflowError::ValidationError(msg)) = result {
            assert!(msg.contains("process ID"));
        }
    }

    #[test]
    fn test_phase3_validates_cfrag_count() {
        let service = create_test_service();

        // Exactly threshold
        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(result.is_ok());

        // Above threshold
        let cfrags = create_mock_cfrags(5);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_phase3_insufficient_cfrags() {
        let service = create_test_service();

        let cfrags = create_mock_cfrags(2);
        let result = service.verify_threshold(&cfrags, 3);

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
        }
    }

    #[test]
    fn test_phase3_resource_not_found_cfrags() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        let result = service.retrieve_cfrags(&secret_id, "requester-123");
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_phase3_resource_not_found_capsule() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        let result = service.retrieve_capsule(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_phase3_resource_not_found_encrypted_shares() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        let result = service.retrieve_encrypted_shares(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    // ========================================================================
    // Workflow Tests (Task 18)
    // ========================================================================

    #[test]
    fn test_execute_recovery_storage_not_implemented() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        // Should fail at cFrag retrieval since storage is not implemented
        let result = service.execute_secret_recovery(request);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_can_recover_storage_not_implemented() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        // Should fail because storage is not implemented
        let result = service.can_recover(&secret_id, "requester-process-123");
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_verify_threshold_success() {
        let service = create_test_service();

        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_threshold_insufficient() {
        let service = create_test_service();

        let cfrags = create_mock_cfrags(2);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(matches!(
            result,
            Err(WorkflowError::InsufficientCFrags {
                required: 3,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_phase3_exactly_k_cfrags() {
        let service = create_test_service();

        // Exactly k (threshold) cFrags should be sufficient
        let cfrags = create_mock_cfrags(3);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_phase3_more_than_k_cfrags() {
        let service = create_test_service();

        // More than k cFrags should also work
        let cfrags = create_mock_cfrags(7);
        let result = service.verify_threshold(&cfrags, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_phase3_zero_cfrags() {
        let service = create_test_service();

        let cfrags: Vec<CFragData> = vec![];
        let result = service.verify_threshold(&cfrags, 2);
        assert!(matches!(
            result,
            Err(WorkflowError::InsufficientCFrags {
                required: 2,
                actual: 0
            })
        ));
    }

    #[test]
    fn test_record_audit_trail_placeholder() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        let result = service.record_audit_trail(&secret_id, "requester-123");
        assert!(result.is_ok());

        // Placeholder returns a specific string
        let audit_tx_id = result.unwrap();
        assert!(!audit_tx_id.is_empty());
    }

    // ========================================================================
    // Decryption Helper Tests (Task 18 continued)
    // ========================================================================

    #[test]
    fn test_decrypt_shares_with_valid_key() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        // Generate a valid symmetric key
        let symmetric_key = crypto.generate_symmetric_key().unwrap();

        // Encrypt some test data
        let original_data = vec![b"share1".to_vec(), b"share2".to_vec()];
        let encrypted_shares: Vec<Vec<u8>> = original_data
            .iter()
            .map(|data| crypto.aes_gcm_encrypt(&symmetric_key, data).unwrap())
            .collect();

        // Decrypt using the service
        let result = service.decrypt_shares(&encrypted_shares, &symmetric_key);
        assert!(result.is_ok());

        let decrypted = result.unwrap();
        assert_eq!(decrypted.len(), 2);

        // Verify decrypted data matches original
        assert_eq!(decrypted[0].data, b"share1");
        assert_eq!(decrypted[1].data, b"share2");
    }

    #[test]
    fn test_decrypt_shares_with_invalid_key() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        let correct_key = crypto.generate_symmetric_key().unwrap();
        let wrong_key = crypto.generate_symmetric_key().unwrap();

        // Encrypt with correct key
        let encrypted_share = crypto.aes_gcm_encrypt(&correct_key, b"test data").unwrap();

        // Try to decrypt with wrong key
        let result = service.decrypt_shares(&[encrypted_share], &wrong_key);
        assert!(matches!(result, Err(WorkflowError::DecryptionError { .. })));
    }

    #[test]
    fn test_decrypt_shares_empty_input() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let symmetric_key = crypto.generate_symmetric_key().unwrap();

        let encrypted_shares: Vec<Vec<u8>> = vec![];
        let result = service.decrypt_shares(&encrypted_shares, &symmetric_key);
        assert!(result.is_ok());

        let decrypted = result.unwrap();
        assert!(decrypted.is_empty());
    }

    // ========================================================================
    // Shamir Reconstruction Tests (Task 18 continued)
    // ========================================================================

    #[test]
    fn test_reconstruct_secret_with_valid_shares() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        // Create original secret and split it
        let original_secret = b"Test secret!".to_vec();
        let shares = crypto.split_secret_shamir(&original_secret, 3, 5).unwrap();

        // Take exactly threshold number of shares
        let subset: Vec<ShamirShare> = shares.into_iter().take(3).collect();

        // Reconstruct
        let result = service.reconstruct_secret(&subset, 3);
        assert!(result.is_ok());

        let recovered = result.unwrap();
        assert_eq!(recovered, original_secret);
    }

    #[test]
    fn test_reconstruct_secret_insufficient_shares() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();

        let original_secret = b"Test secret!".to_vec();
        let shares = crypto.split_secret_shamir(&original_secret, 3, 5).unwrap();

        // Take less than threshold
        let subset: Vec<ShamirShare> = shares.into_iter().take(2).collect();

        let result = service.reconstruct_secret(&subset, 3);
        // Reconstruction should fail due to insufficient shares
        assert!(result.is_err());
    }

    // ========================================================================
    // Error Handling Tests (Task 19)
    // ========================================================================

    #[test]
    fn test_phase3_decryption_error_message() {
        let err = WorkflowError::DecryptionError {
            phase: "PRE decapsulation".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Decryption failed"));
        assert!(msg.contains("PRE decapsulation"));
    }

    #[test]
    fn test_phase3_ao_retrieval_error() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        // This tests that cFrag retrieval returns proper error
        let result = service.retrieve_cfrags(&secret_id, "requester-123");
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        if let Err(WorkflowError::ResourceNotFound(msg)) = result {
            assert!(msg.contains("not yet implemented") || msg.contains("Issue #47"));
        }
    }

    #[test]
    fn test_phase3_storage_retrieval_error() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        // Test capsule retrieval error
        let result = service.retrieve_capsule(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        // Test encrypted shares retrieval error
        let result = service.retrieve_encrypted_shares(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));

        // Test threshold retrieval error
        let result = service.retrieve_threshold(&secret_id);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_workflow_fails_at_first_storage_operation() {
        let service = create_test_service();
        let crypto = CryptoServiceImpl::new();
        let request = create_test_request(&crypto);

        // execute_secret_recovery should fail at the first storage operation (cFrag retrieval)
        let result = service.execute_secret_recovery(request);
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    #[test]
    fn test_can_recover_fails_at_threshold_retrieval() {
        let service = create_test_service();
        let secret_id = SecretId::generate();

        // can_recover first tries to get threshold, which fails
        let result = service.can_recover(&secret_id, "requester-123");
        assert!(matches!(result, Err(WorkflowError::ResourceNotFound(_))));
    }

    // ========================================================================
    // CFragData Structure Tests
    // ========================================================================

    #[test]
    fn test_cfrag_data_structure() {
        let cfrag = CFragData {
            cfrag_data: vec![1, 2, 3, 4],
            holder_id: "holder-abc".to_string(),
        };

        assert_eq!(cfrag.cfrag_data, vec![1, 2, 3, 4]);
        assert_eq!(cfrag.holder_id, "holder-abc");
    }

    #[test]
    fn test_cfrag_data_clone() {
        let cfrag = CFragData {
            cfrag_data: vec![1, 2, 3, 4],
            holder_id: "holder-abc".to_string(),
        };

        let cloned = cfrag.clone();
        assert_eq!(cloned.cfrag_data, cfrag.cfrag_data);
        assert_eq!(cloned.holder_id, cfrag.holder_id);
    }
}
