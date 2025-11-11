use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use cw_storage_plus::{Item, Map};

pub const DEFAULT_LIST_LIMIT: u32 = 50;
pub const DEFAULT_HOLDER_PROCESS_ID: &str = "holder_ABC123XYZ456DEF789GHI012JKL345MNO678PQR";

// --------------------- 設定 ---------------------
#[cw_serde]
pub struct Config {
    pub process_id: String,
    pub default_list_limit: u32,
}
pub const CONFIG: Item<Config> = Item::new("config");

// Owner-Holder 委譲マッピング（将来的にRandAO差し替え予定）
pub const KFRAG_HOLDERS: Map<(String, String), String> = Map::new("kfrag_holders");

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
    Error,
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

// --------------------- マッピング ---------------------
// すべて process_id を先頭キーに含むタプルキー

pub const OWNER_KFRAGS: Map<(String, String), OwnerKFragData> = Map::new("owner_kfrags");

pub const OWNER_CAPSULES: Map<(String, String, String), OwnerCapsuleData> =
    Map::new("owner_capsules");

pub const HOLDER_CFRAGS: Map<(String, String, String), HolderCFragData> = Map::new("holder_cfrags");

pub const INDEX_KFRAG_TO_CAPS: Map<(String, String, String), IndexKFragToCapsValue> =
    Map::new("index_kfrag_to_caps");

pub const INDEX_CAPS_TO_CFRAG: Map<(String, String, String), IndexCapsToCFragValue> =
    Map::new("index_caps_to_cfrag");

pub const IDEM_FLAGS: Map<(String, String, String), IdemFlag> = Map::new("idem_flags");

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
    CFragNotReady {
        kfrag_id: String,
        capsule_id: String,
    },

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

// --------------------- KVユーティリティ ---------------------
pub fn get_current_timestamp(env: &cosmwasm_std::Env) -> String {
    format!("{}Z", env.block.time.seconds())
}

// --------------------- バリデーション ---------------------
pub fn validate_id(id: &str, max_len: usize) -> Result<(), StorageError> {
    if id.is_empty() || id.len() > max_len {
        return Err(StorageError::BadRequest {
            msg: format!("ID must be 1-{} characters", max_len),
        });
    }

    // ASCII安全文字のみ許可
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
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

// tests moved to `tests/`
