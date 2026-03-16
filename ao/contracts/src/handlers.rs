/// Business logic handlers - AO-native port of ao_cwao/contracts/src/handlers.rs
///
/// Key changes from CosmWasm version:
/// - deps.storage replaced by &mut ProcessState
/// - SubMsg replaced by OutgoingMessage (async AO messages)
/// - Response::new() replaced by AOResponse::success/error
/// - umbral-pre logic: UNCHANGED

use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};
use serde_json::json;

use crate::message::{AOMessage, AOResponse, OutgoingMessage};
use crate::state::{CapsuleStatus, OwnerCapsuleData, ProcessState};

pub fn dispatch(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    match msg.action.as_str() {
        "DelegateKFrag"    => handle_delegate_kfrag(state, msg),
        "DelegateCapsule"  => handle_delegate_capsule(state, msg),
        "SubmitKFrag"      => handle_submit_kfrag(state, msg),
        "SubmitCapsule"    => handle_submit_capsule(state, msg),
        "Reencrypt"        => handle_reencrypt(state, msg),
        "GetCFrag"         => handle_get_cfrag(state, msg),
        "ListCapsules"     => handle_list_capsules(state, msg),
        action             => AOResponse::error(format!("Unknown action: {action}")),
    }
}

// ─── DelegateKFrag ─────────────────────────────────────────────────────────────
// Owner sends kFrag + holder target to this contract.
// Previously used SubMsg to call SubmitKFrag on a separate holder contract.
// AO version: record the delegation, emit OutgoingMessage to holder.
fn handle_delegate_kfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data {
        Some(d) => d,
        None    => return AOResponse::error("Missing data"),
    };

    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None    => return AOResponse::error("Missing kfrag_id"),
    };
    let kfrag_bytes = match data["kfrag"].as_array() {
        Some(arr) => arr.iter()
            .filter_map(|v| v.as_u64())
            .map(|n| n as u8)
            .collect::<Vec<_>>(),
        None => return AOResponse::error("Missing or invalid kfrag bytes"),
    };
    let holder_process_id = match data["holder_process_id"].as_str() {
        Some(s) => s.to_string(),
        None    => return AOResponse::error("Missing holder_process_id"),
    };

    // Store the kfrag and holder mapping
    state.owner_kfrags.insert(kfrag_id.clone(), kfrag_bytes.clone());
    state.kfrag_holders.insert(kfrag_id.clone(), holder_process_id.clone());

    // Emit message to holder (replaces SubMsg)
    let outgoing = OutgoingMessage {
        target: holder_process_id,
        action: "SubmitKFrag".to_string(),
        data: json!({
            "kfrag_id": kfrag_id,
            "kfrag": kfrag_bytes,
        }),
    };

    AOResponse::success_with_messages(
        json!({ "kfrag_id": kfrag_id, "delegated": true }),
        vec![outgoing],
    )
}

// ─── DelegateCapsule ──────────────────────────────────────────────────────────
fn handle_delegate_capsule(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data {
        Some(d) => d,
        None    => return AOResponse::error("Missing data"),
    };

    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None    => return AOResponse::error("Missing kfrag_id"),
    };
    let capsule_id = match data["capsule_id"].as_str() {
        Some(s) => s.to_string(),
        None    => return AOResponse::error("Missing capsule_id"),
    };
    let capsule_bytes = match data["capsule"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_u64()).map(|n| n as u8).collect::<Vec<_>>(),
        None      => return AOResponse::error("Missing capsule bytes"),
    };

    let holder = match state.kfrag_holders.get(&kfrag_id) {
        Some(h) => h.clone(),
        None    => return AOResponse::error(format!("No holder registered for kfrag_id: {kfrag_id}")),
    };

    // Store capsule locally
    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);
    state.owner_capsules.insert(cap_key.clone(), OwnerCapsuleData {
        capsule_bytes: capsule_bytes.clone(),
        status: CapsuleStatus::Received,
        kfrag_id: kfrag_id.clone(),
        capsule_id: capsule_id.clone(),
    });

    // Track for listing
    state.kfrag_to_caps
        .entry(kfrag_id.clone())
        .or_default()
        .push(capsule_id.clone());

    // Emit message to holder
    let outgoing = OutgoingMessage {
        target: holder,
        action: "SubmitCapsule".to_string(),
        data: json!({
            "kfrag_id": kfrag_id,
            "capsule_id": capsule_id,
            "capsule": capsule_bytes,
        }),
    };

    AOResponse::success_with_messages(
        json!({ "capsule_id": capsule_id, "delegated": true }),
        vec![outgoing],
    )
}

// ─── SubmitKFrag ──────────────────────────────────────────────────────────────
// Holder stores kFrag. Called via AO message (async, was SubMsg before).
fn handle_submit_kfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data { Some(d) => d, None => return AOResponse::error("Missing data") };
    let kfrag_id = match data["kfrag_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing kfrag_id") };
    let kfrag_bytes: Vec<u8> = match data["kfrag"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_u64()).map(|n| n as u8).collect(),
        None      => return AOResponse::error("Missing kfrag"),
    };

    // Idempotent: if already stored, no-op
    if state.owner_kfrags.contains_key(&kfrag_id) {
        return AOResponse::success(json!({ "kfrag_id": kfrag_id, "stored": true, "idempotent": true }));
    }

    state.owner_kfrags.insert(kfrag_id.clone(), kfrag_bytes);
    AOResponse::success(json!({ "kfrag_id": kfrag_id, "stored": true }))
}

// ─── SubmitCapsule ────────────────────────────────────────────────────────────
// Holder stores capsule and performs re-encryption.
fn handle_submit_capsule(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data { Some(d) => d, None => return AOResponse::error("Missing data") };
    let kfrag_id = match data["kfrag_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing kfrag_id") };
    let capsule_id = match data["capsule_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing capsule_id") };
    let capsule_bytes: Vec<u8> = match data["capsule"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_u64()).map(|n| n as u8).collect(),
        None      => return AOResponse::error("Missing capsule"),
    };

    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);

    // Idempotency check
    if state.holder_cfrags.contains_key(&cap_key) {
        return AOResponse::success(json!({ "capsule_id": capsule_id, "idempotent": true }));
    }

    // Store capsule
    state.owner_capsules.insert(cap_key.clone(), OwnerCapsuleData {
        capsule_bytes: capsule_bytes.clone(),
        status: CapsuleStatus::ReencInProgress,
        kfrag_id: kfrag_id.clone(),
        capsule_id: capsule_id.clone(),
    });

    // Perform re-encryption (same logic as ao_cwao/handlers.rs)
    match perform_reencryption(state, &kfrag_id, &capsule_bytes) {
        Ok(cfrag_bytes) => {
            state.holder_cfrags.insert(cap_key.clone(), cfrag_bytes);
            if let Some(cap) = state.owner_capsules.get_mut(&cap_key) {
                cap.status = CapsuleStatus::CFragReady;
            }
            AOResponse::success(json!({ "capsule_id": capsule_id, "cfrag_ready": true }))
        }
        Err(e) => {
            if let Some(cap) = state.owner_capsules.get_mut(&cap_key) {
                cap.status = CapsuleStatus::Error(e.clone());
            }
            AOResponse::error(format!("Reencryption failed: {e}"))
        }
    }
}

// ─── Reencrypt (retry) ────────────────────────────────────────────────────────
fn handle_reencrypt(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data { Some(d) => d, None => return AOResponse::error("Missing data") };
    let kfrag_id = match data["kfrag_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing kfrag_id") };
    let capsule_id = match data["capsule_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing capsule_id") };
    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);

    let capsule_bytes = match state.owner_capsules.get(&cap_key) {
        Some(cap) => cap.capsule_bytes.clone(),
        None => return AOResponse::error(format!("Capsule not found: {cap_key}")),
    };

    match perform_reencryption(state, &kfrag_id, &capsule_bytes) {
        Ok(cfrag_bytes) => {
            state.holder_cfrags.insert(cap_key.clone(), cfrag_bytes);
            if let Some(cap) = state.owner_capsules.get_mut(&cap_key) {
                cap.status = CapsuleStatus::CFragReady;
            }
            AOResponse::success(json!({ "capsule_id": capsule_id, "cfrag_ready": true }))
        }
        Err(e) => AOResponse::error(format!("Reencryption retry failed: {e}")),
    }
}

// ─── GetCFrag (query) ─────────────────────────────────────────────────────────
fn handle_get_cfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data { Some(d) => d, None => return AOResponse::error("Missing data") };
    let kfrag_id = match data["kfrag_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing kfrag_id") };
    let capsule_id = match data["capsule_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing capsule_id") };
    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);

    match state.holder_cfrags.get(&cap_key) {
        Some(cfrag_bytes) => AOResponse::success(json!({
            "kfrag_id": kfrag_id,
            "capsule_id": capsule_id,
            "cfrag": cfrag_bytes,
        })),
        None => AOResponse::error(format!("CFrag not ready: {cap_key}")),
    }
}

// ─── ListCapsules ─────────────────────────────────────────────────────────────
fn handle_list_capsules(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data { Some(d) => d, None => return AOResponse::error("Missing data") };
    let kfrag_id = match data["kfrag_id"].as_str() { Some(s) => s.to_string(), None => return AOResponse::error("Missing kfrag_id") };

    let capsule_ids = state.kfrag_to_caps.get(&kfrag_id).cloned().unwrap_or_default();
    AOResponse::success(json!({ "kfrag_id": kfrag_id, "capsule_ids": capsule_ids }))
}

// ─── Core re-encryption logic (UNCHANGED from ao_cwao/handlers.rs) ────────────
fn perform_reencryption(state: &ProcessState, kfrag_id: &str, capsule_bytes: &[u8])
    -> Result<Vec<u8>, String>
{
    use umbral_pre::{KeyFrag, Capsule};

    // Deserialize kFrag
    let kfrag_bytes = state.owner_kfrags.get(kfrag_id)
        .ok_or_else(|| format!("KFrag not found: {kfrag_id}"))?;

    // bincode → StoredKeyFrag (same as ao_cwao/)
    let stored_kfrag: StoredKeyFrag = bincode::deserialize(kfrag_bytes)
        .map_err(|e| format!("KFrag deserialize failed: {e}"))?;

    // Deserialize capsule
    let capsule = Capsule::from_bytes(capsule_bytes)
        .map_err(|e| format!("Capsule deserialize failed: {e:?}"))?;

    // Reconstruct kFrag for umbral-pre
    let kfrag = KeyFrag::from_bytes(&stored_kfrag.key_data)
        .map_err(|e| format!("KeyFrag from bytes failed: {e:?}"))?;

    // Perform re-encryption
    let cfrag = umbral_pre::reencrypt(&capsule, &kfrag, None);

    // Serialize cFrag
    let stored_cfrag = StoredCFrag {
        fragment_data: cfrag.to_bytes().to_vec(),
    };
    bincode::serialize(&stored_cfrag)
        .map_err(|e| format!("CFrag serialize failed: {e}"))
}

// ─── Serialized crypto types (mirrors ao_cwao/state.rs) ──────────────────────
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredKeyFrag {
    pub id: String,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredCFrag {
    pub fragment_data: Vec<u8>,
}
