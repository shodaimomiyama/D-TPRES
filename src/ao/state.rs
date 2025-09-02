//! AO State Management Module
//! 
//! This module provides state persistence and retrieval for AO Network processes.

use super::{AOState, ProcessRole, SerializedKeyFragment};
use std::collections::HashMap;

/// State manager for AO processes
pub struct StateManager {
    storage: HashMap<String, Vec<u8>>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }
    
    /// Save state to storage
    pub fn save_state(&mut self, process_id: &str, state: &AOState) -> Result<(), String> {
        let serialized = state.serialize()
            .map_err(|e| format!("Failed to serialize state: {}", e))?;
        
        self.storage.insert(process_id.to_string(), serialized);
        Ok(())
    }
    
    /// Load state from storage
    pub fn load_state(&self, process_id: &str) -> Result<AOState, String> {
        let data = self.storage.get(process_id)
            .ok_or_else(|| format!("State not found for process: {}", process_id))?;
        
        AOState::deserialize(data)
            .map_err(|e| format!("Failed to deserialize state: {}", e))
    }
    
    /// Check if state exists
    pub fn has_state(&self, process_id: &str) -> bool {
        self.storage.contains_key(process_id)
    }
    
    /// Delete state
    pub fn delete_state(&mut self, process_id: &str) -> bool {
        self.storage.remove(process_id).is_some()
    }
    
    /// List all process IDs
    pub fn list_processes(&self) -> Vec<String> {
        self.storage.keys().cloned().collect()
    }
}

/// Arweave storage adapter for production use
pub struct ArweaveStorage {
    // In production, this would interface with Arweave
    // For MVP, we use in-memory storage
    memory_storage: StateManager,
}

impl ArweaveStorage {
    /// Create new Arweave storage
    pub fn new() -> Self {
        Self {
            memory_storage: StateManager::new(),
        }
    }
    
    /// Store state in Arweave
    pub async fn store(&mut self, process_id: &str, state: &AOState) -> Result<String, String> {
        // MVP: Use in-memory storage
        // In production: Upload to Arweave and return transaction ID
        self.memory_storage.save_state(process_id, state)?;
        Ok(format!("tx_mock_{}", process_id))
    }
    
    /// Retrieve state from Arweave
    pub async fn retrieve(&self, process_id: &str) -> Result<AOState, String> {
        // MVP: Use in-memory storage
        // In production: Download from Arweave
        self.memory_storage.load_state(process_id)
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ArweaveStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_state_manager() {
        let mut manager = StateManager::new();
        
        // Create and save state
        let state = AOState::new("test_process".to_string(), ProcessRole::Owner);
        manager.save_state("test_process", &state).unwrap();
        
        // Load state
        let loaded = manager.load_state("test_process").unwrap();
        assert_eq!(loaded.process_id, "test_process");
        
        // Check existence
        assert!(manager.has_state("test_process"));
        assert!(!manager.has_state("non_existent"));
        
        // List processes
        let processes = manager.list_processes();
        assert_eq!(processes.len(), 1);
        assert!(processes.contains(&"test_process".to_string()));
        
        // Delete state
        assert!(manager.delete_state("test_process"));
        assert!(!manager.has_state("test_process"));
    }
    
    #[tokio::test]
    async fn test_arweave_storage() {
        let mut storage = ArweaveStorage::new();
        
        // Store state
        let state = AOState::new("test_process".to_string(), ProcessRole::Holder);
        let tx_id = storage.store("test_process", &state).await.unwrap();
        assert!(tx_id.contains("test_process"));
        
        // Retrieve state
        let retrieved = storage.retrieve("test_process").await.unwrap();
        assert_eq!(retrieved.process_id, "test_process");
    }
}