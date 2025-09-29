//! AO Message Handling Module
//! 
//! This module provides message routing and handling for AO Network integration.

use super::{AOMessage, AOResponse, AOState, ProcessRole};
use crate::usecase::owner::owner_handlers;

/// Message router for AO messages
pub struct MessageRouter {
    state: AOState,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new(state: AOState) -> Self {
        Self { state }
    }
    
    /// Route and handle incoming messages
    pub fn handle_message(&mut self, message: AOMessage) -> AOResponse {
        match self.state.role {
            ProcessRole::Owner => self.handle_owner_message(message),
            ProcessRole::Holder => self.handle_holder_message(message),
            ProcessRole::Requester => self.handle_requester_message(message),
        }
    }
    
    /// Handle messages for Owner-Process
    fn handle_owner_message(&mut self, message: AOMessage) -> AOResponse {
        match message.action.as_str() {
            "SetupEncryption" => self.handle_setup_encryption(message),
            "StoreKfrags" => self.handle_store_kfrags(message),
            "GetKfrags" => self.handle_get_kfrags(message),
            "ProcessAccessRequest" => self.handle_access_request(message),
            _ => AOResponse::error(format!("Unknown action: {}", message.action)),
        }
    }
    
    /// Handle messages for Holder-Process
    fn handle_holder_message(&mut self, _message: AOMessage) -> AOResponse {
        // MVP: Holder functionality not implemented yet
        AOResponse::error("Holder functionality not implemented in MVP".to_string())
    }
    
    /// Handle messages for Requester-Process
    fn handle_requester_message(&mut self, _message: AOMessage) -> AOResponse {
        // MVP: Requester functionality not implemented yet
        AOResponse::error("Requester functionality not implemented in MVP".to_string())
    }
    
    /// Handle setup encryption message
    fn handle_setup_encryption(&mut self, message: AOMessage) -> AOResponse {
        // Parse message data
        let params: SetupEncryptionParams = match serde_json::from_slice(&message.data) {
            Ok(p) => p,
            Err(e) => return AOResponse::error(format!("Invalid parameters: {}", e)),
        };
        
        // Execute setup
        match owner_handlers::handle_setup_encryption(&params.secret, params.threshold, params.total_shares) {
            Ok(result) => {
                // Convert and store kfrags
                let serialized_kfrags = result.kfrags.clone().into_iter()
                    .map(|kf| super::SerializedKeyFragment {
                        id: kf.id,
                        key_data: kf.key_data,
                        verification_data: kf.verification_data,
                        precursor: kf.precursor,
                    })
                    .collect();
                
                self.state.update_kfrags(serialized_kfrags);
                
                let response_data = serde_json::to_vec(&result).ok();
                AOResponse::success("Encryption setup completed".to_string(), response_data)
            }
            Err(e) => AOResponse::error(format!("Setup failed: {:?}", e)),
        }
    }
    
    /// Handle store kfrags message
    fn handle_store_kfrags(&mut self, message: AOMessage) -> AOResponse {
        // Parse kfrags from message
        let kfrags: Vec<super::SerializedKeyFragment> = match serde_json::from_slice(&message.data) {
            Ok(kf) => kf,
            Err(e) => return AOResponse::error(format!("Invalid kfrags data: {}", e)),
        };
        
        // Store kfrags in state
        self.state.update_kfrags(kfrags.clone());
        
        let response_data = serde_json::json!({
            "stored_count": kfrags.len(),
            "process_id": self.state.process_id,
        });
        
        AOResponse::success(
            format!("Stored {} kfrags", kfrags.len()),
            serde_json::to_vec(&response_data).ok(),
        )
    }
    
    /// Handle get kfrags message
    fn handle_get_kfrags(&self, _message: AOMessage) -> AOResponse {
        let response_data = serde_json::json!({
            "kfrags": self.state.kfrags,
            "count": self.state.kfrags.len(),
        });
        
        AOResponse::success(
            "Retrieved kfrags".to_string(),
            serde_json::to_vec(&response_data).ok(),
        )
    }
    
    /// Handle access request message
    fn handle_access_request(&self, message: AOMessage) -> AOResponse {
        let params: AccessRequestParams = match serde_json::from_slice(&message.data) {
            Ok(p) => p,
            Err(e) => return AOResponse::error(format!("Invalid parameters: {}", e)),
        };
        
        // MVP: Auto-approve
        let response_data = serde_json::json!({
            "approved": true,
            "requester_id": params.requester_id,
            "capsule_id": params.capsule_id,
            "kfrags_available": !self.state.kfrags.is_empty(),
        });
        
        AOResponse::success(
            "Access request approved".to_string(),
            serde_json::to_vec(&response_data).ok(),
        )
    }
}

/// Parameters for setup encryption
#[derive(serde::Deserialize)]
struct SetupEncryptionParams {
    secret: Vec<u8>,
    threshold: u8,
    total_shares: u8,
}

/// Parameters for access request
#[derive(serde::Deserialize)]
struct AccessRequestParams {
    requester_id: String,
    capsule_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_router() {
        let state = AOState::new("test_process".to_string(), ProcessRole::Owner);
        let mut router = MessageRouter::new(state);
        
        // Test setup encryption
        let params = serde_json::json!({
            "secret": vec![1, 2, 3, 4, 5],
            "threshold": 2,
            "total_shares": 3,
        });
        
        let message = AOMessage::new(
            "SetupEncryption".to_string(),
            serde_json::to_vec(&params).unwrap(),
            "test_sender".to_string(),
        );
        
        let response = router.handle_message(message);
        assert!(response.success);
        assert!(response.message.contains("completed"));
    }
    
    #[test]
    fn test_store_and_get_kfrags() {
        let state = AOState::new("test_process".to_string(), ProcessRole::Owner);
        let mut router = MessageRouter::new(state);
        
        // Store kfrags
        let kfrags = vec![
            super::super::SerializedKeyFragment {
                id: 0,
                key_data: vec![1, 2, 3],
                verification_data: vec![4, 5, 6],
                precursor: vec![],
            },
        ];
        
        let store_msg = AOMessage::new(
            "StoreKfrags".to_string(),
            serde_json::to_vec(&kfrags).unwrap(),
            "test_sender".to_string(),
        );
        
        let store_response = router.handle_message(store_msg);
        assert!(store_response.success);
        
        // Get kfrags
        let get_msg = AOMessage::new(
            "GetKfrags".to_string(),
            vec![],
            "test_sender".to_string(),
        );
        
        let get_response = router.handle_message(get_msg);
        assert!(get_response.success);
        assert!(get_response.data.is_some());
    }
}
