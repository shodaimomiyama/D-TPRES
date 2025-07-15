//! ShareEntity - Shamir data share representation
//!
//! Data fragments split using Shamir Secret Sharing (Phase 1).
//! Contains encrypted fragments that can reconstruct the original secret.

use serde::{Serialize, Deserialize};

/// Data share entity - Shamir-split data fragment
/// 
/// Generated in PRD Phase 1, required for secret reconstruction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShareEntity {
    /// Share identifier
    pub share_id: String,
    
    /// Data group identifier (identifies shares from the same secret)
    pub data_id: String,
    
    /// Related secret identifier
    pub secret_id: String,
    
    /// Threshold index (1 to n)
    pub threshold_index: u8,
    
    /// Shamir threshold (k: minimum shares needed for reconstruction)
    pub shamir_threshold: u8,
    
    /// Shamir total shares (n: total number of generated shares)
    pub shamir_total_shares: u8,
    
    /// Encrypted fragment data
    /// Ci = AES_GCM(Ki, f(i)) where f(i) is Shamir share
    pub encrypted_fragment: Vec<u8>,
    
    /// Fragment size (bytes)
    pub fragment_size: usize,
    
    /// Data owner's public key (pkO)
    pub owner_public_key: Vec<u8>,
    
    /// Integrity verification hash (SHA-256)
    pub integrity_hash: Vec<u8>,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Last access timestamp
    pub last_accessed_at: Option<u64>,
    
    /// Version
    pub version: u64,
}