use serde::{Deserialize, Serialize};

/// Incoming AO message structure.
/// HyperBEAM serializes the AO message to JSON before passing to WASM.
#[derive(Debug, Deserialize)]
pub struct AOMessage {
    /// Message action (e.g., "DelegateKFrag", "Reencrypt", "GetCFrag")
    pub action: String,
    /// Sender address / AO process ID
    pub from: Option<String>,
    /// Message ID (from AO network)
    pub id: Option<String>,
    /// Action-specific payload
    pub data: Option<serde_json::Value>,
}

/// Response returned from handle().
/// Serialized as JSON and written to WASM memory.
#[derive(Debug, Serialize)]
pub struct AOResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Messages to send to other AO processes (replaces CosmWasm SubMsg)
    /// HyperBEAM will route these to their targets via ao_send.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<OutgoingMessage>,
}

/// Outgoing AO message (replaces CosmWasm SubMsg).
#[derive(Debug, Serialize)]
pub struct OutgoingMessage {
    pub target: String,
    pub action: String,
    pub data: serde_json::Value,
}

impl AOResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            messages: Vec::new(),
        }
    }

    pub fn success_with_messages(data: serde_json::Value, messages: Vec<OutgoingMessage>) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            messages,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
            messages: Vec::new(),
        }
    }
}
