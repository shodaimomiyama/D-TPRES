//! Actions Layer - Developer-facing API endpoints (Facade Pattern)
//!
//! Provides simple functions for D-TPRES operations:
//! - `share()` - Phase 1: Secret splitting and distribution
//! - `recover()` - Phase 3: Secret recovery
//! - `generate_keypair()` - PRE key pair generation
//!
//! # Architecture
//! ```text
//! Developer API
//!        │
//!        ▼
//! ┌──────────────────┐
//! │   Actions Layer  │ ← This module
//! │   (Facade)       │
//! └────────┬─────────┘
//!          │
//!     ┌────┴────┐
//!     ▼         ▼
//! Controller   UseCase
//!    Layer      Layer
//! ```
//!
//! # Example
//! ```rust,ignore
//! use dtpres_client::actions::{ActionsContainer, ShareOptions};
//!
//! let container = ActionsContainer::new();
//!
//! // Generate key pairs
//! let (owner_sk, owner_pk) = container.generate_keypair()?;
//! let (requester_sk, requester_pk) = container.generate_keypair()?;
//!
//! // Share a secret
//! let result = container.share(
//!     b"my secret data".to_vec(),
//!     3,  // threshold
//!     5,  // total_shares
//!     owner_sk,
//!     requester_pk,
//!     "owner_process_123".to_string(),
//!     None,
//! )?;
//!
//! // Recover the secret
//! let recovered = container.recover(
//!     &result.secret_id.as_str(),
//!     requester_sk,
//!     "requester_process_456".to_string(),
//!     None,
//! )?;
//! ```

pub mod di;
pub mod error;
pub mod options;

pub use di::{ActionsContainer, DefaultActionsContainer};
pub use error::{ActionError, ActionResult};
pub use options::{RecoverOptions, ShareOptions};

use zeroize::Zeroizing;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{CryptoService, PublicKey, SecretKey};
use crate::usecase::dto::{SecretRecoveryResult, SecretSharingResult};
use crate::usecase::workflow::secret_recovery_service::SecretRecoveryWorkflowService;
use crate::usecase::workflow::secret_sharing_service::SecretSharingWorkflowService;

impl<C: CryptoService> ActionsContainer<C> {
    /// Share a secret by splitting and distributing it
    ///
    /// This is the main Phase 1 API endpoint that:
    /// 1. Validates input parameters via Controller layer
    /// 2. Creates SecretSharingRequest DTO via Extractor
    /// 3. Executes Phase 1 workflow via WorkflowService
    /// 4. Returns SecretSharingResult with secret_id and kFrags info
    ///
    /// # Arguments
    /// * `secret` - Secret data to split (will be zeroized after use)
    /// * `threshold` - Minimum shares required for reconstruction (k)
    /// * `total_shares` - Total number of shares to generate (n)
    /// * `owner_secret_key` - Owner's secret key for PRE
    /// * `owner_public_key` - Owner's public key for PRE (generated with owner_secret_key)
    /// * `requester_public_key` - Requester's public key for PRE
    /// * `owner_process_id` - Owner-Process ID for AO communication
    /// * `options` - Optional parameters (metadata, etc.)
    ///
    /// # Returns
    /// * `Ok(SecretSharingResult)` - Contains secret_id, tx IDs, and kFrag count
    /// * `Err(ActionError::ValidationFailed)` - Invalid parameters
    /// * `Err(ActionError::WorkflowFailed)` - Workflow execution failed
    ///
    /// # Example
    /// ```rust,ignore
    /// let (owner_sk, owner_pk) = container.generate_keypair()?;
    /// let (_, requester_pk) = container.generate_keypair()?;
    /// let result = container.share(
    ///     b"my secret".to_vec(),
    ///     3, 5,
    ///     owner_sk, owner_pk, requester_pk,
    ///     "owner_process".to_string(),
    ///     None,
    /// )?;
    /// println!("Secret ID: {}", result.secret_id);
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn share(
        &self,
        secret: Vec<u8>,
        threshold: u8,
        total_shares: u8,
        owner_secret_key: SecretKey,
        owner_public_key: PublicKey,
        requester_public_key: PublicKey,
        owner_process_id: String,
        options: Option<ShareOptions>,
    ) -> ActionResult<SecretSharingResult> {
        // Wrap secret in Zeroizing to ensure cleanup on early returns
        let mut secret = Zeroizing::new(secret);

        // Step 1: Validate parameters via Controller layer
        self.controller().share_validator().validate(
            &secret,
            threshold,
            total_shares,
            &owner_secret_key,
            &owner_public_key,
            &requester_public_key,
        )?;

        // Step 2: Extract DTO via Controller layer
        // Take ownership from Zeroizing wrapper (leaves empty vec, which is a no-op for zeroize)
        let metadata = options.and_then(|o| o.metadata);
        let request = self.controller().share_extractor().extract(
            std::mem::take(&mut *secret),
            owner_secret_key,
            owner_public_key,
            requester_public_key,
            threshold,
            total_shares,
            owner_process_id,
            metadata,
        );

        // Step 3: Execute workflow via UseCase layer
        let result = self
            .workflow_services()
            .secret_sharing_service()
            .execute_secret_sharing(request)?;

        Ok(result)
    }

    /// Recover a secret using the secret_id
    ///
    /// This is the main Phase 3 API endpoint that:
    /// 1. Validates input parameters via Controller layer
    /// 2. Creates SecretRecoveryRequest DTO via Extractor
    /// 3. Executes Phase 3 workflow via WorkflowService
    /// 4. Returns SecretRecoveryResult with recovered secret (Zeroize on drop)
    ///
    /// # Arguments
    /// * `secret_id` - ID of the secret to recover
    /// * `requester_secret_key` - Requester's secret key for PRE decryption
    /// * `requester_process_id` - Requester-Process ID for AO communication
    /// * `options` - Optional parameters (reserved for future use)
    ///
    /// # Returns
    /// * `Ok(SecretRecoveryResult)` - Contains recovered_secret (Zeroize on drop)
    /// * `Err(ActionError::ValidationFailed)` - Invalid parameters
    /// * `Err(ActionError::ResourceNotFound)` - Secret not found
    /// * `Err(ActionError::WorkflowFailed)` - Workflow execution failed
    ///
    /// # Security
    /// The `recovered_secret` field in the result implements Zeroize trait
    /// and will be automatically cleared from memory when dropped.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = container.recover(
    ///     "secret_abc123",
    ///     requester_sk,
    ///     "requester_process".to_string(),
    ///     None,
    /// )?;
    /// // Use result.recovered_secret
    /// // Memory is cleared when result goes out of scope
    /// ```
    pub fn recover(
        &self,
        secret_id: &str,
        requester_secret_key: SecretKey,
        requester_process_id: String,
        _options: Option<RecoverOptions>,
    ) -> ActionResult<SecretRecoveryResult> {
        // Step 1: Validate parameters via Controller layer
        self.controller().recover_validator().validate(
            secret_id,
            &requester_secret_key,
            &requester_process_id,
        )?;

        // Step 2: Extract DTO via Controller layer
        let secret_id_value = SecretId::new(secret_id);
        let request = self.controller().recover_extractor().extract(
            secret_id_value,
            requester_secret_key,
            requester_process_id,
        );

        // Step 3: Execute workflow via UseCase layer
        let result = self
            .workflow_services()
            .secret_recovery_service()
            .execute_secret_recovery(request)?;

        Ok(result)
    }

    /// Generate a PRE key pair for owner or requester operations
    ///
    /// Creates an Umbral PRE-compatible key pair that can be used for:
    /// - Owner: `owner_secret_key` and `owner_public_key` in share()
    /// - Requester: `requester_secret_key` in recover(), `requester_public_key` in share()
    ///
    /// # Returns
    /// * `Ok((SecretKey, PublicKey))` - Generated key pair
    /// * `Err(ActionError::CryptoError)` - Key generation failed
    ///
    /// # Security
    /// - `SecretKey` implements Zeroize trait and is cleared on drop
    /// - Keys are generated using cryptographically secure random
    /// - `SecretKey` is 32 bytes (256-bit)
    ///
    /// # Example
    /// ```rust,ignore
    /// let (secret_key, public_key) = container.generate_keypair()?;
    /// // Use keys for share() or recover()
    /// // secret_key is cleared when dropped
    /// ```
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.crypto_service()
            .generate_keypair()
            .map_err(|e| ActionError::crypto_error(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_valid_params() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container.share(
            b"test secret data".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process_123".to_string(),
            None,
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.secret_id.as_str().is_empty());
        assert_eq!(result.kfrag_count, 5);
    }

    #[test]
    fn test_share_empty_secret() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container.share(
            vec![],
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process_123".to_string(),
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ValidationFailed { code, .. } => {
                assert!(code.contains("secret") || code.contains("empty"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_share_zero_threshold() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container.share(
            b"test secret".to_vec(),
            0,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ValidationFailed { code, .. } => {
                assert!(code.contains("threshold"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_share_threshold_exceeds_total() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container.share(
            b"test secret".to_vec(),
            6,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ValidationFailed { code, .. } => {
                assert!(code.contains("threshold"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_share_result_has_valid_secret_id() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                b"test secret".to_vec(),
                3,
                5,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .unwrap();

        assert!(!result.secret_id.as_str().is_empty());
    }

    #[test]
    fn test_share_result_kfrag_count_matches() {
        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let result = container
            .share(
                b"test secret".to_vec(),
                3,
                7,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            )
            .unwrap();

        assert_eq!(result.kfrag_count, 7);
    }

    #[test]
    fn test_share_with_metadata() {
        use crate::usecase::dto::SecretMetadata;

        let container = DefaultActionsContainer::new();
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        let metadata = SecretMetadata {
            name: Some("test secret".to_string()),
            description: Some("A test secret".to_string()),
            expires_at: Some(1735689600),
            tags: vec!["test".to_string()],
        };
        let options = ShareOptions::with_metadata(metadata);

        let result = container.share(
            b"test secret".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            Some(options),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_various_threshold_combinations() {
        let container = DefaultActionsContainer::new();

        let combinations = [(2, 3), (3, 5), (5, 10), (2, 10)];

        for (threshold, total) in combinations {
            let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
            let (_, requester_pk) = container.generate_keypair().unwrap();

            let result = container.share(
                b"test secret".to_vec(),
                threshold,
                total,
                owner_sk,
                owner_pk,
                requester_pk,
                "owner_process".to_string(),
                None,
            );

            assert!(
                result.is_ok(),
                "Failed for threshold={}, total={}",
                threshold,
                total
            );
            assert_eq!(result.unwrap().kfrag_count, total);
        }
    }

    #[test]
    fn test_recover_empty_secret_id() {
        let container = DefaultActionsContainer::new();
        let (requester_sk, _) = container.generate_keypair().unwrap();

        let result = container.recover("", requester_sk, "requester_process".to_string(), None);

        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ValidationFailed { code, .. } => {
                assert!(code.contains("secret_id"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_recover_empty_process_id() {
        let container = DefaultActionsContainer::new();
        let (requester_sk, _) = container.generate_keypair().unwrap();

        let result = container.recover("test_secret_id", requester_sk, String::new(), None);

        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ValidationFailed { code, .. } => {
                assert!(code.contains("process_id"));
            }
            _ => panic!("Expected ValidationFailed"),
        }
    }

    #[test]
    fn test_recover_nonexistent_secret() {
        let container = DefaultActionsContainer::new();
        let (requester_sk, _) = container.generate_keypair().unwrap();

        let result = container.recover(
            "nonexistent_secret_id",
            requester_sk,
            "requester_process".to_string(),
            None,
        );

        // Should fail with ResourceNotFound since storage is not implemented
        assert!(result.is_err());
        match result.unwrap_err() {
            ActionError::ResourceNotFound { .. } | ActionError::WorkflowFailed { .. } => {
                // Expected - storage not implemented
            }
            other => panic!(
                "Expected ResourceNotFound or WorkflowFailed, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_generate_keypair_valid() {
        let container = DefaultActionsContainer::new();
        let result = container.generate_keypair();
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_keypair_secret_key_not_empty() {
        let container = DefaultActionsContainer::new();
        let (secret_key, _) = container.generate_keypair().unwrap();
        assert!(!secret_key.is_empty());
    }

    #[test]
    fn test_generate_keypair_public_key_not_empty() {
        let container = DefaultActionsContainer::new();
        let (_, public_key) = container.generate_keypair().unwrap();
        assert!(!public_key.key_data.is_empty());
    }

    #[test]
    fn test_generate_keypair_unique() {
        let container = DefaultActionsContainer::new();

        let (_, pk1) = container.generate_keypair().unwrap();
        let (_, pk2) = container.generate_keypair().unwrap();

        assert_ne!(pk1.key_data, pk2.key_data);
    }

    #[test]
    fn test_generate_keypair_usable_with_share() {
        let container = DefaultActionsContainer::new();

        // Generate keys
        let (owner_sk, owner_pk) = container.generate_keypair().unwrap();
        let (_, requester_pk) = container.generate_keypair().unwrap();

        // Use keys in share
        let result = container.share(
            b"test secret".to_vec(),
            3,
            5,
            owner_sk,
            owner_pk,
            requester_pk,
            "owner_process".to_string(),
            None,
        );

        assert!(result.is_ok());
    }
}
