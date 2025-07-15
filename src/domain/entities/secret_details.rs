//! SecretDetailsEntity - Detailed secret management information
//!
//! Holds detailed secret management information separated from ProcessEntity.
//! Supports ProcessEntity lightweight design for AO's stateless environment.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Secret management details entity - Detailed information about a secret
/// 
/// Separated from ProcessEntity, loaded only when needed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretDetailsEntity {
    /// Details entity identifier
    pub details_id: String,
    
    /// Related secret identifier
    pub secret_id: String,
    
    /// Access control conditions
    /// Example: ["erc20_balance_check", "nft_ownership_check"]
    pub access_control_conditions: Vec<String>,
    
    /// Generated kFrag collection by condition (created in Phase 3)
    /// Key: access control condition, Value: RekeyFragmentEntity ID list
    pub generated_kfrags_by_condition: HashMap<String, Vec<String>>,
    
    /// Access history
    pub access_history: Vec<AccessRecord>,
    
    /// Secret metadata
    pub metadata: HashMap<String, String>,
    
    /// Secret description
    pub description: Option<String>,
    
    /// Expiration timestamp
    pub expires_at: Option<u64>,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Last update timestamp
    pub updated_at: u64,
    
    /// Version
    pub version: u64,
}

/// Access record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessRecord {
    /// Access request ID
    pub request_id: String,
    
    /// Accessor process ID
    pub accessor_process_id: String,
    
    /// Access timestamp
    pub accessed_at: u64,
    
    /// Access result
    /// Values: "granted", "denied", "expired"
    pub result: String,
    
    /// Used access control condition
    pub condition_used: String,
}