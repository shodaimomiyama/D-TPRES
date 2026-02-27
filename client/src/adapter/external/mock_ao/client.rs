//! Mock AO Network client implementation
//!
//! Provides MockAOClient for testing and development without
//! actual AO Network connection.

#![allow(clippy::unwrap_used)]
#![allow(clippy::disallowed_names)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::manual_let_else)]

use std::collections::HashMap;
use std::sync::RwLock;

use async_trait::async_trait;

use crate::adapter::errors::AOCommunicationError;
use crate::adapter::external::ao::{
    AOClient, AOEvent, AOResponse, Binary, BlobMeta, CapsuleInfo, CapsuleStatus, ExecuteMsg,
    GetCFragResponse, ListCapsulesByKFragResponse, QueryMsg, ValidateMessage,
};

/// Configuration for MockAOClient behavior
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MockConfig {
    /// Simulated network delay in milliseconds (not applied in current impl)
    pub delay_ms: Option<u64>,
    /// Random failure rate (0.0 to 1.0) - not implemented in current version
    pub fail_rate: Option<f64>,
}

/// Type alias for capsule storage: process_id -> (capsule_id -> (kfrag_id, capsule_data))
type CapsuleStorage = HashMap<String, HashMap<String, (String, Vec<u8>)>>;

/// Mock AO Client for testing and development
///
/// Stores kFrags and cFrags in memory, allowing testing without
/// actual AO Network connection.
pub struct MockAOClient {
    /// Storage: process_id -> (kfrag_id -> kfrag_data)
    kfrag_storage: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
    /// Storage: process_id -> (key -> cfrag_data) where key = "{kfrag_id}:{capsule_id}"
    cfrag_storage: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
    /// Capsule storage using type alias for readability
    capsule_storage: RwLock<CapsuleStorage>,
    /// Configuration
    #[allow(dead_code)]
    config: MockConfig,
    /// Error to inject on next operation
    error_injection: RwLock<Option<AOCommunicationError>>,
}

impl MockAOClient {
    /// Create a new MockAOClient with default configuration
    pub fn new() -> Self {
        Self {
            kfrag_storage: RwLock::new(HashMap::new()),
            cfrag_storage: RwLock::new(HashMap::new()),
            capsule_storage: RwLock::new(HashMap::new()),
            config: MockConfig::default(),
            error_injection: RwLock::new(None),
        }
    }

    /// Create a new MockAOClient with custom configuration
    pub fn with_config(config: MockConfig) -> Self {
        Self {
            kfrag_storage: RwLock::new(HashMap::new()),
            cfrag_storage: RwLock::new(HashMap::new()),
            capsule_storage: RwLock::new(HashMap::new()),
            config,
            error_injection: RwLock::new(None),
        }
    }

    /// Inject an error to be returned on the next operation
    pub fn inject_error(&self, error: AOCommunicationError) {
        let mut injection = self.error_injection.write().unwrap();
        *injection = Some(error);
    }

    /// Clear any injected error
    pub fn clear_error(&self) {
        let mut injection = self.error_injection.write().unwrap();
        *injection = None;
    }

    /// Get all stored kFrags for a process
    pub fn get_stored_kfrags(&self, process_id: &str) -> Vec<(String, Vec<u8>)> {
        let storage = self.kfrag_storage.read().unwrap();
        storage
            .get(process_id)
            .map(|kfrags| {
                kfrags
                    .iter()
                    .map(|(id, data)| (id.clone(), data.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all stored cFrags for a process
    pub fn get_stored_cfrags(&self, process_id: &str) -> Vec<(String, Vec<u8>)> {
        let storage = self.cfrag_storage.read().unwrap();
        storage
            .get(process_id)
            .map(|cfrags| {
                cfrags
                    .iter()
                    .map(|(id, data)| (id.clone(), data.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all stored capsules for a process
    pub fn get_stored_capsules(&self, process_id: &str) -> Vec<(String, String, Vec<u8>)> {
        let storage = self.capsule_storage.read().unwrap();
        storage
            .get(process_id)
            .map(|capsules| {
                capsules
                    .iter()
                    .map(|(capsule_id, (kfrag_id, data))| {
                        (capsule_id.clone(), kfrag_id.clone(), data.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Clear all storage
    pub fn clear(&self) {
        let mut kfrag_storage = self.kfrag_storage.write().unwrap();
        let mut cfrag_storage = self.cfrag_storage.write().unwrap();
        let mut capsule_storage = self.capsule_storage.write().unwrap();
        kfrag_storage.clear();
        cfrag_storage.clear();
        capsule_storage.clear();
    }

    /// Check if an error should be returned
    fn check_error_injection(&self) -> Option<AOCommunicationError> {
        let mut injection = self.error_injection.write().unwrap();
        injection.take()
    }

    /// Store a kFrag
    fn store_kfrag(&self, process_id: &str, kfrag_id: &str, kfrag_data: Vec<u8>) {
        let mut storage = self.kfrag_storage.write().unwrap();
        storage
            .entry(process_id.to_string())
            .or_default()
            .insert(kfrag_id.to_string(), kfrag_data);
    }

    /// Store a capsule
    fn store_capsule(
        &self,
        process_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
        capsule_data: Vec<u8>,
    ) {
        let mut storage = self.capsule_storage.write().unwrap();
        storage
            .entry(process_id.to_string())
            .or_default()
            .insert(capsule_id.to_string(), (kfrag_id.to_string(), capsule_data));
    }

    /// Store a cFrag (result of reencryption)
    fn store_cfrag(&self, process_id: &str, kfrag_id: &str, capsule_id: &str, cfrag_data: Vec<u8>) {
        let key = format!("{kfrag_id}:{capsule_id}");
        let mut storage = self.cfrag_storage.write().unwrap();
        storage
            .entry(process_id.to_string())
            .or_default()
            .insert(key, cfrag_data);
    }

    /// Get a cFrag
    fn get_cfrag(&self, process_id: &str, kfrag_id: &str, capsule_id: &str) -> Option<Vec<u8>> {
        let key = format!("{kfrag_id}:{capsule_id}");
        let storage = self.cfrag_storage.read().unwrap();
        storage
            .get(process_id)
            .and_then(|cfrags| cfrags.get(&key).cloned())
    }

    /// List capsules by kFrag ID
    fn list_capsules_by_kfrag(
        &self,
        process_id: &str,
        kfrag_id: &str,
        start_after: Option<&str>,
        limit: Option<u32>,
    ) -> Vec<(String, CapsuleStatus)> {
        let storage = self.capsule_storage.read().unwrap();
        let capsules = match storage.get(process_id) {
            Some(c) => c,
            None => return vec![],
        };

        let mut result: Vec<_> = capsules
            .iter()
            .filter(|(_, (kid, _))| kid == kfrag_id)
            .map(|(capsule_id, _)| capsule_id.clone())
            .collect();

        result.sort();

        if let Some(start) = start_after {
            result.retain(|id| id.as_str() > start);
        }

        let limit = limit.unwrap_or(100) as usize;
        result.truncate(limit);

        result
            .into_iter()
            .map(|id| (id, CapsuleStatus::Pending))
            .collect()
    }
}

impl Default for MockAOClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AOClient for MockAOClient {
    async fn execute(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }

        if let Err(e) = msg.validate() {
            return Err(AOCommunicationError::validation_error(e));
        }

        if process_id.is_empty() {
            return Err(AOCommunicationError::invalid_process_id("empty process ID"));
        }

        let event = match &msg {
            ExecuteMsg::DelegateKFrag { kfrag_id, kfrag } => {
                self.store_kfrag(process_id, kfrag_id, kfrag.as_slice().to_vec());
                AOEvent::with_attributes(
                    "delegate_kfrag",
                    vec![("kfrag_id", kfrag_id.as_str()), ("process_id", process_id)],
                )
            }
            ExecuteMsg::DelegateCapsule {
                kfrag_id,
                capsule_id,
                capsule,
            } => {
                self.store_capsule(
                    process_id,
                    kfrag_id,
                    capsule_id,
                    capsule.as_slice().to_vec(),
                );
                AOEvent::with_attributes(
                    "delegate_capsule",
                    vec![
                        ("kfrag_id", kfrag_id.as_str()),
                        ("capsule_id", capsule_id.as_str()),
                        ("process_id", process_id),
                    ],
                )
            }
            ExecuteMsg::SubmitKFrag { kfrag_id, kfrag } => {
                self.store_kfrag(process_id, kfrag_id, kfrag.as_slice().to_vec());
                AOEvent::with_attributes(
                    "submit_kfrag",
                    vec![("kfrag_id", kfrag_id.as_str()), ("process_id", process_id)],
                )
            }
            ExecuteMsg::SubmitCapsule {
                kfrag_id,
                capsule_id,
                capsule,
            } => {
                self.store_capsule(
                    process_id,
                    kfrag_id,
                    capsule_id,
                    capsule.as_slice().to_vec(),
                );
                AOEvent::with_attributes(
                    "submit_capsule",
                    vec![
                        ("kfrag_id", kfrag_id.as_str()),
                        ("capsule_id", capsule_id.as_str()),
                        ("process_id", process_id),
                    ],
                )
            }
            ExecuteMsg::Reencrypt {
                kfrag_id,
                capsule_id,
            } => {
                // Simulate reencryption by storing a dummy cFrag
                let dummy_cfrag = format!("cfrag:{kfrag_id}:{capsule_id}").into_bytes();
                self.store_cfrag(process_id, kfrag_id, capsule_id, dummy_cfrag);
                AOEvent::with_attributes(
                    "reencrypt",
                    vec![
                        ("kfrag_id", kfrag_id.as_str()),
                        ("capsule_id", capsule_id.as_str()),
                        ("process_id", process_id),
                    ],
                )
            }
        };

        Ok(AOResponse::success_with_events(vec![event]))
    }

    async fn query(&self, process_id: &str, msg: QueryMsg) -> Result<Binary, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }

        if let Err(e) = msg.validate() {
            return Err(AOCommunicationError::validation_error(e));
        }

        if process_id.is_empty() {
            return Err(AOCommunicationError::invalid_process_id("empty process ID"));
        }

        match &msg {
            QueryMsg::GetCFrag {
                kfrag_id,
                capsule_id,
            } => {
                let cfrag_data = self
                    .get_cfrag(process_id, kfrag_id, capsule_id)
                    .ok_or_else(|| {
                        AOCommunicationError::execution_error(process_id, "CFrag not ready")
                    })?;
                let response = GetCFragResponse {
                    cfrag: Binary::from(cfrag_data),
                    meta: BlobMeta {
                        size: 0,
                        created_at: "2024-01-01T00:00:00Z".to_string(),
                        content_type: Some("application/octet-stream".to_string()),
                    },
                };
                let json = serde_json::to_vec(&response)
                    .map_err(|e| AOCommunicationError::serialization_error(e.to_string()))?;
                Ok(Binary::from(json))
            }
            QueryMsg::ListCapsulesByKFrag {
                kfrag_id,
                start_after,
                limit,
            } => {
                let capsules = self.list_capsules_by_kfrag(
                    process_id,
                    kfrag_id,
                    start_after.as_deref(),
                    *limit,
                );
                let last_capsule_id = capsules.last().map(|(id, _)| id.clone());
                let response = ListCapsulesByKFragResponse {
                    capsules: capsules
                        .into_iter()
                        .map(|(id, status)| CapsuleInfo {
                            capsule_id: id,
                            status,
                            updated_ts: "2024-01-01T00:00:00Z".to_string(),
                        })
                        .collect(),
                    next_start_after: last_capsule_id,
                };
                let json = serde_json::to_vec(&response)
                    .map_err(|e| AOCommunicationError::serialization_error(e.to_string()))?;
                Ok(Binary::from(json))
            }
        }
    }

    async fn dry_run(
        &self,
        process_id: &str,
        msg: ExecuteMsg,
    ) -> Result<AOResponse, AOCommunicationError> {
        if let Some(err) = self.check_error_injection() {
            return Err(err);
        }

        if let Err(e) = msg.validate() {
            return Err(AOCommunicationError::validation_error(e));
        }

        if process_id.is_empty() {
            return Err(AOCommunicationError::invalid_process_id("empty process ID"));
        }

        // Dry run simulates execution without persisting changes
        let event_type = match &msg {
            ExecuteMsg::DelegateKFrag { .. } => "dry_run:delegate_kfrag",
            ExecuteMsg::DelegateCapsule { .. } => "dry_run:delegate_capsule",
            ExecuteMsg::SubmitKFrag { .. } => "dry_run:submit_kfrag",
            ExecuteMsg::SubmitCapsule { .. } => "dry_run:submit_capsule",
            ExecuteMsg::Reencrypt { .. } => "dry_run:reencrypt",
        };

        Ok(AOResponse::success_with_events(vec![
            AOEvent::with_attributes(event_type, vec![("process_id", process_id)]),
        ]))
    }
}
