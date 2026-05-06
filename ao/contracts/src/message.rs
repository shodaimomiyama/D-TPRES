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

/// AO Tag name-value pair as sent by JSON-Iface.
#[derive(Debug, Deserialize)]
pub struct Tag {
    pub name: String,
    pub value: String,
}

/// Raw incoming message from JSON-Iface (AO Tags format).
/// Parsed from null-terminated JSON written to WASM memory.
#[derive(Debug, Deserialize)]
pub struct AOIncomingMessage {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "From")]
    pub from: Option<String>,
    #[serde(rename = "Owner")]
    pub owner: Option<String>,
    #[serde(rename = "Tags")]
    pub tags: Option<Vec<Tag>>,
    #[serde(rename = "Data")]
    pub data: Option<serde_json::Value>,
    #[serde(rename = "Target")]
    pub target: Option<String>,
    #[serde(rename = "Module")]
    pub module: Option<String>,
    #[serde(rename = "Block-Height")]
    pub block_height: Option<serde_json::Value>,
}

impl AOIncomingMessage {
    /// Extract Action from Tags and convert to internal AOMessage.
    pub fn into_ao_message(self) -> AOMessage {
        let action = self
            .tags
            .as_ref()
            .and_then(|tags| tags.iter().find(|t| t.name == "Action"))
            .map(|t| t.value.clone())
            .unwrap_or_default();

        // Data may arrive as a JSON string that needs re-parsing,
        // or as a direct JSON object.
        let data = match self.data {
            Some(serde_json::Value::String(s)) => serde_json::from_str(&s)
                .ok()
                .or(Some(serde_json::Value::String(s))),
            other => other,
        };

        AOMessage {
            action,
            from: self.from.or(self.owner),
            id: self.id,
            data,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ao_incoming_message_with_tags() {
        let json = r#"{
            "Id": "msg-123",
            "From": "wallet-abc",
            "Tags": [
                {"name": "Action", "value": "Init"},
                {"name": "Data-Protocol", "value": "ao"}
            ],
            "Data": "{\"role\":\"Owner\"}"
        }"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.action, "Init");
        assert_eq!(msg.from, Some("wallet-abc".to_string()));
        assert_eq!(msg.id, Some("msg-123".to_string()));
        assert!(msg.data.is_some());
        assert_eq!(msg.data.unwrap()["role"], "Owner");
    }

    #[test]
    fn parse_incoming_data_as_object() {
        let json = r#"{
            "Tags": [{"name": "Action", "value": "ListCapsules"}],
            "Data": {"kfrag_id": "kf-1"}
        }"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.action, "ListCapsules");
        assert_eq!(msg.data.unwrap()["kfrag_id"], "kf-1");
    }

    #[test]
    fn parse_incoming_no_tags_yields_empty_action() {
        let json = r#"{"From": "wallet-abc"}"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.action, "");
        assert_eq!(msg.from, Some("wallet-abc".to_string()));
    }

    #[test]
    fn parse_incoming_falls_back_to_owner() {
        let json = r#"{
            "Owner": "owner-wallet",
            "Tags": [{"name": "Action", "value": "Init"}],
            "Data": "{\"role\":\"Holder\"}"
        }"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.from, Some("owner-wallet".to_string()));
    }

    #[test]
    fn parse_incoming_from_takes_precedence_over_owner() {
        let json = r#"{
            "From": "sender-wallet",
            "Owner": "owner-wallet",
            "Tags": [{"name": "Action", "value": "Init"}]
        }"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.from, Some("sender-wallet".to_string()));
    }

    #[test]
    fn parse_incoming_empty_tags_yields_empty_action() {
        let json = r#"{"Tags": [], "From": "wallet"}"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.action, "");
    }

    #[test]
    fn parse_incoming_data_non_json_string_preserved() {
        let json = r#"{
            "Tags": [{"name": "Action", "value": "Eval"}],
            "Data": "return 1+1"
        }"#;

        let incoming: AOIncomingMessage = serde_json::from_str(json).unwrap();
        let msg = incoming.into_ao_message();

        assert_eq!(msg.action, "Eval");
        assert_eq!(msg.data.unwrap(), "return 1+1");
    }
}
