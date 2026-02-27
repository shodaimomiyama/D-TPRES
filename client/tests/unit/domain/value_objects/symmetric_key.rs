use formix::domain::{SymmetricKey, SYMMETRIC_KEY_SIZE};

#[test]
fn test_symmetric_key_new() {
    let key_bytes = [1u8; SYMMETRIC_KEY_SIZE];
    let key = SymmetricKey::new(key_bytes);

    assert_eq!(key.as_bytes(), &key_bytes);
}

#[test]
fn test_symmetric_key_from_slice_valid() {
    let key_bytes = [2u8; SYMMETRIC_KEY_SIZE];
    let key = SymmetricKey::from_slice(&key_bytes).unwrap();

    assert_eq!(key.as_bytes(), &key_bytes);
}

#[test]
fn test_symmetric_key_from_slice_invalid_length() {
    let short = [1u8; 16];
    let long = [1u8; 64];

    assert!(SymmetricKey::from_slice(&short).is_none());
    assert!(SymmetricKey::from_slice(&long).is_none());
}

#[test]
fn test_symmetric_key_debug_redacted() {
    let key = SymmetricKey::new([1u8; SYMMETRIC_KEY_SIZE]);

    let debug_str = format!("{:?}", key);
    assert!(debug_str.contains("REDACTED"));
}
