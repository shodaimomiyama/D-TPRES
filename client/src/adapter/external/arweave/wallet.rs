//! Arweave wallet implementation
//!
//! Provides wallet abstraction for Arweave transaction signing.

use std::env;
use std::fs;
use std::path::Path;

use rsa::pss::{BlindedSigningKey, Signature};
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use rsa::{BigUint, RsaPrivateKey};
use sha2::{Digest, Sha256};

use crate::adapter::errors::AdapterError;
use crate::adapter::external::arweave::client::base64url_encode;

/// Arweave wallet for transaction signing
#[derive(Debug)]
pub struct ArweaveWallet {
    jwk: serde_json::Value,
    public_key_bytes: Vec<u8>,
}

impl ArweaveWallet {
    /// Create wallet from JWK JSON value
    pub fn from_jwk(jwk: serde_json::Value) -> Result<Self, AdapterError> {
        // Validate JWK structure
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

        let n = jwk.get("n").and_then(|v| v.as_str()).ok_or_else(|| {
            AdapterError::configuration_error("wallet", "Missing 'n' (modulus) in JWK")
        })?;

        // Decode public key modulus
        let public_key_bytes = base64url_decode_internal(n).map_err(|e| {
            AdapterError::configuration_error("wallet", &format!("Invalid Base64URL in 'n': {e}"))
        })?;

        Ok(Self {
            jwk,
            public_key_bytes,
        })
    }

    /// Create wallet from ARWEAVE_WALLET_PATH environment variable
    ///
    /// The environment variable should contain a path to a JWK JSON file.
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

    /// Get the JWK value
    pub fn jwk(&self) -> &serde_json::Value {
        &self.jwk
    }

    /// Sign message using RSA-PSS with SHA-256
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, AdapterError> {
        let private_key = self.build_rsa_private_key()?;

        let signing_key: BlindedSigningKey<Sha256> = BlindedSigningKey::new(private_key);

        let mut rng = rand::thread_rng();
        let signature: Signature = signing_key.sign_with_rng(&mut rng, message);

        Ok(signature.to_vec())
    }

    /// Build RSA private key from JWK components
    fn build_rsa_private_key(&self) -> Result<RsaPrivateKey, AdapterError> {
        let modulus = self.decode_jwk_component("n", "modulus")?;
        let public_exp = self.decode_jwk_component("e", "public exponent")?;
        let private_exp = self.decode_jwk_component("d", "private exponent")?;
        let prime_p = self.decode_jwk_component("p", "prime p")?;
        let prime_q = self.decode_jwk_component("q", "prime q")?;

        RsaPrivateKey::from_components(modulus, public_exp, private_exp, vec![prime_p, prime_q])
            .map_err(|err| {
                AdapterError::configuration_error(
                    "wallet",
                    &format!("Failed to construct RSA private key: {err}"),
                )
            })
    }

    /// Decode a JWK component from Base64URL to BigUint
    fn decode_jwk_component(&self, key: &str, name: &str) -> Result<BigUint, AdapterError> {
        let value = self.jwk.get(key).and_then(|v| v.as_str()).ok_or_else(|| {
            AdapterError::configuration_error(
                "wallet",
                &format!("Missing JWK component '{key}' ({name})"),
            )
        })?;

        let bytes = base64url_decode_internal(value).map_err(|e| {
            AdapterError::configuration_error(
                "wallet",
                &format!("Invalid Base64URL in JWK component '{key}': {e}"),
            )
        })?;

        Ok(BigUint::from_bytes_be(&bytes))
    }
}

/// Decode Base64URL (internal helper to avoid circular dependency)
fn base64url_decode_internal(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_wallet_from_jwk_valid() {
        // Minimal valid RSA JWK for testing
        let jwk = json!({
            "kty": "RSA",
            "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
            "e": "AQAB"
        });

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
