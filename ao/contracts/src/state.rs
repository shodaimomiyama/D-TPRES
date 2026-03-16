use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Replaces ao_cwao/'s CosmWasm storage maps.
/// All data lives in WASM linear memory; HyperBEAM snapshots/restores it.
///
/// Key naming mirrors ao_cwao/:
///   OWNER_KFRAGS     → owner_kfrags:  (kfrag_id)         → OwnerKFragData
///   OWNER_CAPSULES   → owner_capsules: (kfrag_id, cap_id) → OwnerCapsuleData
///   HOLDER_CFRAGS    → holder_cfrags:  (kfrag_id, cap_id) → HolderCFragData
///   KFRAG_HOLDERS    → kfrag_holders:  kfrag_id           → holder_process_id
///   INDEX_KFRAG_TO_CAPS → kfrag_to_caps: kfrag_id         → [capsule_ids]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProcessState {
    /// kfrag_id → raw kFrag bytes (bincode-encoded StoredKeyFrag)
    pub owner_kfrags: HashMap<String, Vec<u8>>,

    /// (kfrag_id, capsule_id) compound key → capsule data
    pub owner_capsules: HashMap<String, OwnerCapsuleData>,

    /// (kfrag_id, capsule_id) compound key → re-encrypted cFrag bytes
    pub holder_cfrags: HashMap<String, Vec<u8>>,

    /// kfrag_id → target holder AO process ID (replaces DEFAULT_HOLDER_PROCESS_ID)
    pub kfrag_holders: HashMap<String, String>,

    /// kfrag_id → list of capsule_ids (for listing)
    pub kfrag_to_caps: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Compound key for capsule/cfrag lookups
    pub fn cap_key(kfrag_id: &str, capsule_id: &str) -> String {
        format!("{}/{}", kfrag_id, capsule_id)
    }
}
