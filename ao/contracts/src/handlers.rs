use serde_json::json;
use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};

use crate::message::{AOMessage, AOResponse, OutgoingMessage};
use crate::state::{
    CapsuleStatus, OwnerCapsuleData, ProcessRole, ProcessState, StoredCFrag, StoredKeyFrag,
    VerificationData,
};

fn parse_byte_array(arr: &[serde_json::Value]) -> Result<Vec<u8>, &'static str> {
    arr.iter()
        .map(|v| {
            v.as_u64()
                .filter(|&n| n <= 255)
                .map(|n| n as u8)
                .ok_or("Byte array contains invalid value (expected 0-255)")
        })
        .collect()
}

pub fn dispatch(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    if msg.action == "Init" {
        return handle_init(state, msg);
    }

    if !state.is_initialized() {
        return AOResponse::error("Process not initialized — send Init first");
    }

    let role = state.role.as_ref().unwrap();

    // PRD §4: role-based action routing via process tag / msg.role
    // Combined role allows all Owner + Holder actions (single-process setups).
    match (role, msg.action.as_str()) {
        (ProcessRole::Owner | ProcessRole::Combined, "DelegateKFrag") => {
            handle_delegate_kfrag(state, msg)
        }
        (ProcessRole::Owner | ProcessRole::Combined, "DelegateCapsule") => {
            handle_delegate_capsule(state, msg)
        }

        (ProcessRole::Holder | ProcessRole::Combined, "SubmitKFrag") => {
            handle_submit_kfrag(state, msg)
        }
        (ProcessRole::Holder | ProcessRole::Combined, "SubmitCapsule") => {
            handle_submit_capsule(state, msg)
        }
        (ProcessRole::Holder | ProcessRole::Combined, "Reencrypt") => handle_reencrypt(state, msg),

        // Queries — allowed for any initialized role
        (_, "GetCFrag") => handle_get_cfrag(state, msg),
        (_, "ListCapsules") => handle_list_capsules(state, msg),

        (role, action) => {
            AOResponse::error(format!("Action '{action}' not permitted for role {role:?}"))
        }
    }
}

// ─── Init ──────────────────────────────────────────────────────────────────────
fn handle_init(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    if state.is_initialized() {
        return AOResponse::error("Already initialized");
    }

    let sender = match &msg.from {
        Some(s) if !s.is_empty() => s.clone(),
        _ => return AOResponse::error("Init requires a valid sender (from field)"),
    };

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };

    let role_str = match data["role"].as_str() {
        Some(s) => s,
        None => return AOResponse::error("Missing role"),
    };

    let role = match role_str {
        "Owner" => ProcessRole::Owner,
        "Holder" => ProcessRole::Holder,
        "Requester" => ProcessRole::Requester,
        "Combined" => ProcessRole::Combined,
        other => return AOResponse::error(format!("Invalid role: {other}")),
    };

    state.role = Some(role.clone());
    state.owner_id = Some(sender);

    AOResponse::success(json!({ "initialized": true, "role": role_str }))
}

// ─── DelegateKFrag ─────────────────────────────────────────────────────────────
fn handle_delegate_kfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    if !state.is_authorized_sender(msg.from.as_deref()) {
        return AOResponse::error("Unauthorized: only the owner can delegate kFrags");
    }

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };

    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let kfrag_bytes = match data["kfrag"].as_array() {
        Some(arr) => match parse_byte_array(arr) {
            Ok(bytes) => bytes,
            Err(e) => return AOResponse::error(e),
        },
        None => return AOResponse::error("Missing or invalid kfrag bytes"),
    };
    let holder_process_id = match data["holder_process_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing holder_process_id"),
    };

    state
        .owner_kfrags
        .insert(kfrag_id.clone(), kfrag_bytes.clone());
    state
        .kfrag_holders
        .insert(kfrag_id.clone(), holder_process_id.clone());

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
    if !state.is_authorized_sender(msg.from.as_deref()) {
        return AOResponse::error("Unauthorized: only the owner can delegate capsules");
    }

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };

    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let capsule_id = match data["capsule_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing capsule_id"),
    };
    let capsule_bytes = match data["capsule"].as_array() {
        Some(arr) => match parse_byte_array(arr) {
            Ok(bytes) => bytes,
            Err(e) => return AOResponse::error(e),
        },
        None => return AOResponse::error("Missing capsule bytes"),
    };

    let holder = match state.kfrag_holders.get(&kfrag_id) {
        Some(h) => h.clone(),
        None => return AOResponse::error(format!("No holder registered for kfrag_id: {kfrag_id}")),
    };

    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);
    state.owner_capsules.insert(
        cap_key.clone(),
        OwnerCapsuleData {
            capsule_bytes: capsule_bytes.clone(),
            status: CapsuleStatus::Received,
            kfrag_id: kfrag_id.clone(),
            capsule_id: capsule_id.clone(),
        },
    );

    state
        .kfrag_to_caps
        .entry(kfrag_id.clone())
        .or_default()
        .push(capsule_id.clone());

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
fn handle_submit_kfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    if !state.is_authorized_sender(msg.from.as_deref()) {
        return AOResponse::error("Unauthorized: only the registered owner can submit kFrags");
    }

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };
    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let kfrag_bytes: Vec<u8> = match data["kfrag"].as_array() {
        Some(arr) => match parse_byte_array(arr) {
            Ok(bytes) => bytes,
            Err(e) => return AOResponse::error(e),
        },
        None => return AOResponse::error("Missing kfrag"),
    };

    // Validate kFrag is deserializable before persisting to prevent bricked state
    if let Err(e) = bincode::deserialize::<StoredKeyFrag>(&kfrag_bytes) {
        return AOResponse::error(format!("Invalid kFrag payload: {e}"));
    }

    if state.owner_kfrags.contains_key(&kfrag_id) {
        return AOResponse::success(json!({
            "kfrag_id": kfrag_id, "stored": true, "idempotent": true
        }));
    }

    state.owner_kfrags.insert(kfrag_id.clone(), kfrag_bytes);
    AOResponse::success(json!({ "kfrag_id": kfrag_id, "stored": true }))
}

// ─── SubmitCapsule ────────────────────────────────────────────────────────────
fn handle_submit_capsule(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    if !state.is_authorized_sender(msg.from.as_deref()) {
        return AOResponse::error("Unauthorized: only the registered owner can submit capsules");
    }

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };
    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let capsule_id = match data["capsule_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing capsule_id"),
    };
    let capsule_bytes: Vec<u8> = match data["capsule"].as_array() {
        Some(arr) => match parse_byte_array(arr) {
            Ok(bytes) => bytes,
            Err(e) => return AOResponse::error(e),
        },
        None => return AOResponse::error("Missing capsule"),
    };

    let cap_key = ProcessState::cap_key(&kfrag_id, &capsule_id);

    if state.holder_cfrags.contains_key(&cap_key) {
        return AOResponse::success(json!({
            "capsule_id": capsule_id, "idempotent": true
        }));
    }

    state.owner_capsules.insert(
        cap_key.clone(),
        OwnerCapsuleData {
            capsule_bytes: capsule_bytes.clone(),
            status: CapsuleStatus::ReencInProgress,
            kfrag_id: kfrag_id.clone(),
            capsule_id: capsule_id.clone(),
        },
    );

    state
        .kfrag_to_caps
        .entry(kfrag_id.clone())
        .or_default()
        .push(capsule_id.clone());

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
    if !state.is_authorized_sender(msg.from.as_deref()) {
        return AOResponse::error(
            "Unauthorized: only the registered owner can trigger reencryption",
        );
    }

    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };
    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let capsule_id = match data["capsule_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing capsule_id"),
    };
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
        Err(e) => {
            if let Some(cap) = state.owner_capsules.get_mut(&cap_key) {
                cap.status = CapsuleStatus::Error(e.clone());
            }
            AOResponse::error(format!("Reencryption retry failed: {e}"))
        }
    }
}

// ─── GetCFrag (query) ─────────────────────────────────────────────────────────
fn handle_get_cfrag(state: &mut ProcessState, msg: AOMessage) -> AOResponse {
    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };
    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };
    let capsule_id = match data["capsule_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing capsule_id"),
    };
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
    let data = match msg.data {
        Some(d) => d,
        None => return AOResponse::error("Missing data"),
    };
    let kfrag_id = match data["kfrag_id"].as_str() {
        Some(s) => s.to_string(),
        None => return AOResponse::error("Missing kfrag_id"),
    };

    let capsule_ids = state
        .kfrag_to_caps
        .get(&kfrag_id)
        .cloned()
        .unwrap_or_default();
    AOResponse::success(json!({ "kfrag_id": kfrag_id, "capsule_ids": capsule_ids }))
}

// ─── Core re-encryption (ported from ao_cwao/ with kFrag verification) ────────
fn perform_reencryption(
    state: &ProcessState,
    kfrag_id: &str,
    capsule_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    let kfrag_bytes = state
        .owner_kfrags
        .get(kfrag_id)
        .ok_or_else(|| format!("KFrag not found: {kfrag_id}"))?;

    if kfrag_bytes.is_empty() {
        return Err("Empty kFrag payload".to_string());
    }
    if capsule_bytes.is_empty() {
        return Err("Empty capsule payload".to_string());
    }

    let stored_kfrag: StoredKeyFrag =
        bincode::deserialize(kfrag_bytes).map_err(|e| format!("KFrag deserialize failed: {e}"))?;

    if stored_kfrag.key_data.is_empty() {
        return Err("Invalid kFrag data".to_string());
    }
    if stored_kfrag.verification_data.is_empty() {
        return Err("Missing verification data".to_string());
    }

    // Cryptographic verification: deserialize public keys from verification_data
    // and verify the kFrag before re-encrypting (PRD §6.1 tamper detection)
    let verification: VerificationData = bincode::deserialize(&stored_kfrag.verification_data)
        .map_err(|_| "Invalid verification payload".to_string())?;

    let verifying_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.verifying_pk)
        .map_err(|_| "Invalid verifying key".to_string())?;

    let delegating_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.delegating_pk)
        .map_err(|_| "Invalid delegating key".to_string())?;

    let receiving_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.receiving_pk)
        .map_err(|_| "Invalid receiving key".to_string())?;

    let key_frag = umbral_pre::KeyFrag::from_bytes(&stored_kfrag.key_data)
        .map_err(|_| "Failed to deserialize kFrag".to_string())?;

    let verified_kfrag = key_frag
        .verify(&verifying_pk, Some(&delegating_pk), Some(&receiving_pk))
        .map_err(|_| "kFrag verification failed".to_string())?;

    let capsule = umbral_pre::Capsule::from_bytes(capsule_bytes)
        .map_err(|e| format!("Capsule deserialize failed: {e:?}"))?;

    let verified_cfrag = umbral_pre::reencrypt(&capsule, verified_kfrag);
    let cfrag = verified_cfrag.unverify();
    let cfrag_bytes = cfrag
        .to_bytes()
        .map_err(|_| "Failed to serialize cFrag".to_string())?;

    let stored_cfrag = StoredCFrag {
        fragment_data: cfrag_bytes.to_vec(),
    };
    bincode::serialize(&stored_cfrag).map_err(|e| format!("CFrag serialize failed: {e}"))
}
