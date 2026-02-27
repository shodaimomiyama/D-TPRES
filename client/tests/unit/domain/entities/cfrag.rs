use formix::domain::{CFrag, DomainError, KFragId, SecretId};

#[test]
fn test_cfrag_new_valid() {
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();
    let cfrag_data = vec![1u8; 150];

    let cfrag = CFrag::new(secret_id.clone(), kfrag_id.clone(), 1, cfrag_data.clone()).unwrap();

    assert_eq!(cfrag.secret_id(), &secret_id);
    assert_eq!(cfrag.kfrag_id(), &kfrag_id);
    assert_eq!(cfrag.holder_index(), 1);
    assert_eq!(cfrag.cfrag_data(), cfrag_data.as_slice());
}

#[test]
fn test_cfrag_empty_data_error() {
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();

    let result = CFrag::new(secret_id, kfrag_id, 1, vec![]);

    assert!(result.is_err());
    if let Err(DomainError::EntityValidation { field, .. }) = result {
        assert_eq!(field, "cfrag_data");
    } else {
        panic!("Expected EntityValidation error");
    }
}

#[test]
fn test_cfrag_verify_valid() {
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();
    let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

    let capsule_data = vec![1u8; 50];
    let result = cfrag.verify(&capsule_data);

    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_cfrag_verify_empty_capsule_error() {
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();
    let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

    let result = cfrag.verify(&[]);

    assert!(result.is_err());
}

#[test]
fn test_cfrag_debug_redacted() {
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();
    let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

    let debug_str = format!("{:?}", cfrag);
    assert!(debug_str.contains("REDACTED"));
}
