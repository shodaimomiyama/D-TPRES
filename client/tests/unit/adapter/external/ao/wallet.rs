use formix::adapter::external::ao::wallet::ArweaveJWK;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::RsaPrivateKey;

fn generate_test_jwk() -> ArweaveJWK {
    let private_key = RsaPrivateKey::new(&mut OsRng, 2048).unwrap();
    let public_key = private_key.to_public_key();
    let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
    let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
    let d = URL_SAFE_NO_PAD.encode(private_key.d().to_bytes_be());
    let primes = private_key.primes();
    let p = URL_SAFE_NO_PAD.encode(primes[0].to_bytes_be());
    let q = URL_SAFE_NO_PAD.encode(primes[1].to_bytes_be());

    ArweaveJWK {
        kty: "RSA".to_string(),
        n,
        e,
        d,
        p,
        q,
        dp: String::new(),
        dq: String::new(),
        qi: String::new(),
    }
}

#[test]
fn test_to_rsa_private_key_roundtrip() {
    let jwk = generate_test_jwk();
    let rsa_key = jwk
        .to_rsa_private_key()
        .expect("should convert to RsaPrivateKey");
    assert!(rsa_key.validate().is_ok());
}

#[test]
fn test_address_is_43_chars_base64url() {
    let jwk = generate_test_jwk();
    let address = jwk.address();
    assert_eq!(
        address.len(),
        43,
        "Arweave address should be 43 chars base64url"
    );
    assert!(address
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
}

#[test]
fn test_sig_name_format() {
    let jwk = generate_test_jwk();
    let sig_name = jwk.sig_name();
    assert!(
        sig_name.starts_with("http-sig-"),
        "sig_name should start with 'http-sig-'"
    );
    assert_eq!(
        sig_name.len(),
        "http-sig-".len() + 16,
        "sig_name hex part should be 16 chars (8 bytes)"
    );
}

#[test]
fn test_address_deterministic() {
    let jwk = generate_test_jwk();
    let addr1 = jwk.address();
    let addr2 = jwk.address();
    assert_eq!(addr1, addr2);
}
