use formix::domain::KeyPair;

#[test]
fn test_key_pair_new() {
    let sk = vec![1u8; 32];
    let pk = vec![2u8; 33];

    let pair = KeyPair::new(sk.clone(), pk.clone());

    assert_eq!(pair.secret_key(), sk.as_slice());
    assert_eq!(pair.public_key(), pk.as_slice());
}

#[test]
fn test_key_pair_debug_redacted() {
    let pair = KeyPair::new(vec![1u8; 32], vec![2u8; 33]);

    let debug_str = format!("{:?}", pair);
    assert!(debug_str.contains("REDACTED"));
    assert!(!debug_str.contains("[1, 1, 1"));
}
