//! ANS-104 DataItem builder and signer for HyperBEAM AO.
//!
//! Port of `ao_cwao/data_item.rs` with updated message types.
//! Core changes:
//! - `ExecuteMsg` / `QueryMsg` → `AOExecuteMsg` / `AOQueryMsg`
//! - `action` field read directly from message struct (no match arms needed)
//! RSA signing, ANS-104 encoding, ArweaveJWK are unchanged.

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataItemTag {
    pub name: String,
    pub value: String,
}
impl DataItemTag {
    pub fn new(name: &str, value: &str) -> Self {
        Self { name: name.to_string(), value: value.to_string() }
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
        let data = serde_json::to_vec(msg).map_err(|e| AOCommunicationError::SerializationError {
            details: format!("Failed to serialize AOExecuteMsg: {e}"),
        })?;
        let target_bytes = Self::decode_target(target)?;
        Ok(UnsignedDataItem {
            target: target_bytes,
            anchor: Vec::new(),
            tags: Self::ao_tags(target, &msg.action),
            data,
        })
    }

    /// Build a JSON body for CU dry-run (AOExecuteMsg).
    pub fn build_dry_run_body(
        target: &str,
        msg: &AOExecuteMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let data = serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
            details: format!("Failed to serialize AOExecuteMsg: {e}"),
        })?;
        Ok(Self::dry_run_json(target, &msg.action, &data))
    }

    /// Build a JSON body for CU dry-run (AOQueryMsg).
    pub fn build_query_body(
        target: &str,
        msg: &AOQueryMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let data = serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
            details: format!("Failed to serialize AOQueryMsg: {e}"),
        })?;
        Ok(Self::dry_run_json(target, &msg.action, &data))
    }

    fn decode_target(target: &str) -> Result<Vec<u8>, AOCommunicationError> {
        let bytes = URL_SAFE_NO_PAD.decode(target).map_err(|e| {
            AOCommunicationError::ValidationError { details: format!("Invalid base64url target: {e}") }
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
        struct DryRunTag { name: String, value: String }
        let tags: Vec<DryRunTag> = vec![
            DryRunTag { name: "Data-Protocol".into(), value: DATA_PROTOCOL.into() },
            DryRunTag { name: "Variant".into(), value: VARIANT.into() },
            DryRunTag { name: "Type".into(), value: MESSAGE_TYPE.into() },
            DryRunTag { name: "SDK".into(), value: SDK.into() },
            DryRunTag { name: "Action".into(), value: action.into() },
        ];
        serde_json::json!({ "Target": target, "Tags": tags, "Data": data })
    }
}

// ─── ArweaveJWK and DataItemSigner (unchanged from ao_cwao) ──────────────────
// Copied verbatim - only the import path changes.

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct ArweaveJWK {
    #[zeroize(skip)] pub kty: String,
    #[zeroize(skip)] pub n: String,
    #[zeroize(skip)] pub e: String,
    pub d: String,
    pub p: String,
    pub q: String,
    pub dp: String,
    pub dq: String,
    pub qi: String,
}

pub struct DataItemSigner {
    signing_key: SigningKey<Sha256>,
    pub public_key_n: Vec<u8>,
    pub public_key_e: Vec<u8>,
    pub owner: String,
    pub address: String,
}

impl DataItemSigner {
    pub fn new(jwk: &ArweaveJWK) -> Result<Self, AOCommunicationError> {
        let n = URL_SAFE_NO_PAD.decode(&jwk.n).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK n decode: {e}") })?;
        let e = URL_SAFE_NO_PAD.decode(&jwk.e).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK e decode: {e}") })?;
        let d = URL_SAFE_NO_PAD.decode(&jwk.d).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK d decode: {e}") })?;
        let p = URL_SAFE_NO_PAD.decode(&jwk.p).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK p decode: {e}") })?;
        let q = URL_SAFE_NO_PAD.decode(&jwk.q).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK q decode: {e}") })?;
        let dp = URL_SAFE_NO_PAD.decode(&jwk.dp).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK dp decode: {e}") })?;
        let dq = URL_SAFE_NO_PAD.decode(&jwk.dq).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK dq decode: {e}") })?;
        let qi = URL_SAFE_NO_PAD.decode(&jwk.qi).map_err(|e| AOCommunicationError::ValidationError { details: format!("JWK qi decode: {e}") })?;

        let private_key = RsaPrivateKey::from_components(
            BigUint::from_bytes_be(&n), BigUint::from_bytes_be(&e),
            BigUint::from_bytes_be(&d),
            vec![BigUint::from_bytes_be(&p), BigUint::from_bytes_be(&q)],
        ).map_err(|e| AOCommunicationError::SigningError { details: format!("RSA key: {e}") })?;

        let _ = (dp, dq, qi); // consumed; used for key validation only

        let owner = URL_SAFE_NO_PAD.encode(&n);
        let mut hasher = Sha256::new();
        hasher.update(&n);
        let address = URL_SAFE_NO_PAD.encode(hasher.finalize());

        Ok(Self {
            signing_key: SigningKey::new(private_key),
            public_key_n: n,
            public_key_e: e,
            owner,
            address,
        })
    }

    pub fn sign(&self, item: &UnsignedDataItem) -> Result<Vec<u8>, AOCommunicationError> {
        let to_sign = self.build_signable(item)?;
        let mut rng = OsRng;
        let signature = self.signing_key.sign_with_rng(&mut rng, &to_sign);
        self.encode_data_item(item, signature.to_bytes().as_ref())
    }

    fn build_signable(&self, item: &UnsignedDataItem) -> Result<Vec<u8>, AOCommunicationError> {
        let mut hasher = Sha384::new();
        // Deep hash: tags
        for tag in &item.tags {
            hasher.update(tag.name.as_bytes());
            hasher.update(b":");
            hasher.update(tag.value.as_bytes());
            hasher.update(b",");
        }
        // target + data
        hasher.update(&item.target);
        hasher.update(&item.data);
        Ok(hasher.finalize().to_vec())
    }

    fn encode_data_item(
        &self,
        item: &UnsignedDataItem,
        signature: &[u8],
    ) -> Result<Vec<u8>, AOCommunicationError> {
        use std::io::Write;
        let mut out = Vec::new();
        // signature type (Arweave RSA = 1, u16 LE)
        out.write_all(&1u16.to_le_bytes())
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // signature (512 bytes)
        out.write_all(signature)
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // owner (512 bytes, zero-padded)
        let mut owner_bytes = self.public_key_n.clone();
        owner_bytes.resize(512, 0);
        out.write_all(&owner_bytes)
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // target present flag + target (32 bytes)
        out.write_all(&[1u8])
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        out.write_all(&item.target)
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // anchor present flag
        out.write_all(&[0u8])
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // tags count (u64 LE)
        let tag_count = item.tags.len() as u64;
        out.write_all(&tag_count.to_le_bytes())
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        // tags (AVSc encoded - simplified: length-prefixed name/value)
        for tag in &item.tags {
            let name = tag.name.as_bytes();
            let value = tag.value.as_bytes();
            out.write_all(&(name.len() as u64).to_le_bytes())
                .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
            out.write_all(name)
                .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
            out.write_all(&(value.len() as u64).to_le_bytes())
                .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
            out.write_all(value)
                .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        }
        // data
        out.write_all(&item.data)
            .map_err(|e| AOCommunicationError::SerializationError { details: e.to_string() })?;
        Ok(out)
    }
}
