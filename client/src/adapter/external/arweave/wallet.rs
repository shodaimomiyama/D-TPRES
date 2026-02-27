//! Arweave wallet implementation
//!
//! Provides wallet abstraction for Arweave transaction signing.

#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

use std::fmt;

use rsa::pss::{BlindedSigningKey, Signature};
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::{BigUint, RsaPrivateKey};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::adapter::errors::AdapterError;
use crate::adapter::external::arweave::client::base64url_encode;

/// Arweave wallet for transaction signing
///
/// Secret JWK components (d, p, q) are zeroized on drop to prevent
/// private key material from lingering in memory.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ArweaveWallet {
    #[zeroize(skip)]
    public_key_bytes: Vec<u8>,
    #[zeroize(skip)]
    public_exp_bytes: Vec<u8>,
    private_exp_bytes: Vec<u8>,
    prime_p_bytes: Vec<u8>,
    prime_q_bytes: Vec<u8>,
}

impl fmt::Debug for ArweaveWallet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArweaveWallet")
            .field(
                "public_key_bytes",
                &format!("[{} bytes]", self.public_key_bytes.len()),
            )
            .field(
                "public_exp_bytes",
                &format!("[{} bytes]", self.public_exp_bytes.len()),
            )
            .field("private_exp_bytes", &"[REDACTED]")
            .field("prime_p_bytes", &"[REDACTED]")
            .field("prime_q_bytes", &"[REDACTED]")
            .finish()
    }
}

impl ArweaveWallet {
    /// Create wallet from JWK JSON value
    ///
    /// Parses all RSA components into byte vectors. The `serde_json::Value`
    /// is not retained so that secret material lives only in zeroizable fields.
    #[allow(clippy::needless_pass_by_value)]
    pub fn from_jwk(jwk: serde_json::Value) -> Result<Self, AdapterError> {
        let kty = jwk
            .get("kty")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AdapterError::configuration_error("wallet", "Missing 'kty' in JWK"))?;

        if kty != "RSA" {
            return Err(AdapterError::configuration_error(
                "wallet",
                &format!("Invalid key type: expected 'RSA', got '{kty}'"),
            ));
        }

        let public_key_bytes = decode_jwk_field(&jwk, "n", "modulus")?;
        let public_exp_bytes = decode_jwk_field(&jwk, "e", "public exponent")?;
        let private_exp_bytes = decode_jwk_field(&jwk, "d", "private exponent")?;
        let prime_p_bytes = decode_jwk_field(&jwk, "p", "prime p")?;
        let prime_q_bytes = decode_jwk_field(&jwk, "q", "prime q")?;

        Ok(Self {
            public_key_bytes,
            public_exp_bytes,
            private_exp_bytes,
            prime_p_bytes,
            prime_q_bytes,
        })
    }

    /// Create wallet from ARWEAVE_WALLET_PATH environment variable
    ///
    /// The environment variable should contain a path to a JWK JSON file.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_env() -> Result<Self, AdapterError> {
        let wallet_path = env::var("ARWEAVE_WALLET_PATH").map_err(|_| {
            AdapterError::configuration_error(
                "wallet",
                "ARWEAVE_WALLET_PATH environment variable not set",
            )
        })?;

        Self::from_file(&wallet_path)
    }

    /// Create wallet from a JWK file path
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, AdapterError> {
        let path = path.as_ref();

        let jwk_str = fs::read_to_string(path).map_err(|e| {
            AdapterError::configuration_error(
                "wallet",
                &format!("Failed to read wallet file '{}': {e}", path.display()),
            )
        })?;

        let jwk: serde_json::Value = serde_json::from_str(&jwk_str).map_err(|e| {
            AdapterError::configuration_error(
                "wallet",
                &format!("Invalid JSON in wallet file '{}': {e}", path.display()),
            )
        })?;

        Self::from_jwk(jwk)
    }

    /// Get wallet address (derived from SHA-256 hash of public key)
    pub fn address(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.public_key_bytes);
        let hash = hasher.finalize();
        base64url_encode(&hash)
    }

    /// Get the owner (public key) in Base64URL format
    pub fn owner(&self) -> String {
        base64url_encode(&self.public_key_bytes)
    }

    /// Sign message using RSA-PSS with SHA-256
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, AdapterError> {
        let private_key = self.build_rsa_private_key()?;

        let signing_key: BlindedSigningKey<Sha256> = BlindedSigningKey::new(private_key);

        let mut rng = rand::thread_rng();
        let signature: Signature = signing_key.sign_with_rng(&mut rng, message);

        Ok(signature.to_vec())
    }

    fn build_rsa_private_key(&self) -> Result<RsaPrivateKey, AdapterError> {
        let modulus = BigUint::from_bytes_be(&self.public_key_bytes);
        let public_exp = BigUint::from_bytes_be(&self.public_exp_bytes);
        let private_exp = BigUint::from_bytes_be(&self.private_exp_bytes);
        let prime_p = BigUint::from_bytes_be(&self.prime_p_bytes);
        let prime_q = BigUint::from_bytes_be(&self.prime_q_bytes);

        RsaPrivateKey::from_components(modulus, public_exp, private_exp, vec![prime_p, prime_q])
            .map_err(|err| {
                AdapterError::configuration_error(
                    "wallet",
                    &format!("Failed to construct RSA private key: {err}"),
                )
            })
    }
}

fn decode_jwk_field(
    jwk: &serde_json::Value,
    key: &str,
    name: &str,
) -> Result<Vec<u8>, AdapterError> {
    let value = jwk.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
        AdapterError::configuration_error(
            "wallet",
            &format!("Missing JWK component '{key}' ({name})"),
        )
    })?;

    base64url_decode_internal(value).map_err(|e| {
        AdapterError::configuration_error(
            "wallet",
            &format!("Invalid Base64URL in JWK component '{key}': {e}"),
        )
    })
}

fn base64url_decode_internal(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_jwk() -> serde_json::Value {
        json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB",
            "d": "X4cTteJY_gn4FYPsXB8rdXix5vwsg1FLN5E3EaG6RJoVH-HLLKD9M7dx5oo7GURknchnrRweUkC7hT5fJLM0WbFAKNLWY2vv7B6NqXSzUvxT0_YSfqijwp3RTzlBaCxWp4doFk5N2o8Gy_nHNKroADIkJ46pRUohsXywbReAdYaMwFs9tv8d_cPVY3i07a3t8MN6TNwm0dSawm9v47UiCl3Sk5ZiG7xojPLu4sbg1U2jx4IBTNBznbJSzFHK66jT8bgkuqsk0GjskDJk19Z4qwjwbsnn4j2WBii3RL-Us2lGVkY8fkFzme1z0HbIkfz0Y6mqnOYtqc0X4jfcKoAC8Q",
            "p": "83i-7IvMGXoMXCskv73TKr8637FiO7Z27zv8oj6pbWUQyLPQBQxtPVnwD20R-60eTDmD2ujnMt5PoqMrm8RfmNhVWDtjjMmCMjOpSXicFHj7XOuVIYQyqVWlWEh6dN36GVZYk93N8Bc9vY41xy8B9RzzOGVQzXvNEvn7O0nVbfs",
            "q": "3dfOR9cuYq-0S-mkFLzgItgMEfFzB2q3hWehMuG0oCuqnb3vobLyumqjb37qSxPODCQt1yY0EHTy6EaJ2sG3-xLLlRqfvPyM7AqZAVzu9NMs0F-4V3OBJzuVxhqkzNjgQCc7Nh9rEGlZyQOXLFHGXUsnJHJCDYUzz7LxPD7pzKM"
        })
    }

    #[test]
    fn test_wallet_from_jwk_valid() {
        let jwk = create_test_jwk();

        let wallet = ArweaveWallet::from_jwk(jwk).unwrap();
        let address = wallet.address();
        assert!(!address.is_empty());
    }

    #[test]
    fn test_wallet_from_jwk_missing_kty() {
        let jwk = json!({
            "n": "test",
            "e": "AQAB"
        });

        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_from_jwk_invalid_kty() {
        let jwk = json!({
            "kty": "EC",
            "n": "test",
            "e": "AQAB"
        });

        let result = ArweaveWallet::from_jwk(jwk);
        assert!(result.is_err());
    }
}
