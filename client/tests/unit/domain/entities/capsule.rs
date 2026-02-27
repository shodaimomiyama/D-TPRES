use formix::domain::{Capsule, DomainError, SecretId};

#[test]
fn test_capsule_new_valid() {
    let secret_id = SecretId::generate();
    let capsule_data = vec![1u8; 100];
    let owner_pk = vec![2u8; 33];

    let capsule = Capsule::new(secret_id.clone(), capsule_data.clone(), owner_pk.clone()).unwrap();

    assert_eq!(capsule.secret_id(), &secret_id);
    assert_eq!(capsule.capsule_data(), capsule_data.as_slice());
    assert_eq!(capsule.owner_public_key(), owner_pk.as_slice());
    assert!(capsule.arweave_tx_id().is_none());
}

#[test]
fn test_capsule_empty_data_error() {
    let secret_id = SecretId::generate();
    let result = Capsule::new(secret_id, vec![], vec![1u8; 33]);

    assert!(result.is_err());
    if let Err(DomainError::EntityValidation { field, .. }) = result {
        assert_eq!(field, "capsule_data");
    } else {
        panic!("Expected EntityValidation error");
    }
}

#[test]
fn test_capsule_empty_public_key_error() {
    let secret_id = SecretId::generate();
    let result = Capsule::new(secret_id, vec![1u8; 100], vec![]);

    assert!(result.is_err());
    if let Err(DomainError::EntityValidation { field, .. }) = result {
        assert_eq!(field, "owner_public_key");
    } else {
        panic!("Expected EntityValidation error");
    }
}

#[test]
fn test_capsule_set_arweave_tx_id() {
    let secret_id = SecretId::generate();
    let mut capsule = Capsule::new(secret_id, vec![1u8; 100], vec![2u8; 33]).unwrap();

    assert!(capsule.arweave_tx_id().is_none());

    capsule.set_arweave_tx_id("tx-456".to_string());

    assert_eq!(capsule.arweave_tx_id(), Some("tx-456"));
}
