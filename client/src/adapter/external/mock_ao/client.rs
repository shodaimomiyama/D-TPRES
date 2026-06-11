//! Mock AO Network client (updated for HyperBEAM-native message types)
//!
//! Implements `AOClient` from `ao/` using `AOExecuteMsg`/`AOQueryMsg`/`AONativeResponse`.
//! All business logic dispatches on `msg.action()` string instead of enum variants.
//! Uses real Umbral proxy re-encryption for DelegateCapsule/SubmitCapsule/Reencrypt.

#![allow(clippy::unwrap_used)]
#![allow(clippy::disallowed_names)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::manual_let_else)]

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;
use serde::Deserialize;
use umbral_pre::{DefaultDeserialize, DefaultSerialize};

use crate::adapter::errors::AOCommunicationError;
use crate::adapter::external::ao::{
    AOClient, AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary, CFragEntry, GetCFragResponse,
    GetCFragsResponse,
};

// ─── Local structs mirroring contract's StoredKeyFrag / VerificationData ──────
// Duplicated here to avoid layering dependency on ao/contracts.

#[derive(Deserialize)]
struct MockStoredKeyFrag {
    #[allow(dead_code)]
    id: String,
    key_data: Vec<u8>,
    verification_data: Vec<u8>,
    #[allow(dead_code)]
    precursor: Vec<u8>,
}

#[derive(Deserialize)]
struct MockVerificationData {
    verifying_pk: Vec<u8>,
    delegating_pk: Vec<u8>,
    receiving_pk: Vec<u8>,
}

/// Configuration for MockAOClient behavior
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MockConfig {
    pub delay_ms: Option<u64>,
    pub fail_rate: Option<f64>,
    /// If true, next `execute()` call returns `ok: false` response instead of success.
    pub force_ok_false: bool,
}

/// Type alias: process_id -> (capsule_id -> (kfrag_id, capsule_data))
type CapsuleStorage = HashMap<String, HashMap<String, (String, Vec<u8>)>>;

/// Mock AO Client for testing and development.
/// Type alias: process_id -> (kfrag_id -> holder_process_id)
type KFragHolderMap = HashMap<String, HashMap<String, String>>;

pub struct MockAOClient {
    kfrag_storage: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
    cfrag_storage: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
    capsule_storage: RwLock<CapsuleStorage>,
    kfrag_holders: RwLock<KFragHolderMap>,
    #[allow(dead_code)]
    config: MockConfig,
    error_injection: RwLock<Option<AOCommunicationError>>,
    /// When Some, next execute() returns AONativeResponse { ok: false, error: msg }
    ok_false_injection: RwLock<Option<String>>,
}

impl MockAOClient {
    pub fn new() -> Self {
        Self {
            kfrag_storage: RwLock::new(HashMap::new()),
            cfrag_storage: RwLock::new(HashMap::new()),
            capsule_storage: RwLock::new(HashMap::new()),
            kfrag_holders: RwLock::new(HashMap::new()),
            config: MockConfig::default(),
            error_injection: RwLock::new(None),
            ok_false_injection: RwLock::new(None),
        }
    }
    pub fn with_config(config: MockConfig) -> Self {
        Self {
            kfrag_storage: RwLock::new(HashMap::new()),
            cfrag_storage: RwLock::new(HashMap::new()),
            capsule_storage: RwLock::new(HashMap::new()),
            kfrag_holders: RwLock::new(HashMap::new()),
            config,
            error_injection: RwLock::new(None),
            ok_false_injection: RwLock::new(None),
        }
    }

    pub fn inject_error(&self, error: AOCommunicationError) {
        *self.error_injection.write().unwrap() = Some(error);
    }
    pub fn clear_error(&self) {
        *self.error_injection.write().unwrap() = None;
    }
    /// Makes the next `execute()` call return `ok: false` with the given reason.
    pub fn inject_ok_false(&self, reason: impl Into<String>) {
        *self.ok_false_injection.write().unwrap() = Some(reason.into());
    }

    pub fn get_stored_kfrags(&self, process_id: &str) -> Vec<(String, Vec<u8>)> {
        let s = self.kfrag_storage.read().unwrap();
        s.get(process_id)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }
    pub fn get_stored_cfrags(&self, process_id: &str) -> Vec<(String, Vec<u8>)> {
        let s = self.cfrag_storage.read().unwrap();
        s.get(process_id)
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }
    pub fn get_stored_capsules(&self, process_id: &str) -> Vec<(String, String, Vec<u8>)> {
        let s = self.capsule_storage.read().unwrap();
        s.get(process_id)
            .map(|m| {
                m.iter()
                    .map(|(cid, (kid, d))| (cid.clone(), kid.clone(), d.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn clear(&self) {
        self.kfrag_storage.write().unwrap().clear();
        self.cfrag_storage.write().unwrap().clear();
        self.capsule_storage.write().unwrap().clear();
        self.kfrag_holders.write().unwrap().clear();
    }

    fn check_error_injection(&self) -> Option<AOCommunicationError> {
        self.error_injection.write().unwrap().take()
    }
    fn check_ok_false_injection(&self) -> Option<String> {
        self.ok_false_injection.write().unwrap().take()
    }
    fn store_kfrag(&self, process_id: &str, kfrag_id: &str, data: Vec<u8>) {
        self.kfrag_storage
            .write()
            .unwrap()
            .entry(process_id.to_string())
            .or_default()
            .insert(kfrag_id.to_string(), data);
    }
    fn store_capsule(&self, process_id: &str, kfrag_id: &str, capsule_id: &str, data: Vec<u8>) {
        self.capsule_storage
            .write()
            .unwrap()
            .entry(process_id.to_string())
            .or_default()
            .insert(capsule_id.to_string(), (kfrag_id.to_string(), data));
    }
    fn store_cfrag(&self, process_id: &str, kfrag_id: &str, capsule_id: &str, data: Vec<u8>) {
        let key = format!("{kfrag_id}:{capsule_id}");
        self.cfrag_storage
            .write()
            .unwrap()
            .entry(process_id.to_string())
            .or_default()
            .insert(key, data);
    }
    fn get_cfrag(&self, process_id: &str, kfrag_id: &str, capsule_id: &str) -> Option<Vec<u8>> {
        let key = format!("{kfrag_id}:{capsule_id}");
        let s = self.cfrag_storage.read().unwrap();
        s.get(process_id).and_then(|m| m.get(&key).cloned())
    }
}

impl Default for MockAOClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Real Umbral proxy re-encryption (ported from ao/contracts/src/handlers.rs:402-464).
/// Takes bincode-serialized StoredKeyFragPayload and raw Capsule bytes.
/// Returns raw CapsuleFrag bytes (suitable for `CapsuleFrag::from_bytes`).
fn perform_reencryption(
    kfrag_bytes: &[u8],
    capsule_bytes: &[u8],
) -> Result<Vec<u8>, AOCommunicationError> {
    if kfrag_bytes.is_empty() {
        return Err(AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Empty kFrag payload".to_string(),
        });
    }
    if capsule_bytes.is_empty() {
        return Err(AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Empty capsule payload".to_string(),
        });
    }

    let stored_kfrag: MockStoredKeyFrag =
        bincode::deserialize(kfrag_bytes).map_err(|e| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: format!("KFrag deserialize failed: {e}"),
        })?;

    if stored_kfrag.key_data.is_empty() {
        return Err(AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Invalid kFrag data".to_string(),
        });
    }
    if stored_kfrag.verification_data.is_empty() {
        return Err(AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Missing verification data".to_string(),
        });
    }

    let verification: MockVerificationData = bincode::deserialize(&stored_kfrag.verification_data)
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Invalid verification payload".to_string(),
        })?;

    let verifying_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.verifying_pk)
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Invalid verifying key".to_string(),
        })?;

    let delegating_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.delegating_pk)
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Invalid delegating key".to_string(),
        })?;

    let receiving_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.receiving_pk)
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Invalid receiving key".to_string(),
        })?;

    let key_frag = umbral_pre::KeyFrag::from_bytes(&stored_kfrag.key_data).map_err(|_| {
        AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Failed to deserialize kFrag".to_string(),
        }
    })?;

    let verified_kfrag = key_frag
        .verify(&verifying_pk, Some(&delegating_pk), Some(&receiving_pk))
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "kFrag verification failed".to_string(),
        })?;

    let capsule: umbral_pre::Capsule =
        umbral_pre::Capsule::from_bytes(capsule_bytes).map_err(|e| {
            AOCommunicationError::ExecutionError {
                process_id: String::new(),
                details: format!("Capsule deserialize failed: {e:?}"),
            }
        })?;

    let verified_cfrag = umbral_pre::reencrypt(&capsule, verified_kfrag);
    let cfrag = verified_cfrag.unverify();
    let cfrag_bytes = cfrag
        .to_bytes()
        .map_err(|_| AOCommunicationError::ExecutionError {
            process_id: String::new(),
            details: "Failed to serialize cFrag".to_string(),
        })?;

    Ok(cfrag_bytes.to_vec())
}

/// Extract a string field from `msg.data` JSON.
fn data_str<'a>(data: &'a serde_json::Value, field: &str) -> Result<&'a str, AOCommunicationError> {
    data.get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AOCommunicationError::ValidationError {
            details: format!("missing field: {field}"),
        })
}
/// Extract bytes from `msg.data[field]` (JSON array of u8 values, 0..=255).
/// Returns an error if the field is missing, not an array, or contains out-of-range values.
fn data_bytes(data: &serde_json::Value, field: &str) -> Result<Vec<u8>, AOCommunicationError> {
    let arr = data.get(field).and_then(|v| v.as_array()).ok_or_else(|| {
        AOCommunicationError::ValidationError {
            details: format!("missing or non-array field: {field}"),
        }
    })?;

    arr.iter()
        .enumerate()
        .map(|(i, v)| {
            let n = v
                .as_u64()
                .ok_or_else(|| AOCommunicationError::ValidationError {
                    details: format!("field {field}[{i}] is not a number"),
                })?;
            if n > 255 {
                return Err(AOCommunicationError::ValidationError {
                    details: format!("field {field}[{i}] = {n} is out of u8 range (0..=255)"),
                });
            }
            Ok(n as u8)
        })
        .collect()
}

#[async_trait]
impl AOClient for MockAOClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }
        if let Some(reason) = self.check_ok_false_injection() {
            return Ok(AONativeResponse::error_response(reason));
        }
        if process_id.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "empty process ID".into(),
            });
        }

        let d = msg.data();
        match msg.action() {
            "DelegateKFrag" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let kfrag = data_bytes(d, "kfrag")?;
                let holder_process_id = data_str(d, "holder_process_id")?;
                self.store_kfrag(process_id, kfrag_id, kfrag);
                self.kfrag_holders
                    .write()
                    .unwrap()
                    .entry(process_id.to_string())
                    .or_default()
                    .insert(kfrag_id.to_string(), holder_process_id.to_string());
                Ok(AONativeResponse::success(
                    serde_json::json!({ "kfrag_id": kfrag_id, "delegated": true }),
                ))
            }
            "DelegateCapsule" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let capsule_id = data_str(d, "capsule_id")?;
                let capsule = data_bytes(d, "capsule")?;
                let holder_process_id = data_str(d, "holder_process_id")?;
                // Mirror contract: verify holder was registered via DelegateKFrag
                {
                    let holders = self.kfrag_holders.read().unwrap();
                    let registered = holders.get(process_id).and_then(|m| m.get(kfrag_id));
                    if registered.is_none() {
                        return Err(AOCommunicationError::ExecutionError {
                            process_id: process_id.to_string(),
                            details: format!("No holder registered for kfrag_id: {kfrag_id}"),
                        });
                    }
                }
                self.store_capsule(holder_process_id, kfrag_id, capsule_id, capsule.clone());

                // Perform real Umbral re-encryption (replacing dummy cFrag)
                let kfrag_bytes = {
                    let storage = self.kfrag_storage.read().unwrap();
                    storage
                        .get(process_id)
                        .and_then(|m| m.get(kfrag_id))
                        .cloned()
                };
                match kfrag_bytes {
                    Some(kb) => {
                        let cfrag = perform_reencryption(&kb, &capsule)?;
                        self.store_cfrag(holder_process_id, kfrag_id, capsule_id, cfrag);
                        Ok(AONativeResponse::success(
                            serde_json::json!({ "capsule_id": capsule_id, "cfrag_ready": true }),
                        ))
                    }
                    None => Err(AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: format!("KFrag not found for re-encryption: {kfrag_id}"),
                    }),
                }
            }
            "SubmitKFrag" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let kfrag = data_bytes(d, "kfrag")?;
                self.store_kfrag(process_id, kfrag_id, kfrag);
                Ok(AONativeResponse::success(
                    serde_json::json!({ "kfrag_id": kfrag_id, "stored": true }),
                ))
            }
            "SubmitCapsule" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let capsule_id = data_str(d, "capsule_id")?;
                let capsule = data_bytes(d, "capsule")?;
                self.store_capsule(process_id, kfrag_id, capsule_id, capsule.clone());

                // Perform real Umbral re-encryption (no dummy fallback)
                let kfrag_bytes = {
                    let storage = self.kfrag_storage.read().unwrap();
                    storage
                        .get(process_id)
                        .and_then(|m| m.get(kfrag_id))
                        .cloned()
                };
                match kfrag_bytes {
                    Some(kb) => {
                        let cfrag = perform_reencryption(&kb, &capsule)?;
                        self.store_cfrag(process_id, kfrag_id, capsule_id, cfrag);
                        Ok(AONativeResponse::success(
                            serde_json::json!({ "capsule_id": capsule_id, "cfrag_ready": true }),
                        ))
                    }
                    None => Err(AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: format!("KFrag not found for re-encryption: {kfrag_id}"),
                    }),
                }
            }
            "Reencrypt" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let capsule_id = data_str(d, "capsule_id")?;

                // Look up kfrag and capsule, then perform real re-encryption
                let kfrag_bytes = {
                    let storage = self.kfrag_storage.read().unwrap();
                    storage
                        .get(process_id)
                        .and_then(|m| m.get(kfrag_id))
                        .cloned()
                };
                let capsule_bytes = {
                    let storage = self.capsule_storage.read().unwrap();
                    storage
                        .get(process_id)
                        .and_then(|m| m.get(capsule_id))
                        .map(|(_, data)| data.clone())
                };

                match (kfrag_bytes, capsule_bytes) {
                    (Some(kb), Some(cb)) => {
                        let cfrag = perform_reencryption(&kb, &cb)?;
                        self.store_cfrag(process_id, kfrag_id, capsule_id, cfrag);
                        Ok(AONativeResponse::success(
                            serde_json::json!({ "capsule_id": capsule_id, "cfrag_ready": true }),
                        ))
                    }
                    (None, _) => Err(AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: format!("KFrag not found: {kfrag_id}"),
                    }),
                    (_, None) => Err(AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: format!("Capsule not found: {capsule_id}"),
                    }),
                }
            }
            other => Err(AOCommunicationError::ExecutionError {
                process_id: process_id.to_string(),
                details: format!("Unknown action: {other}"),
            }),
        }
    }

    async fn query(
        &self,
        process_id: &str,
        msg: AOQueryMsg,
    ) -> Result<Binary, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }
        if process_id.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "empty process ID".into(),
            });
        }

        let d = msg.data();
        match msg.action() {
            "GetCFrag" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                let capsule_id = data_str(d, "capsule_id")?;
                let cfrag_data = self
                    .get_cfrag(process_id, kfrag_id, capsule_id)
                    .ok_or_else(|| AOCommunicationError::ExecutionError {
                        process_id: process_id.to_string(),
                        details: "CFrag not ready".to_string(),
                    })?;
                let response = GetCFragResponse {
                    kfrag_id: kfrag_id.to_string(),
                    capsule_id: capsule_id.to_string(),
                    cfrag: cfrag_data,
                };
                let json = serde_json::to_vec(&response).map_err(|e| {
                    AOCommunicationError::SerializationError {
                        details: e.to_string(),
                    }
                })?;
                Ok(Binary::from(json))
            }
            "GetCFrags" => {
                let capsule_id = data_str(d, "capsule_id")?;
                let s = self.cfrag_storage.read().unwrap();
                // mock key format: "{kfrag_id}:{capsule_id}"
                let suffix = format!(":{capsule_id}");
                let mut cfrags: Vec<CFragEntry> = s
                    .get(process_id)
                    .map(|m| {
                        m.iter()
                            .filter_map(|(key, bytes)| {
                                let kfrag_id = key.strip_suffix(&suffix)?;
                                Some(CFragEntry {
                                    kfrag_id: kfrag_id.to_string(),
                                    cfrag: bytes.clone(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                cfrags.sort_by(|a, b| a.kfrag_id.cmp(&b.kfrag_id));
                let response = GetCFragsResponse {
                    capsule_id: capsule_id.to_string(),
                    cfrags,
                };
                let json = serde_json::to_vec(&response).map_err(|e| {
                    AOCommunicationError::SerializationError {
                        details: e.to_string(),
                    }
                })?;
                Ok(Binary::from(json))
            }
            "ListCapsules" => {
                let kfrag_id = data_str(d, "kfrag_id")?;
                // Optional pagination params
                let start_after = d.get("start_after").and_then(|v| v.as_str());
                let limit = d.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

                let s = self.capsule_storage.read().unwrap();
                let mut capsule_ids: Vec<String> = s
                    .get(process_id)
                    .map(|m| {
                        m.iter()
                            .filter(|(_, (kid, _))| kid.as_str() == kfrag_id)
                            .map(|(cid, _)| cid.clone())
                            .collect()
                    })
                    .unwrap_or_default();

                // Deterministic order + pagination (matches production contract behaviour)
                capsule_ids.sort();
                if let Some(start) = start_after {
                    capsule_ids.retain(|id| id.as_str() > start);
                }
                capsule_ids.truncate(limit);
                let next_start_after = capsule_ids.last().cloned();

                // Response format mirrors the production contract:
                // { kfrag_id, capsule_ids, next_start_after }
                let json = serde_json::to_vec(&serde_json::json!({
                    "kfrag_id": kfrag_id,
                    "capsule_ids": capsule_ids,
                    "next_start_after": next_start_after,
                }))
                .map_err(|e| AOCommunicationError::SerializationError {
                    details: e.to_string(),
                })?;
                Ok(Binary::from(json))
            }
            other => Err(AOCommunicationError::ExecutionError {
                process_id: process_id.to_string(),
                details: format!("Unknown query action: {other}"),
            }),
        }
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: AOExecuteMsg,
    ) -> Result<AONativeResponse, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }
        if process_id.is_empty() {
            return Err(AOCommunicationError::ValidationError {
                details: "empty process ID".into(),
            });
        }
        // Dry-run: simulate without side-effects
        Ok(AONativeResponse::success(
            serde_json::json!({ "dry_run": true, "action": msg.action() }),
        ))
    }
}
