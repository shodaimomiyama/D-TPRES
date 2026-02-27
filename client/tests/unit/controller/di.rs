use std::sync::Arc;

use formix::controller::ControllerContainer;
use formix::domain::SecretId;
use formix::usecase::core::crypto::{CryptoService, CryptoServiceImpl};
use zeroize::Zeroizing;

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

#[test]
fn test_container_new() {
    let crypto_service = create_crypto_service();
    let container = ControllerContainer::new(crypto_service);
    let (owner_sk, owner_pk) = create_test_keys();
    let (_, requester_pk) = create_test_keys();

    let result =
        container
            .share_validator()
            .validate(b"secret", 3, 5, &owner_sk, &owner_pk, &requester_pk);
    assert!(result.is_ok());
}

#[test]
fn test_container_share_validator() {
    let crypto_service = create_crypto_service();
    let container = ControllerContainer::new(crypto_service);
    let (owner_sk, owner_pk) = create_test_keys();
    let (_, requester_pk) = create_test_keys();

    let result =
        container
            .share_validator()
            .validate(b"secret", 0, 5, &owner_sk, &owner_pk, &requester_pk);
    assert!(result.is_err());
}

#[test]
fn test_container_recover_validator() {
    let crypto_service = create_crypto_service();
    let container = ControllerContainer::new(crypto_service);
    let (requester_sk, _) = create_test_keys();
    let (_, owner_pk) = create_test_keys();

    let result =
        container
            .recover_validator()
            .validate("secret_id", &requester_sk, &owner_pk, "process_id");
    assert!(result.is_ok());

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
        Zeroizing::new(b"secret".to_vec()),
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
    let secret_id = SecretId::new("test_secret");

    let request = container.recover_extractor().extract(
        secret_id,
        requester_sk,
        owner_pk,
        "requester_process".to_string(),
    );

    assert_eq!(request.secret_id.as_str(), "test_secret");
    assert_eq!(request.requester_process_id, "requester_process");
}
