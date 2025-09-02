//! AO Network Integration Layer
//! 
//! This module provides the integration with the AO Network for message handling
//! and state management in the D-TPRES system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod message;
pub mod state;

/// AO Process Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOMessage {
    pub id: String,
    pub action: String,
    pub data: Vec<u8>,
    pub from: String,
    pub timestamp: u64,
    pub tags: HashMap<String, String>,
}

/// AO Process Response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Vec<u8>>,
}

/// AO Process State structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AOState {
    pub process_id: String,
    pub role: ProcessRole,
    pub kfrags: Vec<SerializedKeyFragment>,
    pub metadata: ProcessMetadata,
}

/// Process role enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessRole {
    Owner,
    Holder,
    Requester,
}

/// Serialized KeyFragment for AO storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedKeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// Process metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetadata {
    pub created_at: u64,
    pub updated_at: u64,
    pub version: String,
    pub tags: HashMap<String, String>,
}

impl AOMessage {
    /// Create a new AO message
    pub fn new(action: String, data: Vec<u8>, from: String) -> Self {
        Self {
            id: generate_message_id(),
            action,
            data,
            from,
            timestamp: current_timestamp(),
            tags: HashMap::new(),
        }
    }
    
    /// Add a tag to the message
    pub fn with_tag(mut self, key: String, value: String) -> Self {
        self.tags.insert(key, value);
        self
    }
}

impl AOResponse {
    /// Create a successful response
    pub fn success(message: String, data: Option<Vec<u8>>) -> Self {
        Self {
            success: true,
            message,
            data,
        }
    }
    
    /// Create an error response
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            message,
            data: None,
        }
    }
}

impl AOState {
    /// Create a new AO state
    pub fn new(process_id: String, role: ProcessRole) -> Self {
        Self {
            process_id,
            role,
            kfrags: vec![],
            metadata: ProcessMetadata {
                created_at: current_timestamp(),
                updated_at: current_timestamp(),
                version: "0.1.0".to_string(),
                tags: HashMap::new(),
            },
        }
    }
    
    /// Update kfrags in state
    pub fn update_kfrags(&mut self, kfrags: Vec<SerializedKeyFragment>) {
        self.kfrags = kfrags;
        self.metadata.updated_at = current_timestamp();
    }
    
    /// Serialize state for storage
    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
    
    /// Deserialize state from storage
    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

/// Generate unique message ID
fn generate_message_id() -> String {
    format!("msg_{}", uuid_simple())
}

/// Get current timestamp
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Simple UUID generator for MVP
fn uuid_simple() -> String {
    format!("{:x}", current_timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ao_message_creation() {
        let msg = AOMessage::new(
            "StoreKfrags".to_string(),
            vec![1, 2, 3],
            "owner_process".to_string(),
        );
        
        assert_eq!(msg.action, "StoreKfrags");
        assert_eq!(msg.data, vec![1, 2, 3]);
        assert_eq!(msg.from, "owner_process");
        assert!(msg.timestamp > 0);
    }
    
    #[test]
    fn test_ao_response() {
        let success = AOResponse::success(
            "Operation completed".to_string(),
            Some(vec![4, 5, 6]),
        );
        
        assert!(success.success);
        assert_eq!(success.message, "Operation completed");
        assert_eq!(success.data, Some(vec![4, 5, 6]));
        
        let error = AOResponse::error("Failed to process".to_string());
        
        assert!(!error.success);
        assert_eq!(error.message, "Failed to process");
        assert!(error.data.is_none());
    }
    
    #[test]
    fn test_ao_state_serialization() {
        let mut state = AOState::new("process_123".to_string(), ProcessRole::Owner);
        
        let kfrag = SerializedKeyFragment {
            id: 0,
            key_data: vec![7, 8, 9],
            verification_data: vec![10, 11, 12],
            precursor: vec![],
        };
        
        state.update_kfrags(vec![kfrag]);
        
        let serialized = state.serialize().unwrap();
        let deserialized = AOState::deserialize(&serialized).unwrap();
        
        assert_eq!(deserialized.process_id, "process_123");
        assert_eq!(deserialized.kfrags.len(), 1);
        assert_eq!(deserialized.kfrags[0].id, 0);
    }
}