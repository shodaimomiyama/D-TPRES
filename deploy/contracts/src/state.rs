use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

// Owner-Process専用ストレージ
pub const OWNER_KFRAGS: Map<String, OwnerKFragData> = Map::new("owner_kfrags");
pub const HOLDER_ASSIGNMENTS: Map<String, HolderAssignment> = Map::new("holder_assignments");
pub const OWNER_METADATA: Item<OwnerMetadata> = Item::new("owner_metadata");
pub const OWNER_CONFIG: Item<OwnerConfig> = Item::new("owner_config");

// Holder-Process専用ストレージ
pub const HOLDER_KFRAGS: Map<String, HolderKFragData> = Map::new("holder_kfrags");
pub const HOLDER_CFRAGS: Map<String, HolderCFragData> = Map::new("holder_cfrags");
pub const CAPSULE_CACHE: Map<String, CapsuleData> = Map::new("capsule_cache");
pub const HOLDER_METADATA: Item<HolderMetadata> = Item::new("holder_metadata");

// Requester-Process専用ストレージ
pub const CFRAG_COLLECTION: Map<String, CFragCollection> = Map::new("cfrag_collection");
pub const RECOVERY_SESSIONS: Map<String, RecoverySession> = Map::new("recovery_sessions");
pub const THRESHOLD_TRACKER: Item<ThresholdInfo> = Item::new("threshold_tracker");
pub const REQUESTER_METADATA: Item<RequesterMetadata> = Item::new("requester_metadata");

// AO Network プロセス管理ストレージ
pub const CONNECTED_PROCESSES: Map<String, AOProcessInfo> = Map::new("connected_processes");
pub const PROCESS_REGISTRY: Item<ProcessRegistry> = Item::new("process_registry");
pub const MESSAGE_QUEUE: Map<String, PendingMessage> = Map::new("message_queue");

// プロセス役割定義
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ProcessRole {
    Owner,
    Holder,
    Requester,
}

// Owner-Process データ構造
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct OwnerKFragData {
    pub kfrag_id: String,
    pub encrypted_kfrag: Vec<u8>, // AES-GCM暗号化済み
    #[zeroize(skip)]
    pub target_holder: String,
    #[zeroize(skip)]
    pub created_at: u64,
    #[zeroize(skip)]
    pub distributed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderAssignment {
    pub holder_id: String,
    pub assigned_kfrags: Vec<String>,
    pub assignment_time: u64,
    pub status: AssignmentStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AssignmentStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OwnerMetadata {
    pub owner_id: String,
    pub total_holders_n: u32,        // RandAOでHolder選出に必要
    pub creation_time: u64,
    pub signer_pubkey: String,       // O-Browserの署名検証用公開鍵
}

impl Default for OwnerMetadata {
    fn default() -> Self {
        Self {
            owner_id: String::new(),
            total_holders_n: 0,
            creation_time: 0,
            signer_pubkey: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OwnerConfig {
    pub process_role: ProcessRole,  // プロセスロール識別のみ
}

// Holder-Process データ構造
#[derive(Serialize, Deserialize, Clone, Debug, Zeroize, ZeroizeOnDrop)]
pub struct HolderKFragData {
    pub kfrag_id: String,
    #[zeroize(skip)]
    pub source_owner: String,
    pub encrypted_kfrag: Vec<u8>, // 暗号化されたkFragment
    pub signature: Vec<u8>,
    #[zeroize(skip)]
    pub received_at: u64,
    #[zeroize(skip)]
    pub processed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderCFragData {
    pub cfrag_id: String,
    pub source_kfrag: String,
    pub cfrag_data: Vec<u8>,
    pub arweave_txid: Option<String>,
    pub generated_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapsuleData {
    pub capsule_id: String,
    pub capsule_bytes: Vec<u8>,
    pub arweave_txid: String,
    pub cached_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderMetadata {
    pub holder_id: String,
    pub process_role: ProcessRole,
    pub assigned_owners: Vec<String>,
    pub initialization_time: u64,
}

impl Default for HolderMetadata {
    fn default() -> Self {
        Self {
            holder_id: String::new(),
            process_role: ProcessRole::Holder,
            assigned_owners: Vec::new(),
            initialization_time: 0,
        }
    }
}

// Requester-Process データ構造
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragCollection {
    pub session_id: String,
    pub collected_cfrags: Vec<CollectedCFrag>,
    pub target_threshold: u32,
    pub collection_started: u64,
    pub status: CollectionStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectedCFrag {
    pub cfrag_id: String,
    pub holder_id: String,
    pub cfrag_data: Vec<u8>,
    pub collected_at: u64,
    pub verified: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum CollectionStatus {
    InProgress,
    ThresholdMet,
    Completed,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoverySession {
    pub session_id: String,
    pub capsule_data: Vec<u8>,
    pub collected_cfrags: Vec<String>,
    pub threshold_met: bool,
    pub recovery_completed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThresholdInfo {
    pub required_threshold: u32,
    pub current_collected: u32,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequesterMetadata {
    pub requester_id: String,
    pub process_role: ProcessRole,
    pub active_sessions: Vec<String>,
    pub initialization_time: u64,
}

impl Default for RequesterMetadata {
    fn default() -> Self {
        Self {
            requester_id: String::new(),
            process_role: ProcessRole::Requester,
            active_sessions: Vec::new(),
            initialization_time: 0,
        }
    }
}

// 共通エラー型
#[derive(thiserror::Error, Debug)]
pub enum StorageError {
    #[error("Storage operation failed: {reason}")]
    StorageFailure { reason: String },

    #[error("Data not found: {key}")]
    DataNotFound { key: String },

    #[error("Invalid data format")]
    InvalidFormat,

    #[error("Unauthorized access")]
    Unauthorized,
}

impl ToString for AssignmentStatus {
    fn to_string(&self) -> String {
        match self {
            AssignmentStatus::Pending => "pending".to_string(),
            AssignmentStatus::Confirmed => "confirmed".to_string(),
            AssignmentStatus::Failed => "failed".to_string(),
        }
    }
}

impl ToString for CollectionStatus {
    fn to_string(&self) -> String {
        match self {
            CollectionStatus::InProgress => "in_progress".to_string(),
            CollectionStatus::ThresholdMet => "threshold_met".to_string(),
            CollectionStatus::Completed => "completed".to_string(),
            CollectionStatus::Failed => "failed".to_string(),
        }
    }
}

impl ToString for ProcessRole {
    fn to_string(&self) -> String {
        match self {
            ProcessRole::Owner => "owner".to_string(),
            ProcessRole::Holder => "holder".to_string(),
            ProcessRole::Requester => "requester".to_string(),
        }
    }
}

// 状態検証のためのヘルパー関数
impl OwnerKFragData {
    pub fn is_ready_for_distribution(&self) -> bool {
        !self.encrypted_kfrag.is_empty() && !self.target_holder.is_empty()
    }
}

impl HolderKFragData {
    pub fn is_valid(&self) -> bool {
        !self.encrypted_kfrag.is_empty()
            && !self.signature.is_empty()
            && !self.source_owner.is_empty()
    }
}

impl CFragCollection {
    pub fn has_reached_threshold(&self) -> bool {
        self.collected_cfrags.len() >= self.target_threshold as usize
    }

    pub fn verified_cfrags_count(&self) -> usize {
        self.collected_cfrags
            .iter()
            .filter(|cfrag| cfrag.verified)
            .count()
    }
}

// AO Network プロセス管理データ構造
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AOProcessInfo {
    pub process_id: String,
    pub wasm_tx_id: String, // ArweaveのWASMモジュールTXID
    pub process_role: ProcessRole,
    pub spawned_at: u64,
    pub status: AOProcessStatus,
    pub last_heartbeat: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AOProcessStatus {
    Active,
    Inactive,
    Failed,
    Initializing,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessRegistry {
    pub current_process_id: String,
    pub current_role: ProcessRole,
    pub initialization_time: u64,
    pub holder_processes: Vec<String>, // Owner-Processが管理するHolder-Processのリスト
    pub requester_processes: Vec<String>, // システム内のRequester-Processのリスト
    pub owner_process: Option<String>, // Holder/RequesterからみたOwner-Process
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingMessage {
    pub message_id: String,
    pub target_process: String,
    pub message_type: String,
    pub message_data: Vec<u8>,
    pub created_at: u64,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl AOProcessInfo {
    pub fn new(process_id: String, wasm_tx_id: String, role: ProcessRole) -> Self {
        Self {
            process_id,
            wasm_tx_id,
            process_role: role,
            spawned_at: 0, // 実際のタイムスタンプで設定
            status: AOProcessStatus::Initializing,
            last_heartbeat: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == AOProcessStatus::Active
    }

    pub fn update_heartbeat(&mut self, timestamp: u64) {
        self.last_heartbeat = timestamp;
        if self.status == AOProcessStatus::Initializing {
            self.status = AOProcessStatus::Active;
        }
    }
}

impl ProcessRegistry {
    pub fn new(process_id: String, role: ProcessRole) -> Self {
        Self {
            current_process_id: process_id,
            current_role: role,
            initialization_time: 0, // 実際のタイムスタンプで設定
            holder_processes: Vec::new(),
            requester_processes: Vec::new(),
            owner_process: None,
        }
    }

    pub fn add_holder_process(&mut self, holder_id: String) {
        if !self.holder_processes.contains(&holder_id) {
            self.holder_processes.push(holder_id);
        }
    }

    pub fn add_requester_process(&mut self, requester_id: String) {
        if !self.requester_processes.contains(&requester_id) {
            self.requester_processes.push(requester_id);
        }
    }

    pub fn set_owner_process(&mut self, owner_id: String) {
        self.owner_process = Some(owner_id);
    }

    pub fn get_holder_count(&self) -> usize {
        self.holder_processes.len()
    }
}

impl PendingMessage {
    pub fn new(target_process: String, message_type: String, message_data: Vec<u8>) -> Self {
        Self {
            message_id: format!("msg_{}_{}", target_process, generate_message_id()),
            target_process,
            message_type,
            message_data,
            created_at: 0, // 実際のタイムスタンプで設定
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }
}

// ヘルパー関数
fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{:x}", timestamp)
}
