use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};

pub const DEFAULT_CHUNK_THRESHOLD: u32 = 4 * 1024;
pub const MAX_CHUNKS: u32 = 32;
pub const DEFAULT_LIST_LIMIT: u32 = 50;

// --------------------- 設定 ---------------------
#[cw_serde]
pub struct Config {
    pub process_id: String,
    pub chunk_threshold: u32,
    pub max_chunks: u32,
    pub default_list_limit: u32,
}
pub const CONFIG: Item<Config> = Item::new("config");

// --------------------- メタ ---------------------
pub type Rfc3339String = String;

#[cw_serde]
pub struct BlobMeta {
    pub size_bytes: u64,
    pub sha256_hex: String,
    pub updated_ts: Rfc3339String,
}

// --------------------- ドメイン ---------------------
#[cw_serde]
pub enum CapsuleStatus {
    Received,
    ReencInProgress,
    CFragReady,
    Error
}

#[cw_serde]
pub struct OwnerKFragData {
    pub kfrag: Binary,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct OwnerCapsuleData {
    pub capsule: Binary,
    pub status: CapsuleStatus,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct HolderCFragData {
    pub cfrag: Binary,
    pub meta: BlobMeta,
}

#[cw_serde]
pub struct IndexKFragToCapsValue {
    pub status: CapsuleStatus,
    pub updated_ts: Rfc3339String,
}

#[cw_serde]
pub struct IndexCapsToCFragValue {
    pub exists: bool,
    pub updated_ts: Rfc3339String,
}

#[cw_serde]
pub struct IdemFlag {
    pub generated: bool,
    pub updated_ts: Rfc3339String,
}

#[cw_serde]
pub struct AuditLogEntry {
    pub event: String,
    pub actor_wallet: String,
    pub kfrag_id: String,
    pub capsule_id: String,
    pub reason_or_meta: String,
    pub ext_ts: Rfc3339String,
}

#[cw_serde]
pub struct ChunkMeta {
    pub size_bytes: u64,
    pub parts: u32,
    pub sha256_hex: String,
    pub updated_ts: Rfc3339String,
}

// --------------------- マッピング ---------------------
// すべて process_id を先頭キーに含むタプルキー

pub const OWNER_KFRAGS: Map<(String, String), OwnerKFragData> =
    Map::new("owner_kfrags");

pub const OWNER_CAPSULES: Map<(String, String, String), OwnerCapsuleData> =
    Map::new("owner_capsules");

pub const HOLDER_CFRAGS: Map<(String, String, String), HolderCFragData> =
    Map::new("holder_cfrags");

pub const INDEX_KFRAG_TO_CAPS: Map<(String, String, String), IndexKFragToCapsValue> =
    Map::new("index_kfrag_to_caps");

pub const INDEX_CAPS_TO_CFRAG: Map<(String, String, String), IndexCapsToCFragValue> =
    Map::new("index_caps_to_cfrag");

pub const IDEM_FLAGS: Map<(String, String, String), IdemFlag> =
    Map::new("idem_flags");

pub const AUDIT_LOGS: Map<String, AuditLogEntry> =
    Map::new("audit_logs");

pub const CHUNK_META: Map<(String, String, String), ChunkMeta> =
    Map::new("chunk_meta");

pub const CHUNK_PARTS: Map<(String, String, String, u32), Binary> =
    Map::new("chunk_parts");

// --------------------- エラー型 ---------------------
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

    #[error("KFrag not found: {kfrag_id}")]
    KFragNotFound { kfrag_id: String },

    #[error("CFrag not ready: {kfrag_id}/{capsule_id}")]
    CFragNotReady { kfrag_id: String, capsule_id: String },

    #[error("Reencryption failed: {reason}")]
    ReencryptionFailed { reason: String },

    #[error("Object too large: {size} bytes")]
    ObjectTooLarge { size: u64 },

    #[error("Bad request: {msg}")]
    BadRequest { msg: String },
}

// --------------------- ヘルパー実装 ---------------------
impl ToString for CapsuleStatus {
    fn to_string(&self) -> String {
        match self {
            CapsuleStatus::Received => "received".to_string(),
            CapsuleStatus::ReencInProgress => "reenc_in_progress".to_string(),
            CapsuleStatus::CFragReady => "cfrag_ready".to_string(),
            CapsuleStatus::Error => "error".to_string(),
        }
    }
}

impl OwnerKFragData {
    pub fn new(kfrag: Binary, timestamp: String) -> Self {
        let meta = BlobMeta {
            size_bytes: kfrag.len() as u64,
            sha256_hex: sha256::digest(kfrag.as_slice()).to_string(),
            updated_ts: timestamp,
        };
        Self { kfrag, meta }
    }
}

impl OwnerCapsuleData {
    pub fn new(capsule: Binary, timestamp: String) -> Self {
        let meta = BlobMeta {
            size_bytes: capsule.len() as u64,
            sha256_hex: sha256::digest(capsule.as_slice()).to_string(),
            updated_ts: timestamp.clone(),
        };
        Self {
            capsule,
            status: CapsuleStatus::Received,
            meta,
        }
    }

    pub fn update_status(&mut self, status: CapsuleStatus, timestamp: String) {
        self.status = status;
        self.meta.updated_ts = timestamp;
    }
}

impl HolderCFragData {
    pub fn new(cfrag: Binary, timestamp: String) -> Self {
        let meta = BlobMeta {
            size_bytes: cfrag.len() as u64,
            sha256_hex: sha256::digest(cfrag.as_slice()).to_string(),
            updated_ts: timestamp,
        };
        Self { cfrag, meta }
    }
}

impl IndexKFragToCapsValue {
    pub fn new(status: CapsuleStatus, timestamp: String) -> Self {
        Self {
            status,
            updated_ts: timestamp,
        }
    }
}

impl IndexCapsToCFragValue {
    pub fn new(timestamp: String) -> Self {
        Self {
            exists: true,
            updated_ts: timestamp,
        }
    }
}

impl IdemFlag {
    pub fn new(timestamp: String) -> Self {
        Self {
            generated: true,
            updated_ts: timestamp,
        }
    }
}

impl AuditLogEntry {
    pub fn new(
        event: String,
        actor_wallet: String,
        kfrag_id: String,
        capsule_id: String,
        reason_or_meta: String,
        ext_ts: String,
    ) -> Self {
        Self {
            event,
            actor_wallet,
            kfrag_id,
            capsule_id,
            reason_or_meta,
            ext_ts,
        }
    }
}

// --------------------- KVユーティリティ ---------------------
pub fn get_current_timestamp(env: &cosmwasm_std::Env) -> String {
    let timestamp = env.block.time.seconds();
    format!("{}Z", timestamp) // 簡易RFC3339形式
}

pub fn generate_audit_key(
    ext_ts: &str,
    event: &str,
    process_id: &str,
    kfrag_id: &str,
    capsule_id: &str,
) -> String {
    format!("audit|{}|{}|{}|{}|{}", ext_ts, event, process_id, kfrag_id, capsule_id)
}

// --------------------- バリデーション ---------------------
pub fn validate_id(id: &str, max_len: usize) -> Result<(), StorageError> {
    if id.is_empty() || id.len() > max_len {
        return Err(StorageError::BadRequest {
            msg: format!("ID must be 1-{} characters", max_len),
        });
    }

    // ASCII安全文字のみ許可
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(StorageError::BadRequest {
            msg: "ID must contain only ASCII alphanumeric, underscore, or hyphen".to_string(),
        });
    }

    Ok(())
}

pub fn validate_base64_data(data: &[u8]) -> Result<(), StorageError> {
    if data.is_empty() {
        return Err(StorageError::BadRequest {
            msg: "Data cannot be empty".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Binary;

    #[test]
    fn test_validate_id() {
        assert!(validate_id("valid_id-123", 128).is_ok());
        assert!(validate_id("", 128).is_err());
        assert!(validate_id("a".repeat(129).as_str(), 128).is_err());
        assert!(validate_id("invalid@id", 128).is_err());
    }

    #[test]
    fn test_owner_kfrag_data_creation() {
        let kfrag = Binary::from(b"test_kfrag_data");
        let timestamp = "2025-10-17T12:00:00Z".to_string();
        let data = OwnerKFragData::new(kfrag.clone(), timestamp.clone());

        assert_eq!(data.kfrag, kfrag);
        assert_eq!(data.meta.size_bytes, kfrag.len() as u64);
        assert_eq!(data.meta.updated_ts, timestamp);
    }

    #[test]
    fn test_capsule_status_update() {
        let capsule = Binary::from(b"test_capsule_data");
        let timestamp = "2025-10-17T12:00:00Z".to_string();
        let mut data = OwnerCapsuleData::new(capsule, timestamp);

        assert!(matches!(data.status, CapsuleStatus::Received));

        data.update_status(CapsuleStatus::CFragReady, "2025-10-17T12:01:00Z".to_string());
        assert!(matches!(data.status, CapsuleStatus::CFragReady));
    }
}