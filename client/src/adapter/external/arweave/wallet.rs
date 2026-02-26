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
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.decode(encoded)
}
