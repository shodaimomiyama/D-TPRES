use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessRole {
    Owner,
    Holder,
    Requester,
    /// Single-process mode: allows both Owner and Holder actions.
    Combined,
}

/// WASM-memory-resident state, persisted via HyperBEAM memory snapshots.
///
/// Key naming mirrors ao_cwao/:
///   OWNER_KFRAGS     -> owner_kfrags:  (kfrag_id)         -> raw bytes
///   OWNER_CAPSULES   -> owner_capsules: (kfrag_id, cap_id) -> OwnerCapsuleData
///   HOLDER_CFRAGS    -> holder_cfrags:  (kfrag_id, cap_id) -> raw bytes
///   KFRAG_HOLDERS    -> kfrag_holders:  kfrag_id           -> holder_process_id
///   INDEX_KFRAG_TO_CAPS -> kfrag_to_caps: kfrag_id         -> [capsule_ids]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProcessState {
    pub role: Option<ProcessRole>,
    /// Authorized owner address (the AO process/wallet that initialized this contract)
    pub owner_id: Option<String>,

    pub owner_kfrags: HashMap<String, Vec<u8>>,
    pub owner_capsules: HashMap<String, OwnerCapsuleData>,
    pub holder_cfrags: HashMap<String, Vec<u8>>,
    pub kfrag_holders: HashMap<String, String>,
    pub kfrag_to_caps: HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OwnerCapsuleData {
    pub capsule_bytes: Vec<u8>,
    pub status: CapsuleStatus,
    pub kfrag_id: String,
    pub capsule_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapsuleStatus {
    Received,
    ReencInProgress,
    CFragReady,
    Error(String),
}

impl ProcessState {
    pub fn new() -> Self {
        Self::default()
    }

    // Length-prefixed encoding prevents collisions from IDs containing the delimiter
    pub fn cap_key(kfrag_id: &str, capsule_id: &str) -> String {
        format!("{}:{}/{}", kfrag_id.len(), kfrag_id, capsule_id)
    }

    pub fn is_initialized(&self) -> bool {
        self.role.is_some()
    }

    pub fn is_authorized_sender(&self, from: Option<&str>) -> bool {
        match (&self.owner_id, from) {
            (Some(owner), Some(sender)) => owner == sender,
            _ => false,
        }
    }
}

/// Intermediate deserialized kFrag — zeroized on drop to prevent
/// key material from lingering in WASM linear memory after use.
#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct StoredKeyFrag {
    #[zeroize(skip)]
    pub id: String,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// Deserialized from StoredKeyFrag.verification_data for kFrag verification.
#[derive(Serialize, Deserialize)]
pub struct VerificationData {
    pub verifying_pk: Vec<u8>,
    pub delegating_pk: Vec<u8>,
    pub receiving_pk: Vec<u8>,
}
