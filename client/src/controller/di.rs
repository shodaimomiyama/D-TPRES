//! Controller layer dependency injection container
//!
//! Provides centralized access to Controller layer components.

use crate::controller::extractor::{RecoverExtractor, ShareExtractor};
use crate::controller::validator::{RecoverValidator, ShareValidator};

/// Container for Controller layer components
///
/// Aggregates all Controller layer components (validators and extractors)
/// for simplified access from the Actions layer.
pub struct ControllerContainer {
    share_validator: ShareValidator,
    recover_validator: RecoverValidator,
    share_extractor: ShareExtractor,
    recover_extractor: RecoverExtractor,
}

impl ControllerContainer {
    /// Create a new ControllerContainer with default components
    pub fn new() -> Self {
        Self {
            share_validator: ShareValidator::new(),
            recover_validator: RecoverValidator::new(),
            share_extractor: ShareExtractor::new(),
            recover_extractor: RecoverExtractor::new(),
        }
    }

    /// Get reference to ShareValidator
    pub fn share_validator(&self) -> &ShareValidator {
        &self.share_validator
    }

    /// Get reference to RecoverValidator
    pub fn recover_validator(&self) -> &RecoverValidator {
        &self.recover_validator
    }

    /// Get reference to ShareExtractor
    pub fn share_extractor(&self) -> &ShareExtractor {
        &self.share_extractor
    }

    /// Get reference to RecoverExtractor
    pub fn recover_extractor(&self) -> &RecoverExtractor {
        &self.recover_extractor
    }
}

impl Default for ControllerContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

    fn create_test_keys() -> (
        crate::usecase::core::crypto::SecretKey,
        crate::usecase::core::crypto::PublicKey,
    ) {
        let crypto_service = CryptoServiceImpl::new();
        crypto_service
            .generate_keypair()
            .expect("Failed to generate test keys")
    }

    #[test]
    fn test_container_new() {
        let container = ControllerContainer::new();
        // Verify container was created successfully by using its components
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result =
            container
                .share_validator()
                .validate(b"secret", 3, 5, &owner_sk, &requester_pk);
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_default() {
        let container: ControllerContainer = Default::default();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result =
            container
                .share_validator()
                .validate(b"secret", 3, 5, &owner_sk, &requester_pk);
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_share_validator() {
        let container = ControllerContainer::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        // Test invalid threshold
        let result =
            container
                .share_validator()
                .validate(b"secret", 0, 5, &owner_sk, &requester_pk);
        assert!(result.is_err());
    }

    #[test]
    fn test_container_recover_validator() {
        let container = ControllerContainer::new();
        let (requester_sk, _) = create_test_keys();

        // Test valid parameters
        let result =
            container
                .recover_validator()
                .validate("secret_id", &requester_sk, "process_id");
        assert!(result.is_ok());

        // Test empty secret_id
        let result = container
            .recover_validator()
            .validate("", &requester_sk, "process_id");
        assert!(result.is_err());
    }

    #[test]
    fn test_container_share_extractor() {
        let container = ControllerContainer::new();
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let request = container.share_extractor().extract(
            b"secret".to_vec(),
            owner_sk,
            owner_pk,
            requester_pk,
            3,
            5,
            "owner_process".to_string(),
            None,
        );

        assert_eq!(request.threshold, 3);
        assert_eq!(request.total_shares, 5);
    }

    #[test]
    fn test_container_recover_extractor() {
        let container = ControllerContainer::new();
        let (requester_sk, _) = create_test_keys();
        let secret_id = crate::domain::value_objects::SecretId::new("test_secret");

        let request = container.recover_extractor().extract(
            secret_id,
            requester_sk,
            "requester_process".to_string(),
        );

        assert_eq!(request.secret_id.as_str(), "test_secret");
        assert_eq!(request.requester_process_id, "requester_process");
    }
}
