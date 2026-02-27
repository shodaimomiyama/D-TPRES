//! Controller layer dependency injection container
//!
//! Provides centralized access to Controller layer components.

use std::sync::Arc;

use crate::controller::extractor::{RecoverExtractor, ShareExtractor};
use crate::controller::validator::{RecoverValidator, ShareValidator};
use crate::usecase::core::crypto::CryptoService;

/// Container for Controller layer components
///
/// Aggregates all Controller layer components (validators and extractors)
/// for simplified access from the Actions layer.
pub struct ControllerContainer<C: CryptoService> {
    share_validator: ShareValidator<C>,
    recover_validator: RecoverValidator,
    share_extractor: ShareExtractor,
    recover_extractor: RecoverExtractor,
}

impl<C: CryptoService> ControllerContainer<C> {
    /// Create a new ControllerContainer with CryptoService for key validation
    pub fn new(crypto_service: Arc<C>) -> Self {
        Self {
            share_validator: ShareValidator::new(crypto_service),
            recover_validator: RecoverValidator::new(),
            share_extractor: ShareExtractor::new(),
            recover_extractor: RecoverExtractor::new(),
        }
    }

    /// Get reference to ShareValidator
    pub fn share_validator(&self) -> &ShareValidator<C> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

    fn create_crypto_service() -> Arc<CryptoServiceImpl> {
        Arc::new(CryptoServiceImpl::new())
    }

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
        let crypto_service = create_crypto_service();
        let container = ControllerContainer::new(crypto_service);
        // Verify container was created successfully by using its components
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = container.share_validator().validate(
            b"secret",
            3,
            5,
            &owner_sk,
            &owner_pk,
            &requester_pk,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_share_validator() {
        let crypto_service = create_crypto_service();
        let container = ControllerContainer::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        // Test invalid threshold
        let result = container.share_validator().validate(
            b"secret",
            0,
            5,
            &owner_sk,
            &owner_pk,
            &requester_pk,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_container_recover_validator() {
        let crypto_service = create_crypto_service();
        let container = ControllerContainer::new(crypto_service);
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();

        // Test valid parameters
        let result = container.recover_validator().validate(
            "secret_id",
            &requester_sk,
            &owner_pk,
            "process_id",
        );
        assert!(result.is_ok());

        // Test empty secret_id
        let (_, owner_pk2) = create_test_keys();
        let result =
            container
                .recover_validator()
                .validate("", &requester_sk, &owner_pk2, "process_id");
        assert!(result.is_err());
    }

    #[test]
    fn test_container_share_extractor() {
        let crypto_service = create_crypto_service();
        let container = ControllerContainer::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let request = container.share_extractor().extract(
            zeroize::Zeroizing::new(b"secret".to_vec()),
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
        let crypto_service = create_crypto_service();
        let container = ControllerContainer::new(crypto_service);
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();
        let secret_id = crate::domain::value_objects::SecretId::new("test_secret");

        let request = container.recover_extractor().extract(
            secret_id,
            requester_sk,
            owner_pk,
            "requester_process".to_string(),
        );

        assert_eq!(request.secret_id.as_str(), "test_secret");
        assert_eq!(request.requester_process_id, "requester_process");
    }
}
