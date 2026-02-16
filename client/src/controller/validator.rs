//! Controller layer validators for input validation
//!
//! Provides early validation before DTO construction to fail fast on invalid inputs.

use std::sync::Arc;

use subtle::ConstantTimeEq;

use crate::controller::error::{MAX_SHARES, MIN_THRESHOLD, ValidationError, error_codes};
use crate::usecase::core::crypto::{CryptoService, PublicKey, SecretKey};

/// Validator for secret sharing requests
///
/// Validates input parameters before SecretSharingRequest DTO construction.
/// Validation order is deterministic to ensure consistent error reporting.
pub struct ShareValidator<C: CryptoService> {
    crypto_service: Arc<C>,
}

impl<C: CryptoService> ShareValidator<C> {
    /// Create a new ShareValidator with CryptoService for key validation
    pub fn new(crypto_service: Arc<C>) -> Self {
        Self { crypto_service }
    }

    /// Validate share request parameters
    ///
    /// Validation order (deterministic, per design.md):
    /// 1. secret empty check
    /// 2. threshold > 0 check
    /// 3. threshold >= MIN_THRESHOLD check
    /// 4. total_shares <= MAX_SHARES check
    /// 5. threshold <= total_shares check
    /// 6. owner_secret_key empty check (AC 1.4)
    /// 7. requester_public_key empty check (AC 1.5)
    /// 8. owner_public_key empty check
    /// 9. owner key pair consistency check (owner_public_key matches owner_secret_key)
    ///
    /// # Arguments
    /// * `secret` - Secret data to be split
    /// * `threshold` - Minimum shares required for reconstruction (k)
    /// * `total_shares` - Total number of shares to generate (n)
    /// * `owner_secret_key` - Owner's secret key (validated for non-empty)
    /// * `owner_public_key` - Owner's public key (validated for non-empty and consistency)
    /// * `requester_public_key` - Requester's public key (validated for non-empty)
    ///
    /// # Returns
    /// * `Ok(())` - All validations passed
    /// * `Err(ValidationError)` - First validation failure
    pub fn validate(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
        owner_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        requester_public_key: &PublicKey,
    ) -> Result<(), ValidationError> {
        // 1. Secret empty check
        if secret.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::SECRET_EMPTY,
                "Secret data cannot be empty",
                "secret",
            ));
        }

        // 2. Threshold > 0 check
        if threshold == 0 {
            return Err(ValidationError::with_field(
                error_codes::INVALID_THRESHOLD,
                "Threshold must be greater than 0",
                "threshold",
            ));
        }

        // 3. Threshold >= MIN_THRESHOLD check
        if threshold < MIN_THRESHOLD {
            return Err(ValidationError::with_field(
                error_codes::THRESHOLD_BELOW_MIN,
                format!("Threshold must be at least {MIN_THRESHOLD}"),
                "threshold",
            ));
        }

        // 4. Total shares <= MAX_SHARES check
        if total_shares > MAX_SHARES {
            return Err(ValidationError::with_field(
                error_codes::TOTAL_SHARES_EXCEEDS_MAX,
                format!("Total shares cannot exceed {MAX_SHARES}"),
                "total_shares",
            ));
        }

        // 5. Threshold <= total_shares check
        if threshold > total_shares {
            return Err(ValidationError::with_field(
                error_codes::THRESHOLD_EXCEEDS_TOTAL,
                "Threshold (k) cannot exceed total shares (n)",
                "threshold",
            ));
        }

        // 6. Owner secret key empty check (AC 1.4)
        if owner_secret_key.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_OWNER_KEY,
                "Owner secret key cannot be empty",
                "owner_secret_key",
            ));
        }

        // 7. Requester public key empty check (AC 1.5)
        if requester_public_key.key_data.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_REQUESTER_KEY,
                "Requester public key cannot be empty",
                "requester_public_key",
            ));
        }

        // 8. Owner public key empty check
        if owner_public_key.key_data.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_OWNER_PUBLIC_KEY,
                "Owner public key cannot be empty",
                "owner_public_key",
            ));
        }

        // 9. Owner key pair consistency check
        self.validate_key_pair_consistency(owner_secret_key, owner_public_key)?;

        Ok(())
    }

    /// Validate that owner_public_key is derived from owner_secret_key
    ///
    /// Uses constant-time comparison to prevent timing attacks
    fn validate_key_pair_consistency(
        &self,
        owner_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
    ) -> Result<(), ValidationError> {
        // Derive public key from secret key
        let derived_pk = self
            .crypto_service
            .derive_public_key(owner_secret_key)
            .map_err(|_| {
                ValidationError::with_field(
                    error_codes::INVALID_OWNER_KEY,
                    "Failed to derive public key from owner secret key",
                    "owner_secret_key",
                )
            })?;

        // Constant-time comparison to prevent timing attacks
        let keys_match: bool = derived_pk.key_data.ct_eq(&owner_public_key.key_data).into();

        if !keys_match {
            return Err(ValidationError::with_field(
                error_codes::KEY_MISMATCH,
                "Owner public key does not match owner secret key",
                "owner_public_key",
            ));
        }

        Ok(())
    }
}

/// Validator for secret recovery requests
///
/// Validates input parameters before SecretRecoveryRequest DTO construction.
pub struct RecoverValidator;

impl RecoverValidator {
    /// Create a new RecoverValidator
    pub fn new() -> Self {
        Self
    }

    /// Validate recover request parameters
    ///
    /// Validation order (deterministic):
    /// 1. secret_id empty check
    /// 2. requester_secret_key empty check
    /// 3. requester_process_id empty check
    ///
    /// # Arguments
    /// * `secret_id` - ID of the secret to recover
    /// * `requester_secret_key` - Requester's secret key (validated for non-empty)
    /// * `requester_process_id` - Requester's process ID
    ///
    /// # Returns
    /// * `Ok(())` - All validations passed
    /// * `Err(ValidationError)` - First validation failure
    pub fn validate(
        &self,
        secret_id: &str,
        requester_secret_key: &SecretKey,
        owner_public_key: &PublicKey,
        requester_process_id: &str,
    ) -> Result<(), ValidationError> {
        // 1. Secret ID empty check
        if secret_id.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_SECRET_ID,
                "Secret ID cannot be empty",
                "secret_id",
            ));
        }

        // 2. Requester secret key empty check (AC 2.2)
        if requester_secret_key.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_REQUESTER_KEY,
                "Requester secret key cannot be empty",
                "requester_secret_key",
            ));
        }

        // 3. Owner public key empty check
        if owner_public_key.key_data.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_OWNER_PUBLIC_KEY,
                "Owner public key cannot be empty",
                "owner_public_key",
            ));
        }

        // 4. Requester process ID empty check
        if requester_process_id.is_empty() {
            return Err(ValidationError::with_field(
                error_codes::INVALID_PROCESS_ID,
                "Process ID cannot be empty",
                "requester_process_id",
            ));
        }

        Ok(())
    }
}

impl Default for RecoverValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::core::crypto::CryptoService;
    use crate::usecase::core::crypto::CryptoServiceImpl;

    fn create_crypto_service() -> Arc<CryptoServiceImpl> {
        Arc::new(CryptoServiceImpl::new())
    }

    fn create_test_keys() -> (SecretKey, PublicKey) {
        let crypto_service = CryptoServiceImpl::new();
        crypto_service
            .generate_keypair()
            .expect("Failed to generate test keys")
    }

    fn create_empty_secret_key() -> SecretKey {
        SecretKey::empty_for_test()
    }

    fn create_empty_public_key() -> PublicKey {
        PublicKey { key_data: vec![] }
    }

    // ========================================================================
    // ShareValidator Tests
    // ========================================================================

    #[test]
    fn test_share_validator_empty_secret() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(&[], 3, 5, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::SECRET_EMPTY);
        assert_eq!(err.field(), Some("secret"));
    }

    #[test]
    fn test_share_validator_empty_owner_key() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let empty_owner_sk = create_empty_secret_key();
        let (_, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 5, &empty_owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_OWNER_KEY);
        assert_eq!(err.field(), Some("owner_secret_key"));
    }

    #[test]
    fn test_share_validator_empty_requester_key() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let empty_requester_pk = create_empty_public_key();

        let result = validator.validate(b"secret", 3, 5, &owner_sk, &owner_pk, &empty_requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_REQUESTER_KEY);
        assert_eq!(err.field(), Some("requester_public_key"));
    }

    #[test]
    fn test_share_validator_zero_threshold() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 0, 5, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_THRESHOLD);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_threshold_below_min() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 1, 5, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::THRESHOLD_BELOW_MIN);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_total_shares_exceeds_max() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 25, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::TOTAL_SHARES_EXCEEDS_MAX);
        assert_eq!(err.field(), Some("total_shares"));
    }

    #[test]
    fn test_share_validator_threshold_exceeds_total() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 6, 5, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::THRESHOLD_EXCEEDS_TOTAL);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_valid_params() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret data", 3, 5, &owner_sk, &owner_pk, &requester_pk);

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_validator_min_valid_threshold() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(
            b"secret",
            MIN_THRESHOLD,
            5,
            &owner_sk,
            &owner_pk,
            &requester_pk,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_validator_max_valid_shares() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, owner_pk) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(
            b"secret",
            3,
            MAX_SHARES,
            &owner_sk,
            &owner_pk,
            &requester_pk,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_validator_empty_owner_public_key() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, _) = create_test_keys();
        let empty_owner_pk = create_empty_public_key();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 5, &owner_sk, &empty_owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_OWNER_PUBLIC_KEY);
        assert_eq!(err.field(), Some("owner_public_key"));
    }

    #[test]
    fn test_share_validator_key_mismatch() {
        let crypto_service = create_crypto_service();
        let validator = ShareValidator::new(crypto_service);
        let (owner_sk, _) = create_test_keys();
        let (_, wrong_owner_pk) = create_test_keys(); // Different keypair
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 5, &owner_sk, &wrong_owner_pk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::KEY_MISMATCH);
        assert_eq!(err.field(), Some("owner_public_key"));
    }

    // ========================================================================
    // RecoverValidator Tests
    // ========================================================================

    #[test]
    fn test_recover_validator_empty_secret_id() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();

        let result = validator.validate("", &requester_sk, &owner_pk, "process_123");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_SECRET_ID);
        assert_eq!(err.field(), Some("secret_id"));
    }

    #[test]
    fn test_recover_validator_empty_requester_key() {
        let validator = RecoverValidator::new();
        let empty_requester_sk = create_empty_secret_key();
        let (_, owner_pk) = create_test_keys();

        let result = validator.validate(
            "secret_abc123",
            &empty_requester_sk,
            &owner_pk,
            "process_123",
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_REQUESTER_KEY);
        assert_eq!(err.field(), Some("requester_secret_key"));
    }

    #[test]
    fn test_recover_validator_empty_owner_public_key() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();
        let empty_owner_pk = create_empty_public_key();

        let result = validator.validate(
            "secret_abc123",
            &requester_sk,
            &empty_owner_pk,
            "process_123",
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_OWNER_PUBLIC_KEY);
        assert_eq!(err.field(), Some("owner_public_key"));
    }

    #[test]
    fn test_recover_validator_empty_process_id() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();

        let result = validator.validate("secret_abc123", &requester_sk, &owner_pk, "");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_PROCESS_ID);
        assert_eq!(err.field(), Some("requester_process_id"));
    }

    #[test]
    fn test_recover_validator_valid_params() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();

        let result = validator.validate("secret_abc123", &requester_sk, &owner_pk, "process_123");

        assert!(result.is_ok());
    }

    // ========================================================================
    // Default Trait Tests
    // ========================================================================

    #[test]
    fn test_recover_validator_default() {
        let validator: RecoverValidator = Default::default();
        let (requester_sk, _) = create_test_keys();
        let (_, owner_pk) = create_test_keys();

        let result = validator.validate("secret_id", &requester_sk, &owner_pk, "process_id");
        assert!(result.is_ok());
    }
}
