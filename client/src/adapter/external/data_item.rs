use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rsa::pkcs1v15::SigningKey;
use rsa::signature::SignatureEncoding;
use rsa::signature::Signer;
use rsa::{BigUint, RsaPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::adapter::errors::AOCommunicationError;
use crate::adapter::external::ao_message::{ExecuteMsg, QueryMsg};

// AO protocol constants
const DATA_PROTOCOL: &str = "ao";
const VARIANT: &str = "ao.TN.1";
const MESSAGE_TYPE: &str = "Message";
const SDK: &str = "ao";

/// Tag attached to a DataItem (name-value pair)
#[derive(Debug, Clone, PartialEq)]
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

/// Unsigned DataItem ready for signing (ANS-104 format)
#[derive(Debug, Clone)]
pub struct UnsignedDataItem {
    pub target: Vec<u8>,
    pub anchor: Vec<u8>,
    pub tags: Vec<DataItemTag>,
    pub data: Vec<u8>,
}

/// Builds AO-compatible DataItems from ExecuteMsg/QueryMsg
pub struct DataItemBuilder;

impl DataItemBuilder {
    /// Build an unsigned DataItem from an ExecuteMsg for MU submission
    pub fn build_execute(
        target: &str,
        msg: &ExecuteMsg,
    ) -> Result<UnsignedDataItem, AOCommunicationError> {
        let action = Self::execute_msg_action(msg);
        let data =
            serde_json::to_vec(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize ExecuteMsg: {e}"),
            })?;

        let tags = Self::ao_tags(target, &action);

        Ok(UnsignedDataItem {
            target: target.as_bytes().to_vec(),
            anchor: Vec::new(),
            tags,
            data,
        })
    }

    /// Build a JSON body for CU dry-run API (ExecuteMsg)
    pub fn build_dry_run_body(
        target: &str,
        msg: &ExecuteMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let action = Self::execute_msg_action(msg);
        let data =
            serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize ExecuteMsg: {e}"),
            })?;

        Ok(Self::dry_run_json(target, &action, &data))
    }

    /// Build a JSON body for CU dry-run API (QueryMsg)
    pub fn build_query_body(
        target: &str,
        msg: &QueryMsg,
    ) -> Result<serde_json::Value, AOCommunicationError> {
        let action = Self::query_msg_action(msg);
        let data =
            serde_json::to_string(msg).map_err(|e| AOCommunicationError::SerializationError {
                details: format!("Failed to serialize QueryMsg: {e}"),
            })?;

        Ok(Self::dry_run_json(target, &action, &data))
    }

    fn execute_msg_action(msg: &ExecuteMsg) -> String {
        match msg {
            ExecuteMsg::DelegateKFrag { .. } => "DelegateKFrag".to_string(),
            ExecuteMsg::DelegateCapsule { .. } => "DelegateCapsule".to_string(),
            ExecuteMsg::SubmitKFrag { .. } => "SubmitKFrag".to_string(),
            ExecuteMsg::SubmitCapsule { .. } => "SubmitCapsule".to_string(),
            ExecuteMsg::Reencrypt { .. } => "Reencrypt".to_string(),
        }
    }

    fn query_msg_action(msg: &QueryMsg) -> String {
        match msg {
            QueryMsg::GetCFrag { .. } => "GetCFrag".to_string(),
            QueryMsg::ListCapsulesByKFrag { .. } => "ListCapsulesByKFrag".to_string(),
        }
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
                name: "Data-Protocol".to_string(),
                value: DATA_PROTOCOL.to_string(),
            },
            DryRunTag {
                name: "Variant".to_string(),
                value: VARIANT.to_string(),
            },
            DryRunTag {
                name: "Type".to_string(),
                value: MESSAGE_TYPE.to_string(),
            },
            DryRunTag {
                name: "SDK".to_string(),
                value: SDK.to_string(),
            },
            DryRunTag {
                name: "Action".to_string(),
                value: action.to_string(),
            },
        ];

        serde_json::json!({
            "Target": target,
            "Tags": tags,
            "Data": data,
        })
    }
}

/// Arweave JWK (JSON Web Key) for RSA signing
#[derive(Debug, Clone, Deserialize)]
pub struct ArweaveJWK {
    pub kty: String,
    pub n: String,
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

// ANS-104 signature type for RSA-256
const SIG_TYPE_RSA256: u16 = 1;
const RSA_SIG_LENGTH: usize = 512;
const RSA_OWNER_LENGTH: usize = 512;

/// Signs UnsignedDataItems with an Arweave RSA key
pub struct DataItemSigner {
    private_key: RsaPrivateKey,
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
                details: "JWK missing required RSA fields".to_string(),
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

        Ok(Self {
            private_key,
            owner_bytes,
        })
    }

    /// Sign a DataItem and return complete ANS-104 signed bytes
    pub fn sign(&self, item: &UnsignedDataItem) -> Result<Vec<u8>, AOCommunicationError> {
        let deep_hash_input = self.build_deep_hash_input(item);
        let signing_key = SigningKey::<Sha256>::new(self.private_key.clone());
        let signature = signing_key.sign(&deep_hash_input);
        let sig_bytes = signature.to_bytes();

        let mut result = Vec::new();

        // Signature type (2 bytes, little-endian)
        result.extend_from_slice(&SIG_TYPE_RSA256.to_le_bytes());

        // Signature (512 bytes, zero-padded)
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

        // Target
        if item.target.is_empty() {
            result.push(0);
        } else {
            result.push(1);
            result.extend_from_slice(&item.target);
        }

        // Anchor
        if item.anchor.is_empty() {
            result.push(0);
        } else {
            result.push(1);
            result.extend_from_slice(&item.anchor);
        }

        // Tags
        let tags_bytes = self.serialize_avro_tags(&item.tags);
        let num_tags = item.tags.len() as u64;
        result.extend_from_slice(&num_tags.to_le_bytes());
        result.extend_from_slice(&(tags_bytes.len() as u64).to_le_bytes());
        result.extend_from_slice(&tags_bytes);

        // Data
        result.extend_from_slice(&item.data);

        Ok(result)
    }

    /// Owner address: base64url(SHA-256(owner_public_key_bytes))
    pub fn owner_address(&self) -> String {
        let hash = Sha256::digest(&self.owner_bytes);
        URL_SAFE_NO_PAD.encode(hash)
    }

    fn build_deep_hash_input(&self, item: &UnsignedDataItem) -> Vec<u8> {
        let mut input = Vec::new();
        input.extend_from_slice(b"dataitem");
        input.extend_from_slice(b"1");
        input.extend_from_slice(&SIG_TYPE_RSA256.to_le_bytes());
        input.extend_from_slice(&self.owner_bytes);
        input.extend_from_slice(&item.target);
        input.extend_from_slice(&item.anchor);
        let tags_bytes = self.serialize_avro_tags(&item.tags);
        input.extend_from_slice(&tags_bytes);
        input.extend_from_slice(&item.data);
        input
    }

    fn serialize_avro_tags(&self, tags: &[DataItemTag]) -> Vec<u8> {
        let mut buf = Vec::new();
        for tag in tags {
            let name_bytes = tag.name.as_bytes();
            let value_bytes = tag.value.as_bytes();
            Self::avro_encode_long(&mut buf, name_bytes.len() as i64);
            buf.extend_from_slice(name_bytes);
            Self::avro_encode_long(&mut buf, value_bytes.len() as i64);
            buf.extend_from_slice(value_bytes);
        }
        buf
    }

    fn avro_encode_long(buf: &mut Vec<u8>, val: i64) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::external::ao_message::Binary;

    #[test]
    fn test_execute_msg_to_data_item() {
        let msg = ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf1".to_string(),
            kfrag: Binary::new(vec![1, 2, 3]),
        };
        let item = DataItemBuilder::build_execute("proc-123", &msg).unwrap();
        assert_eq!(item.target, b"proc-123");
        assert!(!item.data.is_empty());
    }

    #[test]
    fn test_data_item_ao_tags() {
        let msg = ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf1".to_string(),
            kfrag: Binary::new(vec![1]),
        };
        let item = DataItemBuilder::build_execute("proc-123", &msg).unwrap();
        let tag_names: Vec<&str> = item.tags.iter().map(|t| t.name.as_str()).collect();
        assert!(tag_names.contains(&"Data-Protocol"));
        assert!(tag_names.contains(&"Variant"));
        assert!(tag_names.contains(&"Type"));
        assert!(tag_names.contains(&"SDK"));
        assert!(tag_names.contains(&"Action"));
        assert!(tag_names.contains(&"Target"));

        let dp = item
            .tags
            .iter()
            .find(|t| t.name == "Data-Protocol")
            .unwrap();
        assert_eq!(dp.value, "ao");
    }

    #[test]
    fn test_delegate_kfrag_data_item() {
        let msg = ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf1".to_string(),
            kfrag: Binary::new(vec![1, 2, 3]),
        };
        let item = DataItemBuilder::build_execute("proc-123", &msg).unwrap();
        let action_tag = item.tags.iter().find(|t| t.name == "Action").unwrap();
        assert_eq!(action_tag.value, "DelegateKFrag");
    }

    #[test]
    fn test_get_cfrag_data_item() {
        let msg = QueryMsg::GetCFrag {
            kfrag_id: "kf1".to_string(),
            capsule_id: "cap1".to_string(),
        };
        let body = DataItemBuilder::build_query_body("proc-123", &msg).unwrap();
        let tags = body["Tags"].as_array().unwrap();
        let action = tags.iter().find(|t| t["name"] == "Action").unwrap();
        assert_eq!(action["value"], "GetCFrag");
    }

    #[test]
    fn test_data_item_target_tag() {
        let msg = ExecuteMsg::Reencrypt {
            kfrag_id: "kf1".to_string(),
            capsule_id: "cap1".to_string(),
        };
        let item = DataItemBuilder::build_execute("target-proc-456", &msg).unwrap();
        let target_tag = item.tags.iter().find(|t| t.name == "Target").unwrap();
        assert_eq!(target_tag.value, "target-proc-456");
    }

    #[test]
    fn test_dry_run_body_structure() {
        let msg = ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf1".to_string(),
            kfrag: Binary::new(vec![1]),
        };
        let body = DataItemBuilder::build_dry_run_body("proc-123", &msg).unwrap();
        assert_eq!(body["Target"], "proc-123");
        assert!(body["Tags"].is_array());
        assert!(body["Data"].is_string());
    }

    fn test_jwk() -> ArweaveJWK {
        use rand::rngs::OsRng;
        use rsa::RsaPrivateKey;
        use rsa::traits::{PrivateKeyParts, PublicKeyParts};

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
    fn test_signer_new_valid_jwk() {
        let jwk = test_jwk();
        let signer = DataItemSigner::new(&jwk);
        assert!(signer.is_ok());
    }

    #[test]
    fn test_signer_invalid_kty() {
        let mut jwk = test_jwk();
        jwk.kty = "EC".to_string();
        let result = DataItemSigner::new(&jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_signer_missing_fields() {
        let mut jwk = test_jwk();
        jwk.d = String::new();
        let result = DataItemSigner::new(&jwk);
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_data_item() {
        let jwk = test_jwk();
        let signer = DataItemSigner::new(&jwk).unwrap();
        let msg = ExecuteMsg::DelegateKFrag {
            kfrag_id: "kf1".to_string(),
            kfrag: Binary::new(vec![1, 2, 3]),
        };
        let item = DataItemBuilder::build_execute("proc-123", &msg).unwrap();
        let signed = signer.sign(&item);
        assert!(signed.is_ok());
        let bytes = signed.unwrap();
        // First 2 bytes: signature type (1 = RSA-256, little-endian)
        assert_eq!(bytes[0], 1);
        assert_eq!(bytes[1], 0);
        // Total header: 2 + 512 (sig) + 512 (owner) = 1026
        assert!(bytes.len() > 1026);
    }

    #[test]
    fn test_owner_address() {
        let jwk = test_jwk();
        let signer = DataItemSigner::new(&jwk).unwrap();
        let addr = signer.owner_address();
        assert!(!addr.is_empty());
        // SHA-256 output = 32 bytes, base64url encoded = 43 chars
        assert_eq!(addr.len(), 43);
    }

    #[test]
    fn test_all_execute_msg_actions() {
        let cases: Vec<(ExecuteMsg, &str)> = vec![
            (
                ExecuteMsg::DelegateKFrag {
                    kfrag_id: "k".to_string(),
                    kfrag: Binary::new(vec![1]),
                },
                "DelegateKFrag",
            ),
            (
                ExecuteMsg::DelegateCapsule {
                    kfrag_id: "k".to_string(),
                    capsule_id: "c".to_string(),
                    capsule: Binary::new(vec![1]),
                },
                "DelegateCapsule",
            ),
            (
                ExecuteMsg::SubmitKFrag {
                    kfrag_id: "k".to_string(),
                    kfrag: Binary::new(vec![1]),
                },
                "SubmitKFrag",
            ),
            (
                ExecuteMsg::SubmitCapsule {
                    kfrag_id: "k".to_string(),
                    capsule_id: "c".to_string(),
                    capsule: Binary::new(vec![1]),
                },
                "SubmitCapsule",
            ),
            (
                ExecuteMsg::Reencrypt {
                    kfrag_id: "k".to_string(),
                    capsule_id: "c".to_string(),
                },
                "Reencrypt",
            ),
        ];

        for (msg, expected_action) in cases {
            let item = DataItemBuilder::build_execute("proc", &msg).unwrap();
            let action_tag = item.tags.iter().find(|t| t.name == "Action").unwrap();
            assert_eq!(action_tag.value, expected_action);
        }
    }
}
