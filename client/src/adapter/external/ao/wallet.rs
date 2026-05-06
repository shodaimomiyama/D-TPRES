use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::adapter::errors::AOCommunicationError;

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ArweaveJWK {
    #[zeroize(skip)]
    pub kty: String,
    #[zeroize(skip)]
    pub n: String,
    #[zeroize(skip)]
    pub e: String,
    #[serde(default)]
    pub d: String,
    #[serde(default)]
    pub p: String,
    #[serde(default)]
    pub q: String,
    #[serde(default)]
    pub dp: String,
    #[serde(default)]
    pub dq: String,
    #[serde(default)]
    pub qi: String,
}

impl std::fmt::Debug for ArweaveJWK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArweaveJWK")
            .field("kty", &self.kty)
            .field("n", &format!("[{} chars]", self.n.len()))
            .field("e", &self.e)
            .field("d", &"[REDACTED]")
            .field("p", &"[REDACTED]")
            .field("q", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

impl ArweaveJWK {
    pub fn from_file(path: &str) -> Result<Self, AOCommunicationError> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            AOCommunicationError::wallet_error(format!("reading wallet file '{path}': {e}"))
        })?;
        serde_json::from_str(&contents)
            .map_err(|e| AOCommunicationError::wallet_error(format!("parsing JWK: {e}")))
    }

    pub fn to_rsa_private_key(&self) -> Result<RsaPrivateKey, AOCommunicationError> {
        let n = decode_biguint(&self.n)
            .map_err(|e| AOCommunicationError::wallet_error(format!("decoding n: {e}")))?;
        let e = decode_biguint(&self.e)
            .map_err(|e| AOCommunicationError::wallet_error(format!("decoding e: {e}")))?;
        let d = decode_biguint(&self.d)
            .map_err(|e| AOCommunicationError::wallet_error(format!("decoding d: {e}")))?;
        let p = decode_biguint(&self.p)
            .map_err(|e| AOCommunicationError::wallet_error(format!("decoding p: {e}")))?;
        let q = decode_biguint(&self.q)
            .map_err(|e| AOCommunicationError::wallet_error(format!("decoding q: {e}")))?;

        let primes = vec![p, q];
        RsaPrivateKey::from_components(n, e, d, primes).map_err(|e| {
            AOCommunicationError::wallet_error(format!("constructing RSA private key: {e}"))
        })
    }

    pub fn address(&self) -> Result<String, AOCommunicationError> {
        let n_bytes = URL_SAFE_NO_PAD.decode(&self.n).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding n for address: {e}"))
        })?;
        let hash = Sha256::digest(&n_bytes);
        Ok(URL_SAFE_NO_PAD.encode(hash))
    }

    pub fn sig_name(&self) -> Result<String, AOCommunicationError> {
        let address = self.address()?;
        let raw = URL_SAFE_NO_PAD.decode(&address).map_err(|e| {
            AOCommunicationError::wallet_error(format!("decoding address for sig_name: {e}"))
        })?;
        if raw.len() < 9 {
            return Err(AOCommunicationError::wallet_error(format!(
                "address hash too short ({} bytes, need 9)",
                raw.len()
            )));
        }
        let slice = &raw[1..9];
        let hex: String = slice.iter().map(|b| format!("{b:02x}")).collect();
        Ok(format!("http-sig-{hex}"))
    }
}

fn decode_biguint(b64: &str) -> Result<rsa::BigUint, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(b64)
        .map_err(|e| format!("base64url decode: {e}"))?;
    Ok(rsa::BigUint::from_bytes_be(&bytes))
}
