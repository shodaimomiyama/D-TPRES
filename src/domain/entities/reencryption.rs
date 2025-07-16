//! ReencryptionEntity - Proxy re-encryption process management
//!
//! Manages k-of-n proxy re-encryption process (Phase 4).
//! Tracks cFrag collection and threshold achievement.

use serde::{Deserialize, Serialize};

/// Re-encryption entity - Proxy re-encryption process management
///
/// Created in Phase 4, tracks cFrag collection and re-encryption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReencryptionEntity {
    /// Re-encryption identifier
    pub reencryption_id: String,

    /// Related access request ID
    pub access_request_id: String,

    /// Target capsule ID
    pub target_capsule_id: String,

    /// Requester process ID (R-Proc)
    pub requester_process_id: String,

    /// Target Holder process IDs
    pub target_holders: Vec<String>,

    /// Required threshold (k)
    pub required_threshold: u8,

    /// Collected cFrag collection
    pub collected_cfrags: Vec<CFragData>,

    /// Re-encryption status
    /// Values: "initiated", "collecting", "threshold_met", "completed", "failed"
    pub status: String,

    /// Start timestamp
    pub started_at: u64,

    /// Completion timestamp
    pub completed_at: Option<u64>,

    /// Timeout timestamp
    pub timeout_at: u64,

    /// Version
    pub version: u64,
}

/// cFrag data - Re-encrypted fragment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CFragData {
    /// cFrag identifier
    pub cfrag_id: String,

    /// Source Holder process ID
    pub holder_id: String,

    /// cFrag data
    /// cFragj = PRE_ReEnc(kFragj, Capsulei)
    pub cfrag_data: Vec<u8>,

    /// Corresponding kFrag ID
    pub corresponding_kfrag_id: String,

    /// Generation timestamp
    pub generated_at: u64,

    /// Holder signature (for integrity)
    pub holder_signature: Vec<u8>,
}
