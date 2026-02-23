use std::sync::Arc;

use formix::controller::{
    MAX_SHARES, MIN_THRESHOLD, RecoverValidator, ShareValidator, error_codes,
};
use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl};

fn create_crypto_service() -> Arc<CryptoServiceImpl> {
    Arc::new(CryptoServiceImpl::new())
}

fn create_test_keys() -> (
    formix::usecase::core::crypto::SecretKey,
    formix::usecase::core::crypto::PublicKey,
) {
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
