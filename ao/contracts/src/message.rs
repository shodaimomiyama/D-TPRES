use serde::{Deserialize, Serialize};

/// Incoming AO message structure.
/// HyperBEAM serializes the AO message to JSON before passing to WASM.
#[derive(Debug, Deserialize)]
pub struct AOMessage {
    /// Message action (e.g., "DelegateKFrag", "Reencrypt", "GetCFrag")
    pub action: String,
    /// Sender address / AO process ID
    pub from: Option<String>,
    /// Message ID (from AO network). Unread for now, but kept so the struct
    /// mirrors the full AO message shape.
    #[allow(dead_code)]
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
    // The remaining fields are part of the JSON-Iface wire format; they are
    // accepted but currently unread by the contract.
    #[serde(rename = "Target")]
    #[allow(dead_code)]
    pub target: Option<String>,
    #[serde(rename = "Module")]
    #[allow(dead_code)]
    pub module: Option<String>,
    #[serde(rename = "Block-Height")]
    #[allow(dead_code)]
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

/// AOS-compatible response wrapper for JSON-Iface.
/// JSON-Iface's `json_to_message` expects this format.
#[derive(Debug, Serialize)]
pub struct AOSResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<AOSResultBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AOSResultBody {
    #[serde(rename = "Output")]
    pub output: AOSOutput,
    #[serde(rename = "Messages")]
    pub messages: Vec<AOSOutgoingMessage>,
    #[serde(rename = "Spawns")]
    pub spawns: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct AOSOutput {
    pub data: serde_json::Value,
}

/// Outgoing message in AOS Tags format for JSON-Iface routing.
#[derive(Debug, Serialize)]
pub struct AOSOutgoingMessage {
    #[serde(rename = "Target")]
    pub target: String,
    #[serde(rename = "Tags")]
    pub tags: Vec<AOSTag>,
    #[serde(rename = "Data")]
    pub data: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AOSTag {
    pub name: String,
    pub value: String,
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

    /// Wrap internal response in AOS-compatible format for JSON-Iface.
    pub fn into_aos_response(self) -> AOSResponse {
        if !self.ok {
            return AOSResponse {
                ok: false,
                response: None,
                error: self.error,
            };
        }

        let messages: Vec<AOSOutgoingMessage> = self
            .messages
            .into_iter()
            .map(|m| AOSOutgoingMessage {
                target: m.target,
                tags: vec![AOSTag {
                    name: "Action".to_string(),
                    value: m.action,
                }],
                data: m.data,
            })
            .collect();

        let data = self
            .data
            .unwrap_or(serde_json::Value::String(String::new()));

        AOSResponse {
            ok: true,
            response: Some(AOSResultBody {
                output: AOSOutput { data },
                messages,
                spawns: Vec::new(),
            }),
            error: None,
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

    #[test]
    fn aos_response_success_format() {
        let response = AOResponse::success(serde_json::json!({"result": "ok"}));
        let aos = response.into_aos_response();

        let json_str = serde_json::to_string(&aos).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["response"]["Output"]["data"]["result"], "ok");
        assert!(
            parsed["response"]["Messages"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(parsed["response"]["Spawns"].as_array().unwrap().is_empty());
    }

    #[test]
    fn aos_response_error_format() {
        let response = AOResponse::error("something failed");
        let aos = response.into_aos_response();

        let json_str = serde_json::to_string(&aos).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["ok"], false);
        assert_eq!(parsed["error"], "something failed");
        assert!(parsed.get("response").is_none());
    }

    #[test]
    fn aos_response_with_messages() {
        let outgoing = OutgoingMessage {
            target: "holder-proc-123".to_string(),
            action: "SubmitKFrag".to_string(),
            data: serde_json::json!({"kfrag_id": "kf-1"}),
        };
        let response = AOResponse::success_with_messages(
            serde_json::json!({"delegated": true}),
            vec![outgoing],
        );
        let aos = response.into_aos_response();

        let json_str = serde_json::to_string(&aos).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let msgs = parsed["response"]["Messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["Target"], "holder-proc-123");
        assert_eq!(msgs[0]["Tags"][0]["name"], "Action");
        assert_eq!(msgs[0]["Tags"][0]["value"], "SubmitKFrag");
        assert_eq!(msgs[0]["Data"]["kfrag_id"], "kf-1");
    }
}
