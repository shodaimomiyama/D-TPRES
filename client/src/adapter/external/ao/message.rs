//! AO-native message types for HyperBEAM communication.
//!
//! Replaces the CosmWasm-style enum messages in `ao_cwao/message.rs`.
//! New format: `{ "action": "...", "data": { ... } }` for both execute and query.
//!
//! The Binary type and validation helpers are carried over from ao_cwao.

use serde::{Deserialize, Serialize};

// ─── Binary (unchanged from ao_cwao) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Binary(#[serde(with = "base64_serde")] pub Vec<u8>);

impl Binary {
    pub fn new(bytes: Vec<u8>) -> Self { Self(bytes) }
    pub fn as_slice(&self) -> &[u8] { &self.0 }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
    pub fn into_vec(self) -> Vec<u8> { self.0 }
}
impl From<Vec<u8>> for Binary {
    fn from(bytes: Vec<u8>) -> Self { Self(bytes) }
}
impl From<&[u8]> for Binary {
    fn from(bytes: &[u8]) -> Self { Self(bytes.to_vec()) }
}
impl AsRef<[u8]> for Binary {
    fn as_ref(&self) -> &[u8] { &self.0 }
}

mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

// ─── AO-native Execute Message ───────────────────────────────────────────────

/// Execute message for state-changing operations (HyperBEAM format).
/// Serializes as: `{ "action": "DelegateKFrag", "data": { ... } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOExecuteMsg {
    pub action: String,
    pub data: serde_json::Value,
}

impl AOExecuteMsg {
    /// Delegate a kFrag to the Holder-Process.
    pub fn delegate_kfrag(kfrag_id: impl Into<String>, kfrag: Vec<u8>) -> Self {
        Self {
            action: "DelegateKFrag".to_string(),
            data: serde_json::json!({
                "kfrag_id": kfrag_id.into(),
                "kfrag": kfrag,
            }),
        }
    }

    /// Delegate a Capsule along with its associated kFrag.
    pub fn delegate_capsule(
        kfrag_id: impl Into<String>,
        capsule_id: impl Into<String>,
        capsule: Vec<u8>,
        holder_process_id: impl Into<String>,
    ) -> Self {
        Self {
            action: "DelegateCapsule".to_string(),
            data: serde_json::json!({
                "kfrag_id": kfrag_id.into(),
                "capsule_id": capsule_id.into(),
                "capsule": capsule,
                "holder_process_id": holder_process_id.into(),
            }),
        }
    }

    /// Retry re-encryption for a specific (kfrag_id, capsule_id) pair.
    pub fn reencrypt(kfrag_id: impl Into<String>, capsule_id: impl Into<String>) -> Self {
        Self {
            action: "Reencrypt".to_string(),
            data: serde_json::json!({
                "kfrag_id": kfrag_id.into(),
                "capsule_id": capsule_id.into(),
            }),
        }
    }
}

// ─── AO-native Query Message ─────────────────────────────────────────────────

/// Query message for read-only operations (HyperBEAM format).
/// Serializes as: `{ "action": "GetCFrag", "data": { ... } }`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOQueryMsg {
    pub action: String,
    pub data: serde_json::Value,
}

impl AOQueryMsg {
    /// Get a cFrag for a specific kFrag and Capsule pair.
    pub fn get_cfrag(kfrag_id: impl Into<String>, capsule_id: impl Into<String>) -> Self {
        Self {
            action: "GetCFrag".to_string(),
            data: serde_json::json!({
                "kfrag_id": kfrag_id.into(),
                "capsule_id": capsule_id.into(),
            }),
        }
    }

    /// List Capsules associated with a kFrag (with optional pagination).
    pub fn list_capsules_by_kfrag(
        kfrag_id: impl Into<String>,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Self {
        Self {
            action: "ListCapsules".to_string(),
            data: serde_json::json!({
                "kfrag_id": kfrag_id.into(),
                "start_after": start_after,
                "limit": limit,
            }),
        }
    }
}

// ─── AO-native Response ───────────────────────────────────────────────────────

/// Response from AO-native contract (HyperBEAM format).
/// Mirrors `ao/contracts/src/message.rs :: AOResponse`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AONativeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Message ID returned from AO MU
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

impl AONativeResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self { ok: true, data: Some(data), error: None, message_id: None }
    }
    pub fn success_empty() -> Self {
        Self { ok: true, data: None, error: None, message_id: None }
    }
}

// ─── Response sub-types (for GetCFrag deserialization) ───────────────────────

/// Parsed response for a GetCFrag query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCFragResponse {
    pub kfrag_id: String,
    pub capsule_id: String,
    /// Raw cFrag bytes (Vec<u8> in JSON, base64 in some encodings)
    pub cfrag: Vec<u8>,
}

// ─── Validation (unchanged from ao_cwao) ─────────────────────────────────────

pub const MAX_ID_LENGTH: usize = 128;
pub const MAX_BINARY_SIZE: usize = 128 * 1024;

pub fn validate_id(id: &str, field: &str) -> Result<(), String> {
    if id.is_empty() { return Err(format!("{field} cannot be empty")); }
    if id.len() > MAX_ID_LENGTH {
        return Err(format!("{field} must be <= {MAX_ID_LENGTH} chars"));
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(format!("{field} must be ASCII alphanumeric, underscore or hyphen"));
    }
    Ok(())
}

pub fn validate_binary(binary: &[u8], field: &str) -> Result<(), String> {
    if binary.is_empty() { return Err(format!("{field} cannot be empty")); }
    if binary.len() > MAX_BINARY_SIZE {
        return Err(format!("{field} exceeds {}KB", MAX_BINARY_SIZE / 1024));
    }
    Ok(())
}
