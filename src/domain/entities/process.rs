//! ProcessEntity - Multi-role process representation for AO
//!
//! Each AO process can have multiple roles (Owner/Holder/Requester) simultaneously.
//! Designed for AO's stateless execution environment with lightweight secret indexing.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

/// Process entity - Complete state representation of an AO process
/// 
/// # Features
/// - Multi-role support (Owner/Holder/Requester can coexist)
/// - AO environment native properties
/// - Integrated performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessEntity {
    /// Process identifier (AO process ID)
    pub process_id: String,
    
    /// Process name (human-readable identifier)
    pub process_name: String,
    
    /// Currently active roles
    /// Values: ["owner"], ["holder"], ["requester"], or combinations
    pub active_roles: Vec<String>,
    
    /// Owner functionality data (used in Phase 0, 1, 3)
    pub owner_data: Option<OwnerData>,
    
    /// Holder functionality data (used in Phase 3, 4)
    pub holder_data: Option<HolderData>,
    
    /// Requester functionality data (used in Phase 2, 4, 5)
    pub requester_data: Option<RequesterData>,
    
    /// Process configuration (Key-Value format)
    /// Example: {"max_concurrent_requests": "10", "timeout_seconds": "300"}
    pub configuration: HashMap<String, String>,
    
    /// Supported crypto operations list
    /// Values: ["shamir_split", "pre_encrypt", "re_encrypt", "verify_proof"]
    pub supported_crypto_operations: Vec<String>,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Creation timestamp (Unix timestamp seconds)
    pub created_at: u64,
    
    /// Last update timestamp (Unix timestamp seconds)
    pub updated_at: u64,
    
    /// Version number (for optimistic locking)
    pub version: u64,
}

/// Owner functionality data - Manages skO and secret splitting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnerData {
    /// Owner public key (pkO corresponding to skO)
    pub owner_public_key: Vec<u8>,
    
    /// Secret index information
    /// Key: secret ID, Value: lightweight secret index
    /// Detailed information is managed separately in SecretDetailsEntity
    pub secret_indices: HashMap<String, SecretIndex>,
    
    /// Owner-specific configuration
    /// Example: {"default_threshold": "3", "default_shares": "5"}
    pub owner_config: HashMap<String, String>,
}

/// Secret index - Lightweight secret management information
/// Minimal information held in ProcessEntity
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretIndex {
    /// Secret identifier
    pub secret_id: String,
    
    /// Secret status
    /// Values: "active", "archived", "expired"
    pub status: String,
    
    /// Entity reference information
    pub entity_references: EntityReferences,
    
    /// Last update timestamp
    pub last_updated: u64,
    
    /// Shamir threshold (k) - kept for frequent reference
    pub shamir_threshold: u8,
    
    /// Shamir total shares (n) - kept for frequent reference
    pub shamir_total_shares: u8,
}

/// Entity references - Holds only IDs of related entities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityReferences {
    /// ShareEntity ID list
    pub share_ids: Vec<String>,
    
    /// CapsuleEntity ID list
    pub capsule_ids: Vec<String>,
    
    /// Active AccessRequestEntity ID list
    pub active_requests: Vec<String>,
    
    /// SecretDetailsEntity ID (reference to detailed information)
    pub details_entity_id: String,
}

/// Holder functionality data - kFrag storage and re-encryption execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderData {
    /// Held kFrag collection
    /// Key: RekeyFragment ID, Value: fragment information
    pub held_fragments: HashMap<String, HolderFragmentInfo>,
    
    /// kFrag management by access control condition
    /// Key: access control condition, Value: RekeyFragment ID list
    pub fragments_by_condition: HashMap<String, Vec<String>>,
    
    /// Reliability score (0.0-1.0)
    pub reliability_score: f64,
    
    /// Completed re-encryption count
    pub completed_reencryptions: u64,
    
    /// Maximum fragment capacity
    pub max_fragment_capacity: u64,
    
    /// Current load status (0-100)
    pub current_load: u64,
}

/// Holder fragment information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderFragmentInfo {
    /// Fragment identifier
    pub fragment_id: String,
    
    /// Related secret ID
    pub secret_id: String,
    
    /// Access control condition
    pub access_control_condition: String,
    
    /// Receipt timestamp
    pub received_at: u64,
    
    /// Usage count
    pub usage_count: u64,
    
    /// Status ("active", "expired", "revoked")
    pub status: String,
}

/// Requester functionality data - Access requests and cFrag collection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequesterData {
    /// Active access request ID list
    pub active_requests: Vec<String>,
    
    /// Active re-encryption request ID list
    pub active_reencryptions: Vec<String>,
    
    /// Completed request count
    pub completed_requests: u64,
    
    /// Success rate (0.0-1.0)
    pub success_rate: f64,
    
    /// Average processing time (milliseconds)
    pub average_processing_time_ms: u64,
    
    /// Requester-specific configuration
    /// Example: {"retry_attempts": "3", "timeout_ms": "30000"}
    pub requester_config: HashMap<String, String>,
}

/// Performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Successful operation count
    pub successful_operations: u64,
    
    /// Failed operation count
    pub failed_operations: u64,
    
    /// Average response time (milliseconds)
    pub average_response_time_ms: u64,
    
    /// Last metrics update timestamp
    pub last_updated_at: u64,
}