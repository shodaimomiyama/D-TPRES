//! AO Message types for client-side communication
//!
//! This module defines message types for communicating with AO Network processes.
//! Types are duplicated from `ao/contracts/src/msg.rs` for client independence.

use serde::{Deserialize, Serialize};

// =====================================================================
// Binary wrapper type
// =====================================================================

/// Binary data wrapper for serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Binary(#[serde(with = "base64_serde")] pub Vec<u8>);

impl Binary {
    /// Create a new Binary from bytes
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Get the underlying bytes as a slice
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Get the length of the binary data
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the binary data is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume and return the underlying Vec
    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for Binary {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Binary {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

impl AsRef<[u8]> for Binary {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

mod base64_serde {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}

// =====================================================================
// Execute Messages (state-changing operations)
// =====================================================================

/// Execute messages for state-changing operations on AO processes
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ExecuteMsg {
    /// Delegate a KFrag to the Owner-Process
    DelegateKFrag { kfrag_id: String, kfrag: Binary },

    /// Delegate a Capsule along with its associated KFrag
    DelegateCapsule {
        kfrag_id: String,
        capsule_id: String,
        capsule: Binary,
    },

    /// Submit a KFrag to a Holder-Process
    SubmitKFrag { kfrag_id: String, kfrag: Binary },

    /// Submit a Capsule to a Holder-Process
    SubmitCapsule {
        kfrag_id: String,
        capsule_id: String,
        capsule: Binary,
    },

    /// Request re-encryption of a Capsule
    Reencrypt {
        kfrag_id: String,
        capsule_id: String,
    },
}

// =====================================================================
// Query Messages (read-only operations)
// =====================================================================

/// Query messages for read-only operations on AO processes
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum QueryMsg {
    /// Get a CFrag for a specific KFrag and Capsule
    GetCFrag {
        kfrag_id: String,
        capsule_id: String,
    },

    /// List all Capsules associated with a KFrag
    ListCapsulesByKFrag {
        kfrag_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Get all CFrags associated with a secret
    GetCFragsBySecret { secret_id: String },
}

// =====================================================================
// Response types
// =====================================================================

/// Response from AO Process execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct AOResponse {
    /// Whether the execution succeeded
    pub success: bool,
    /// Response data (if any)
    pub data: Option<Binary>,
    /// Events emitted during execution
    pub events: Vec<AOEvent>,
    /// MU-returned message ID for AO Link verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

impl AOResponse {
    /// Create a successful response with no data
    pub fn success() -> Self {
        Self {
            success: true,
            data: None,
            events: Vec::new(),
            message_id: None,
        }
    }

    /// Create a successful response with data
    pub fn success_with_data(value: Binary) -> Self {
        Self {
            success: true,
            data: Some(value),
            events: Vec::new(),
            message_id: None,
        }
    }

    /// Create a successful response with events
    pub fn success_with_events(events: Vec<AOEvent>) -> Self {
        Self {
            success: true,
            data: None,
            events,
            message_id: None,
        }
    }

    /// Add an event to the response
    pub fn add_event(&mut self, event: AOEvent) {
        self.events.push(event);
    }
}

/// Event emitted during AO execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct AOEvent {
    /// Type of the event
    pub event_type: String,
    /// Attributes associated with the event
    pub attributes: Vec<AOAttribute>,
}

impl AOEvent {
    /// Create a new event
    pub fn new(event_type: &str) -> Self {
        Self {
            event_type: event_type.to_string(),
            attributes: Vec::new(),
        }
    }

    /// Add an attribute to the event
    pub fn add_attribute(&mut self, key: &str, value: &str) {
        self.attributes.push(AOAttribute {
            key: key.to_string(),
            value: value.to_string(),
        });
    }

    /// Create an event with attributes
    pub fn with_attributes(event_type: &str, attributes: Vec<(&str, &str)>) -> Self {
        Self {
            event_type: event_type.to_string(),
            attributes: attributes
                .into_iter()
                .map(|(k, v)| AOAttribute {
                    key: k.to_string(),
                    value: v.to_string(),
                })
                .collect(),
        }
    }
}

/// Attribute of an AO event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct AOAttribute {
    /// Attribute key
    pub key: String,
    /// Attribute value
    pub value: String,
}

/// Response for GetCFrag query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GetCFragResponse {
    /// The CFrag binary data
    pub cfrag: Binary,
    /// Metadata about the blob
    pub meta: BlobMeta,
}

/// Metadata for a stored blob
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlobMeta {
    /// Size of the blob in bytes
    pub size: u64,
    /// Timestamp when the blob was created
    pub created_at: String,
    /// Optional content type
    pub content_type: Option<String>,
}

/// Status of a Capsule
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapsuleStatus {
    /// Capsule is pending re-encryption
    Pending,
    /// Capsule has been re-encrypted
    Reencrypted,
    /// Capsule processing failed
    Failed,
}

/// Information about a Capsule
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CapsuleInfo {
    /// Capsule identifier
    pub capsule_id: String,
    /// Current status of the capsule
    pub status: CapsuleStatus,
    /// Last update timestamp
    pub updated_ts: String,
}

/// Response for ListCapsulesByKFrag query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ListCapsulesByKFragResponse {
    /// List of capsules
    pub capsules: Vec<CapsuleInfo>,
    /// Cursor for pagination
    pub next_start_after: Option<String>,
}

/// Response for GetCFragsBySecret query
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct GetCFragsBySecretResponse {
    pub cfrags: Vec<CFragEntry>,
}

/// A single cFrag entry returned from AO contract
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CFragEntry {
    pub cfrag_data: Binary,
    pub holder_id: String,
}

// =====================================================================
// AO Message Tags
// =====================================================================

/// Tags for AO messages (metadata)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AOMessageTags {
    /// Application name (always "cwao" for CosmWasm AO)
    pub app_name: String,
    /// Action being performed
    pub action: String,
    /// Whether this is a read-only operation
    pub read_only: String,
    /// JSON-encoded input payload
    pub input: String,
    /// Target process ID
    pub process_id: String,
    /// Actor (sender) address
    pub actor: String,
    /// Timestamp
    pub ts: String,
}

impl AOMessageTags {
    /// Create tags for an execute (state-changing) message
    pub fn new_execute(action: &str, input: &str, process_id: &str, actor: &str, ts: &str) -> Self {
        Self {
            app_name: "cwao".to_string(),
            action: action.to_string(),
            read_only: "False".to_string(),
            input: input.to_string(),
            process_id: process_id.to_string(),
            actor: actor.to_string(),
            ts: ts.to_string(),
        }
    }

    /// Create tags for a query (read-only) message
    pub fn new_query(action: &str, input: &str, process_id: &str, actor: &str, ts: &str) -> Self {
        Self {
            app_name: "cwao".to_string(),
            action: action.to_string(),
            read_only: "True".to_string(),
            input: input.to_string(),
            process_id: process_id.to_string(),
            actor: actor.to_string(),
            ts: ts.to_string(),
        }
    }

    /// Check if this is a read-only operation
    pub fn is_read_only(&self) -> bool {
        self.read_only == "True"
    }
}

// =====================================================================
// Message Validation
// =====================================================================

/// Trait for validating AO messages
pub trait ValidateMessage {
    /// Validate the message contents
    fn validate(&self) -> Result<(), String>;
}

impl ValidateMessage for ExecuteMsg {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::DelegateKFrag { kfrag_id, kfrag } => {
                validate_kfrag_id(kfrag_id)?;
                validate_binary_data(kfrag, "kfrag")?;
                Ok(())
            }
            Self::DelegateCapsule {
                kfrag_id,
                capsule_id,
                capsule,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                validate_binary_data(capsule, "capsule")?;
                Ok(())
            }
            Self::SubmitKFrag { kfrag_id, kfrag } => {
                validate_kfrag_id(kfrag_id)?;
                validate_binary_data(kfrag, "kfrag")?;
                Ok(())
            }
            Self::SubmitCapsule {
                kfrag_id,
                capsule_id,
                capsule,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                validate_binary_data(capsule, "capsule")?;
                Ok(())
            }
            Self::Reencrypt {
                kfrag_id,
                capsule_id,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                Ok(())
            }
        }
    }
}

impl ValidateMessage for QueryMsg {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::GetCFrag {
                kfrag_id,
                capsule_id,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                Ok(())
            }
            Self::ListCapsulesByKFrag {
                kfrag_id,
                start_after,
                limit,
            } => {
                validate_kfrag_id(kfrag_id)?;
                if let Some(start_after) = start_after {
                    validate_capsule_id(start_after)?;
                }
                if let Some(limit) = limit {
                    if *limit == 0 || *limit > 100 {
                        return Err("limit must be between 1 and 100".to_string());
                    }
                }
                Ok(())
            }
            Self::GetCFragsBySecret { secret_id } => {
                validate_id(secret_id, "secret_id")?;
                Ok(())
            }
        }
    }
}

// =====================================================================
// Validation helpers
// =====================================================================

/// Maximum length for IDs (kfrag_id, capsule_id, etc.)
pub const MAX_ID_LENGTH: usize = 128;

/// Maximum size for binary data (128KB)
pub const MAX_BINARY_SIZE: usize = 128 * 1024;

/// Validate a KFrag ID
#[allow(dead_code)]
pub fn validate_kfrag_id(id: &str) -> Result<(), String> {
    validate_id(id, "kfrag_id")
}

/// Validate a Capsule ID
#[allow(dead_code)]
pub fn validate_capsule_id(id: &str) -> Result<(), String> {
    validate_id(id, "capsule_id")
}

/// Validate a process ID
#[allow(dead_code)]
pub fn validate_process_id(id: &str) -> Result<(), String> {
    validate_id(id, "process_id")
}

/// Generic ID validation
fn validate_id(id: &str, field_name: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    if id.len() > MAX_ID_LENGTH {
        return Err(format!(
            "{field_name} must be <= {MAX_ID_LENGTH} characters"
        ));
    }

    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "{field_name} must contain only ASCII alphanumeric, underscore, or hyphen"
        ));
    }

    Ok(())
}

/// Validate binary data
pub fn validate_binary_data(binary: &Binary, field_name: &str) -> Result<(), String> {
    if binary.is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }

    if binary.len() > MAX_BINARY_SIZE {
        let max_kb = MAX_BINARY_SIZE / 1024;
        return Err(format!("{field_name} exceeds maximum size of {max_kb}KB"));
    }

    Ok(())
}
