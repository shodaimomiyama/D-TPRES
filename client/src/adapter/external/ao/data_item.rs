//! ANS-104 DataItem builder and signer for HyperBEAM AO.
//!
//! Port of `ao_cwao/data_item.rs` with updated message types.
//! Core changes:
//! - `ExecuteMsg` / `QueryMsg` → `AOExecuteMsg` / `AOQueryMsg`
//! - `action` read via `.action()` accessor (private field)
//!
//! Signing and ANS-104 encoding are IDENTICAL to ao_cwao — do not diverge.
//! Specifically:
//!   - Deep-hash (SHA-384 recursive tree) is preserved exactly
//!   - Tags are Avro-encoded (zigzag varint + block encoding)
//!   - RSA signature / owner padding is 512-byte zero-padded

#![allow(clippy::disallowed_names)]

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::rngs::OsRng;
use rsa::pss::SigningKey;
use rsa::signature::RandomizedSigner;
use rsa::signature::SignatureEncoding;
use rsa::{BigUint, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha384};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::message::{AOExecuteMsg, AOQueryMsg};
use crate::adapter::errors::AOCommunicationError;

// AO protocol constants (unchanged)
const DATA_PROTOCOL: &str = "ao";
const VARIANT: &str = "ao.TN.1";
const MESSAGE_TYPE: &str = "Message";
const SDK: &str = "ao";

// ANS-104 constants (must match ao_cwao)
const SIG_TYPE_RSA256: u16 = 1;
const RSA_SIG_LENGTH: usize = 512;
const RSA_OWNER_LENGTH: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataItemTag {
    pub name: String,
    pub value: String,
}
impl DataItemTag {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UnsignedDataItem {
    pub target: Vec<u8>,
    pub anchor: Vec<u8>,
    pub tags: Vec<DataItemTag>,
    pub data: Vec<u8>,
}

/// Builds AO-compatible DataItems from `AOExecuteMsg` / `AOQueryMsg`.
pub struct DataItemBuilder;

impl DataItemBuilder {
    /// Build an unsigned DataItem from `AOExecuteMsg` for MU submission.
    pub fn build_execute(
        target: &str,
        msg: &AOExecuteMsg,
    ) -> Result<UnsignedDataItem, AOCommunicationError> {
        let data =
            serde_json::to_vec(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize AOExecuteMsg: {e}"),
            })?;
        let target_bytes = Self::decode_target(target)?;
        Ok(UnsignedDataItem {
            target: target_bytes,
            anchor: Vec::new(),
            tags: Self::ao_tags(target, msg.action()),
            data,
        })
    }

    /// Build a JSON body for CU dry-run (AOExecuteMsg).
    pub fn build_dry_run_body(
        target: &str,
        msg: &AOExecuteMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let data =
            serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize AOExecuteMsg: {e}"),
            })?;
        Ok(Self::dry_run_json(target, msg.action(), &data))
    }

    /// Build a JSON body for CU dry-run (AOQueryMsg).
    pub fn build_query_body(
        target: &str,
        msg: &AOQueryMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let data =
            serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize AOQueryMsg: {e}"),
            })?;
        Ok(Self::dry_run_json(target, msg.action(), &data))
    }

    fn decode_target(target: &str) -> Result<Vec<u8>, AOCommunicationError> {
        let bytes =
            URL_SAFE_NO_PAD
                .decode(target)
                .map_err(|e| AOCommunicationError::ValidationError {
                    details: format!("Invalid base64url target: {e}"),
                })?;
        if bytes.len() != 32 {
            return Err(AOCommunicationError::ValidationError {
                details: format!("Target must be 32 bytes (got {})", bytes.len()),
            });
        }
        Ok(bytes)
    }

    fn ao_tags(target: &str, action: &str) -> Vec<DataItemTag> {
        vec![
            DataItemTag::new("Data-Protocol", DATA_PROTOCOL),
            DataItemTag::new("Variant", VARIANT),
            DataItemTag::new("Type", MESSAGE_TYPE),
            DataItemTag::new("SDK", SDK),
            DataItemTag::new("Action", action),
            DataItemTag::new("Target", target),
        ]
    }

    fn dry_run_json(target: &str, action: &str, data: &str) -> serde_json::Value {
        #[derive(Serialize)]
        struct DryRunTag {
            name: String,
            value: String,
        }
        let tags: Vec<DryRunTag> = vec![
            DryRunTag {
                name: "Data-Protocol".into(),
                value: DATA_PROTOCOL.into(),
            },
            DryRunTag {
                name: "Variant".into(),
                value: VARIANT.into(),
            },
            DryRunTag {
                name: "Type".into(),
                value: MESSAGE_TYPE.into(),
            },
            DryRunTag {
                name: "SDK".into(),
                value: SDK.into(),
            },
            DryRunTag {
                name: "Action".into(),
                value: action.into(),
            },
        ];
        serde_json::json!({ "Target": target, "Tags": tags, "Data": data })
    }
}

// ─── ArweaveJWK ───────────────────────────────────────────────────────────────

/// Arweave JWK (JSON Web Key) for RSA signing.
///
/// Secret fields are zeroized on drop to prevent private key material from
/// lingering in memory.
///
/// `p/q/dp/dq/qi` have `#[serde(default)]` to accept minimal JWKs (e.g. public-key only
/// exports or JWKs where CRT components are absent).
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
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

// ─── DataItemSigner ───────────────────────────────────────────────────────────
// Identical to ao_cwao/data_item.rs — must not diverge.

/// Signs UnsignedDataItems with an Arweave RSA key using ANS-104 format.
///
/// Wire format details:
/// - Signature: RSA-PSS-SHA256, 512-byte zero-padded
/// - Owner: RSA modulus (n), 512-byte zero-padded
/// - Tags: Avro block encoding (zigzag varint lengths)
/// - Hash: ANS-104 deep-hash (SHA-384 recursive tree)
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DataItemSigner {
    #[zeroize(skip)]
    signing_key: SigningKey<Sha256>,
    owner_bytes: Vec<u8>,
}

impl DataItemSigner {
    pub fn new(jwk: &ArweaveJWK) -> Result<Self, AOCommunicationError> {
        if jwk.kty != "RSA" {
            return Err(AOCommunicationError::ValidationError {
                details: "JWK kty must be RSA".to_string(),
            });
        }
        if jwk.n.is_empty() || jwk.e.is_empty() || jwk.d.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "JWK missing required RSA fields (n, e, d)".to_string(),
            });
        }

        let decode = |field: &str, name: &str| -> Result<BigUint, AOCommunicationError> {
            let bytes = URL_SAFE_NO_PAD.decode(field).map_err(|e| {
                AOCommunicationError::ValidationError {
                    details: format!("Invalid base64url in JWK {name}: {e}"),
                }
            })?;
            Ok(BigUint::from_bytes_be(&bytes))
        };

        let n = decode(&jwk.n, "n")?;
        let e = decode(&jwk.e, "e")?;
        let d = decode(&jwk.d, "d")?;

        let mut primes = Vec::new();
        if !jwk.p.is_empty() && !jwk.q.is_empty() {
            primes.push(decode(&jwk.p, "p")?);
            primes.push(decode(&jwk.q, "q")?);
        }

        let private_key = RsaPrivateKey::from_components(n, e, d, primes).map_err(|e| {
            AOCommunicationError::ValidationError {
                details: format!("Invalid RSA key: {e}"),
            }
        })?;

        let owner_bytes =
            URL_SAFE_NO_PAD
                .decode(&jwk.n)
                .map_err(|e| AOCommunicationError::ValidationError {
                    details: format!("Invalid base64url in JWK n: {e}"),
                })?;
        if owner_bytes.len() != RSA_OWNER_LENGTH {
            return Err(AOCommunicationError::ValidationError {
                details: format!(
                    "Invalid RSA modulus length: expected {} bytes, got {}",
                    RSA_OWNER_LENGTH,
                    owner_bytes.len()
                ),
            });
        }

        Ok(Self {
            signing_key: SigningKey::<Sha256>::new(private_key),
            owner_bytes,
        })
    }

    /// Owner address: `base64url(SHA-256(owner_public_key_bytes))`
    pub fn owner_address(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(&self.owner_bytes))
    }

    /// Sign a DataItem and return complete ANS-104 signed bytes.
    pub fn sign(&self, item: &UnsignedDataItem) -> Result<Vec<u8>, AOCommunicationError> {
        let deep_hash = self.build_deep_hash(item);
        let signature = self.signing_key.sign_with_rng(&mut OsRng, &deep_hash);
        let sig_bytes = signature.to_bytes();

        let mut result = Vec::new();

        // Signature type (2 bytes, little-endian)
        result.extend_from_slice(&SIG_TYPE_RSA256.to_le_bytes());

        // Signature (512 bytes, zero-padded to the right)
        let mut sig_padded = vec![0u8; RSA_SIG_LENGTH];
        let sig_vec = sig_bytes.to_vec();
        let start = RSA_SIG_LENGTH.saturating_sub(sig_vec.len());
        sig_padded[start..].copy_from_slice(&sig_vec);
        result.extend_from_slice(&sig_padded);

        // Owner (512 bytes, zero-padded)
        let mut owner_padded = vec![0u8; RSA_OWNER_LENGTH];
        let start = RSA_OWNER_LENGTH.saturating_sub(self.owner_bytes.len());
        owner_padded[start..].copy_from_slice(&self.owner_bytes);
        result.extend_from_slice(&owner_padded);

        // Target present flag + bytes (32 bytes)
        if item.target.is_empty() {
            result.push(0);
        } else {
            result.push(1);
            result.extend_from_slice(&item.target);
        }

        // Anchor present flag + bytes (32 bytes)
        if item.anchor.is_empty() {
            result.push(0);
        } else {
            result.push(1);
            result.extend_from_slice(&item.anchor);
        }

        // Tags: num_tags (u64 LE) + tags_bytes_len (u64 LE) + Avro-encoded tags
        let tags_bytes = self.serialize_avro_tags(&item.tags);
        let num_tags = item.tags.len() as u64;
        result.extend_from_slice(&num_tags.to_le_bytes());
        result.extend_from_slice(&(tags_bytes.len() as u64).to_le_bytes());
        result.extend_from_slice(&tags_bytes);

        // Data
        result.extend_from_slice(&item.data);

        Ok(result)
    }

    // ─── ANS-104 deep-hash ────────────────────────────────────────────────────
    // Identical to ao_cwao — SHA-384 recursive tree hash.
    // Reference: https://github.com/ArweaveTeam/arweave-standards/blob/master/ans/ANS-104.md

    fn build_deep_hash(&self, item: &UnsignedDataItem) -> Vec<u8> {
        let tags_bytes = self.serialize_avro_tags(&item.tags);
        let sig_type_str = SIG_TYPE_RSA256.to_string();

        let parts: Vec<&[u8]> = vec![
            b"dataitem",
            b"1",
            sig_type_str.as_bytes(),
            &self.owner_bytes,
            &item.target,
            &item.anchor,
            &tags_bytes,
            &item.data,
        ];

        let hashed_parts: Vec<[u8; 48]> = parts.iter().map(|p| Self::deep_hash_blob(p)).collect();
        Self::deep_hash_list(&hashed_parts).to_vec()
    }

    fn deep_hash_blob(data: &[u8]) -> [u8; 48] {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"blob");
        tag.extend_from_slice(data.len().to_string().as_bytes());

        let tag_hash = Sha384::digest(&tag);
        let data_hash = Sha384::digest(data);

        let mut combined = Vec::with_capacity(96);
        combined.extend_from_slice(&tag_hash);
        combined.extend_from_slice(&data_hash);
        Sha384::digest(&combined).into()
    }

    fn deep_hash_list(items: &[[u8; 48]]) -> [u8; 48] {
        let mut tag = Vec::new();
        tag.extend_from_slice(b"list");
        tag.extend_from_slice(items.len().to_string().as_bytes());

        let mut acc: [u8; 48] = Sha384::digest(&tag).into();
        for item in items {
            let mut combined = Vec::with_capacity(96);
            combined.extend_from_slice(&acc);
            combined.extend_from_slice(item);
            acc = Sha384::digest(&combined).into();
        }
        acc
    }

    // ─── Avro tag encoding ────────────────────────────────────────────────────
    // ANS-104 tags use Avro object container format (zigzag varint lengths).
    // Identical to ao_cwao — must not diverge.

    fn serialize_avro_tags(&self, tags: &[DataItemTag]) -> Vec<u8> {
        if tags.is_empty() {
            return Vec::new();
        }
        let mut buf = Vec::new();
        #[allow(clippy::cast_possible_wrap)]
        Self::avro_encode_long(&mut buf, tags.len() as i64);
        for tag in tags {
            let name_bytes = tag.name.as_bytes();
            let value_bytes = tag.value.as_bytes();
            #[allow(clippy::cast_possible_wrap)]
            Self::avro_encode_long(&mut buf, name_bytes.len() as i64);
            buf.extend_from_slice(name_bytes);
            #[allow(clippy::cast_possible_wrap)]
            Self::avro_encode_long(&mut buf, value_bytes.len() as i64);
            buf.extend_from_slice(value_bytes);
        }
        Self::avro_encode_long(&mut buf, 0); // end-of-block marker
        buf
    }

    fn avro_encode_long(buf: &mut Vec<u8>, val: i64) {
        // Zigzag encoding
        #[allow(clippy::cast_sign_loss)]
        let mut v = ((val << 1) ^ (val >> 63)) as u64;
        loop {
            if v & !0x7F == 0 {
                buf.push(v as u8);
                break;
            }
            buf.push((v as u8 & 0x7F) | 0x80);
            v >>= 7;
        }
    }
}
