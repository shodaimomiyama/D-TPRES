use formix::domain::{KFrag, SecretId};

#[test]
fn test_kfrag_new_valid() {
    let secret_id = SecretId::generate();
    let kfrag_data = vec![1u8; 200];

    let kfrag = KFrag::new(secret_id.clone(), 1, 3, kfrag_data.clone()).unwrap();

    assert_eq!(kfrag.secret_id(), &secret_id);
    assert_eq!(kfrag.holder_index(), 1);
    assert_eq!(kfrag.kfrag_data(), kfrag_data.as_slice());
    assert!(kfrag.holder_process_id().is_none());
}

#[test]
fn test_kfrag_invalid_index_zero() {
    let secret_id = SecretId::generate();
    let result = KFrag::new(secret_id, 0, 3, vec![1u8; 100]);

    assert!(result.is_err());
}

#[test]
fn test_kfrag_invalid_index_too_high() {
    let secret_id = SecretId::generate();
    let result = KFrag::new(secret_id, 4, 3, vec![1u8; 100]);

    assert!(result.is_err());
}

#[test]
fn test_kfrag_empty_data_error() {
    let secret_id = SecretId::generate();
    let result = KFrag::new(secret_id, 1, 3, vec![]);

    assert!(result.is_err());
}

#[test]
fn test_kfrag_set_holder_process_id() {
    let secret_id = SecretId::generate();
    let mut kfrag = KFrag::new(secret_id, 1, 3, vec![1u8; 100]).unwrap();

    assert!(kfrag.holder_process_id().is_none());

    kfrag.set_holder_process_id("holder-process-123".to_string());

    assert_eq!(kfrag.holder_process_id(), Some("holder-process-123"));
}

#[test]
fn test_kfrag_debug_redacted() {
    let secret_id = SecretId::generate();
    let kfrag = KFrag::new(secret_id, 1, 3, vec![1u8; 100]).unwrap();

    let debug_str = format!("{:?}", kfrag);
    assert!(debug_str.contains("REDACTED"));
}
