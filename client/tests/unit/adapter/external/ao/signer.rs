use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::rngs::OsRng;
use rsa::RsaPrivateKey;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use formix::adapter::external::ao::signer;

fn test_rsa_key() -> RsaPrivateKey {
    RsaPrivateKey::new(&mut OsRng, 2048).unwrap()
}

#[test]
fn test_sign_request_content_digest_is_sha256() {
    let key = test_rsa_key();
    let body = b"hello world";
    let signed = signer::sign_request(&key, body, "test-sig").unwrap();

    let expected_hash = Sha256::digest(body);
    let expected = format!("sha-256=:{}:", STANDARD.encode(expected_hash));
    assert_eq!(signed.content_digest_header, expected);
}

#[test]
fn test_sign_request_signature_header_format() {
    let key = test_rsa_key();
    let signed = signer::sign_request(&key, b"data", "my-sig").unwrap();

    assert!(signed.signature_header.starts_with("my-sig=:"));
    assert!(signed.signature_header.ends_with(':'));
}

#[test]
fn test_sign_request_signature_input_format() {
    let key = test_rsa_key();
    let signed = signer::sign_request(&key, b"data", "my-sig").unwrap();

    assert!(signed.signature_input_header.starts_with("my-sig="));
    assert!(signed
        .signature_input_header
        .contains("alg=\"rsa-pss-sha512\""));
    assert!(signed.signature_input_header.contains("created="));
    assert!(signed.signature_input_header.contains("keyid=\""));
    assert!(signed.signature_input_header.contains("\"content-digest\""));
}

#[test]
fn test_sign_message_covers_all_headers_sorted() {
    let key = test_rsa_key();
    let mut headers = BTreeMap::new();
    headers.insert("type".to_string(), "Message".to_string());
    headers.insert("action".to_string(), "Ping".to_string());

    let signed = signer::sign_message(&key, &headers, b"body", "test-sig").unwrap();

    let input = &signed.signature_input_header;
    let action_pos = input.find("\"action\"").expect("should contain action");
    let cd_pos = input
        .find("\"content-digest\"")
        .expect("should contain content-digest");
    let type_pos = input.find("\"type\"").expect("should contain type");
    assert!(
        action_pos < cd_pos,
        "action before content-digest (alphabetical)"
    );
    assert!(
        cd_pos < type_pos,
        "content-digest before type (alphabetical)"
    );
}

#[test]
fn test_sign_message_empty_body_no_content_digest() {
    let key = test_rsa_key();
    let mut headers = BTreeMap::new();
    headers.insert("action".to_string(), "Ping".to_string());

    let signed = signer::sign_message(&key, &headers, b"", "test-sig").unwrap();

    assert!(signed.content_digest_header.is_empty());
    assert!(!signed.signature_input_header.contains("content-digest"));
}
