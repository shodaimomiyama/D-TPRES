//! RekeyFragmentEntity - Re-encryption key fragment representation
//!
//! Shamir-split re-encryption key fragments (kFrag) for threshold re-encryption (Phase 3).
//! Distributed to holders for decentralized re-encryption capability.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Re-encryption key fragment entity - Split re-encryption key
///
/// Generated in Phase 3, distributed to Holders
/// Contains sensitive key material that must be zeroized
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RekeyFragmentEntity {
    /// Fragment identifier
    #[zeroize(skip)]
    pub fragment_id: String,

    /// Related secret identifier
    #[zeroize(skip)]
    pub secret_id: String,

    /// Related access request ID
    #[zeroize(skip)]
    pub access_request_id: String,

    /// Access control condition
    #[zeroize(skip)]
    pub access_control_condition: String,

    /// Owner public key (pkO)
    #[zeroize(skip)]
    pub owner_public_key: Vec<u8>,

    /// Accessor public key (pkA)
    #[zeroize(skip)]
    pub accessor_public_key: Vec<u8>,

    /// Shamir fragment index (j: 1 to n)
    #[zeroize(skip)]
    pub shamir_index: u8,

    /// Shamir threshold (k: minimum fragments needed for re-encryption)
    #[zeroize(skip)]
    pub shamir_threshold: u8,

    /// Shamir total fragments (n)
    #[zeroize(skip)]
    pub shamir_total_fragments: u8,

    /// kFrag data (Shamir-split re-encryption key fragment)
    /// kFragj = ShamirSplit(ReKey(skO→pkA), j)
    /// This is sensitive cryptographic material
    pub kfrag_data: Vec<u8>,

    /// Assigned Holder process ID
    #[zeroize(skip)]
    pub assigned_holder_id: String,

    /// Fragment status
    /// Values: "created", "distributed", "active", "consumed", "expired"
    #[zeroize(skip)]
    pub status: String,

    /// Expiration timestamp
    #[zeroize(skip)]
    pub expires_at: Option<u64>,

    /// Creation timestamp
    #[zeroize(skip)]
    pub created_at: u64,

    /// Distribution timestamp
    #[zeroize(skip)]
    pub distributed_at: Option<u64>,

    /// Version
    #[zeroize(skip)]
    pub version: u64,
}
