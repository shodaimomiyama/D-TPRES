//! Controller layer validators for input validation
//!
//! Provides early validation before DTO construction to fail fast on invalid inputs.

use std::sync::Arc;

use subtle::ConstantTimeEq;

use crate::controller::error::{error_codes, ValidationError, MAX_SHARES, MIN_THRESHOLD};
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
