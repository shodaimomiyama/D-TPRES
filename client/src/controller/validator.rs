//! Controller layer validators for input validation
//!
//! Provides early validation before DTO construction to fail fast on invalid inputs.

use crate::controller::error::{MAX_SHARES, MIN_THRESHOLD, ValidationError, error_codes};
use crate::usecase::core::crypto::{PublicKey, SecretKey};

/// Validator for secret sharing requests
///
/// Validates input parameters before SecretSharingRequest DTO construction.
/// Validation order is deterministic to ensure consistent error reporting.
pub struct ShareValidator;

impl ShareValidator {
    /// Create a new ShareValidator
    pub fn new() -> Self {
        Self
    }

    /// Validate share request parameters
    ///
    /// Validation order (deterministic):
    /// 1. secret empty check
    /// 2. threshold > 0 check
    /// 3. threshold >= MIN_THRESHOLD check
    /// 4. total_shares <= MAX_SHARES check
    /// 5. threshold <= total_shares check
    ///
    /// # Arguments
    /// * `secret` - Secret data to be split
    /// * `threshold` - Minimum shares required for reconstruction (k)
    /// * `total_shares` - Total number of shares to generate (n)
    /// * `_owner_secret_key` - Owner's secret key (validated for non-empty)
    /// * `_requester_public_key` - Requester's public key (validated for non-empty)
    ///
    /// # Returns
    /// * `Ok(())` - All validations passed
    /// * `Err(ValidationError)` - First validation failure
    pub fn validate(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
        _owner_secret_key: &SecretKey,
        _requester_public_key: &PublicKey,
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
                format!("Threshold must be at least {}", MIN_THRESHOLD),
                "threshold",
            ));
        }

        // 4. Total shares <= MAX_SHARES check
        if total_shares > MAX_SHARES {
            return Err(ValidationError::with_field(
                error_codes::TOTAL_SHARES_EXCEEDS_MAX,
                format!("Total shares cannot exceed {}", MAX_SHARES),
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

        Ok(())
    }
}

impl Default for ShareValidator {
    fn default() -> Self {
        Self::new()
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
    /// 2. requester_process_id empty check
    ///
    /// # Arguments
    /// * `secret_id` - ID of the secret to recover
    /// * `_requester_secret_key` - Requester's secret key
    /// * `requester_process_id` - Requester's process ID
    ///
    /// # Returns
    /// * `Ok(())` - All validations passed
    /// * `Err(ValidationError)` - First validation failure
    pub fn validate(
        &self,
        secret_id: &str,
        _requester_secret_key: &SecretKey,
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

        // 2. Requester process ID empty check
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

    fn create_test_keys() -> (SecretKey, PublicKey) {
        let crypto_service = CryptoServiceImpl::new();
        crypto_service
            .generate_keypair()
            .expect("Failed to generate test keys")
    }

    // ========================================================================
    // ShareValidator Tests
    // ========================================================================

    #[test]
    fn test_share_validator_empty_secret() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(&[], 3, 5, &owner_sk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::SECRET_EMPTY);
        assert_eq!(err.field(), Some("secret"));
    }

    #[test]
    fn test_share_validator_zero_threshold() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 0, 5, &owner_sk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_THRESHOLD);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_threshold_below_min() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 1, 5, &owner_sk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::THRESHOLD_BELOW_MIN);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_total_shares_exceeds_max() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 25, &owner_sk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::TOTAL_SHARES_EXCEEDS_MAX);
        assert_eq!(err.field(), Some("total_shares"));
    }

    #[test]
    fn test_share_validator_threshold_exceeds_total() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 6, 5, &owner_sk, &requester_pk);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::THRESHOLD_EXCEEDS_TOTAL);
        assert_eq!(err.field(), Some("threshold"));
    }

    #[test]
    fn test_share_validator_valid_params() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret data", 3, 5, &owner_sk, &requester_pk);

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_validator_min_valid_threshold() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", MIN_THRESHOLD, 5, &owner_sk, &requester_pk);

        assert!(result.is_ok());
    }

    #[test]
    fn test_share_validator_max_valid_shares() {
        let validator = ShareValidator::new();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, MAX_SHARES, &owner_sk, &requester_pk);

        assert!(result.is_ok());
    }

    // ========================================================================
    // RecoverValidator Tests
    // ========================================================================

    #[test]
    fn test_recover_validator_empty_secret_id() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();

        let result = validator.validate("", &requester_sk, "process_123");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_SECRET_ID);
        assert_eq!(err.field(), Some("secret_id"));
    }

    #[test]
    fn test_recover_validator_empty_process_id() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();

        let result = validator.validate("secret_abc123", &requester_sk, "");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code(), error_codes::INVALID_PROCESS_ID);
        assert_eq!(err.field(), Some("requester_process_id"));
    }

    #[test]
    fn test_recover_validator_valid_params() {
        let validator = RecoverValidator::new();
        let (requester_sk, _) = create_test_keys();

        let result = validator.validate("secret_abc123", &requester_sk, "process_123");

        assert!(result.is_ok());
    }

    // ========================================================================
    // Default Trait Tests
    // ========================================================================

    #[test]
    fn test_share_validator_default() {
        let validator: ShareValidator = Default::default();
        let (owner_sk, _) = create_test_keys();
        let (_, requester_pk) = create_test_keys();

        let result = validator.validate(b"secret", 3, 5, &owner_sk, &requester_pk);
        assert!(result.is_ok());
    }

    #[test]
    fn test_recover_validator_default() {
        let validator: RecoverValidator = Default::default();
        let (requester_sk, _) = create_test_keys();

        let result = validator.validate("secret_id", &requester_sk, "process_id");
        assert!(result.is_ok());
    }
}
