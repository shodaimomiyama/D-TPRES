use formix::domain::{DomainError, SecretData};

#[test]
fn test_secret_data_new_valid() {
    let data = vec![1, 2, 3, 4, 5];
    let secret = SecretData::new(data).unwrap();
    assert_eq!(secret.as_bytes(), &[1, 2, 3, 4, 5]);
    assert_eq!(secret.len(), 5);
    assert!(!secret.is_empty());
}

#[test]
fn test_secret_data_new_empty_error() {
    let result = SecretData::new(vec![]);
    assert!(result.is_err());
    if let Err(DomainError::EntityValidation { field, .. }) = result {
        assert_eq!(field, "secret_bytes");
    } else {
        panic!("Expected EntityValidation error");
    }
}

#[test]
fn test_secret_data_debug_redacted() {
    let secret = SecretData::new(vec![1, 2, 3]).unwrap();
    let debug_str = format!("{:?}", secret);
    assert!(debug_str.contains("REDACTED"));
    assert!(!debug_str.contains("1"));
}
