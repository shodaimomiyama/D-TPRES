//! CapsuleEntity - PRE capsule representation
//!
//! Capsule information for Proxy Re-Encryption (Phase 1).
//! Used in Phase 4 for re-encryption operations.

use serde::{Serialize, Deserialize};

/// Capsule entity - PRE encryption capsule
/// 
/// Generated in Phase 1, used in Phase 4 re-encryption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapsuleEntity {
    /// Capsule identifier
    pub capsule_id: String,
    
    /// Related data identifier
    pub data_id: String,
    
    /// Related secret identifier
    pub secret_id: String,
    
    /// Capsule index (i: corresponding share index)
    pub capsule_index: u8,
    
    /// PRE Capsule data
    /// Capsulei = PRE_Enc(pkO, Ki)
    pub capsule_data: Vec<u8>,
    
    /// Corresponding ciphertext identifier (Ci = AES_GCM(Ki, f(i)))
    pub corresponding_ciphertext_id: String,
    
    /// Owner public key (pkO)
    pub owner_public_key: Vec<u8>,
    
    /// Random key used for capsule generation (stored encrypted)
    pub encrypted_random_key: Vec<u8>,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Version
    pub version: u64,
}