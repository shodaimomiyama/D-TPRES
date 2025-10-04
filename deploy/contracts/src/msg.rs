use serde::{Deserialize, Serialize};
use crate::state::{
    ProcessRole, OwnerMetadata, HolderMetadata, RequesterMetadata,
    CFragCollection, RecoverySession, ThresholdInfo
};

// インスタンス化メッセージ
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub process_role: ProcessRole,
    pub metadata: ProcessMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessMetadata {
    Owner {
        threshold_k: u32,
        total_holders_n: u32,
        capsule_txid: String,
        requester_pubkey: String,
    },
    Holder {
        holder_id: String,
    },
    Requester {
        requester_id: String,
    },
}

// 実行メッセージ
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // Owner-Process メッセージ
    DistributeKFrags {
        kfrags: Vec<KFragDistribution>,
    },
    UpdateHolderAssignment {
        holder_id: String,
        kfrag_ids: Vec<String>,
    },

    // Holder-Process メッセージ
    ReceiveKFrag {
        kfrag_data: KFragReceiptData,
    },
    GenerateCFrag {
        kfrag_id: String,
    },
    StoreCapsule {
        capsule_data: CapsuleStorageData,
    },

    // Requester-Process メッセージ
    StartCFragCollection {
        session_id: String,
        threshold: u32,
    },
    CollectCFrag {
        session_id: String,
        cfrag_data: CFragSubmission,
    },
    InitiateRecovery {
        session_id: String,
        capsule_data: Vec<u8>,
    },

    // AO Network プロセス間メッセージ
    SendKFragToHolder {
        target_process: String,  // Holder ProcessのID
        kfrag: KFragDistribution,
        owner_process: String,   // 送信元Owner ProcessのID
    },
    SendCFragToRequester {
        target_process: String,  // Requester ProcessのID
        cfrag: CFragSubmission,
        holder_process: String,  // 送信元Holder ProcessのID
    },
    RequestCFragFromHolder {
        target_process: String,  // Holder ProcessのID
        session_id: String,
        requester_process: String, // 送信元Requester ProcessのID
    },

    // 共通メッセージ
    UpdateProcessStatus {
        status: String,
    },
}

// クエリメッセージ
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // Owner-Process クエリ
    GetOwnerMetadata {},
    GetKFrags {
        holder_id: Option<String>,
    },
    GetHolderAssignments {},

    // Holder-Process クエリ
    GetHolderMetadata {},
    GetKFragById {
        kfrag_id: String,
    },
    GetCFrags {
        processed_only: Option<bool>,
    },
    GetCachedCapsules {},

    // Requester-Process クエリ
    GetRequesterMetadata {},
    GetCFragCollection {
        session_id: String,
    },
    GetRecoverySession {
        session_id: String,
    },
    GetThresholdInfo {},

    // AO Network プロセス管理クエリ
    GetConnectedProcesses {},
    GetProcessInfo {
        process_id: String,
    },

    // 共通クエリ
    GetProcessRole {},
    GetProcessStatus {},
}

// データ転送オブジェクト
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KFragDistribution {
    pub id: String,
    pub encrypted_data: Vec<u8>,
    pub holder_id: String,
    pub holder_process_id: String,  // AO Network用: Holder ProcessのID
    pub signature: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KFragReceiptData {
    pub id: String,
    pub encrypted_kfrag: Vec<u8>,
    pub signature: Vec<u8>,
    pub owner_id: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CapsuleStorageData {
    pub capsule_id: String,
    pub capsule_bytes: Vec<u8>,
    pub arweave_txid: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CFragSubmission {
    pub cfrag_id: String,
    pub holder_id: String,
    pub holder_process_id: String,  // AO Network用: Holder ProcessのID
    pub cfrag_data: Vec<u8>,
    pub signature: Vec<u8>,
}

// レスポンス型
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OwnerMetadataResponse {
    pub metadata: OwnerMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderMetadataResponse {
    pub metadata: HolderMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequesterMetadataResponse {
    pub metadata: RequesterMetadata,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KFragsResponse {
    pub kfrags: Vec<KFragInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KFragInfo {
    pub kfrag_id: String,
    pub target_holder: String,
    pub created_at: u64,
    pub distributed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragsResponse {
    pub cfrags: Vec<CFragInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragInfo {
    pub cfrag_id: String,
    pub source_kfrag: String,
    pub generated_at: u64,
    pub arweave_txid: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragCollectionResponse {
    pub collection: CFragCollection,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoverySessionResponse {
    pub session: RecoverySession,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ThresholdInfoResponse {
    pub threshold_info: ThresholdInfo,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessRoleResponse {
    pub role: ProcessRole,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessStatusResponse {
    pub status: String,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderAssignmentsResponse {
    pub assignments: Vec<HolderAssignmentInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HolderAssignmentInfo {
    pub holder_id: String,
    pub assigned_kfrags: Vec<String>,
    pub assignment_time: u64,
    pub status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedCapsulesResponse {
    pub capsules: Vec<CapsuleInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CapsuleInfo {
    pub capsule_id: String,
    pub arweave_txid: String,
    pub cached_at: u64,
}

// AO Network用レスポンス型
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectedProcessesResponse {
    pub processes: Vec<ProcessInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessInfo {
    pub process_id: String,
    pub wasm_tx_id: String,
    pub process_role: ProcessRole,
    pub spawned_at: u64,
    pub status: ProcessStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    Active,
    Inactive,
    Failed,
    Initializing,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessInfoResponse {
    pub info: ProcessInfo,
}

// エラーレスポンス
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u32,
}

// 成功レスポンス
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SuccessResponse {
    pub message: String,
    pub data: Option<String>, // JSONを文字列として扱う
}

// バリデーション用のトレイト
pub trait ValidateMessage {
    fn validate(&self) -> Result<(), String>;
}

impl ValidateMessage for InstantiateMsg {
    fn validate(&self) -> Result<(), String> {
        match &self.metadata {
            ProcessMetadata::Owner { threshold_k, total_holders_n, .. } => {
                if *threshold_k == 0 {
                    return Err("Threshold k must be greater than 0".to_string());
                }
                if *total_holders_n == 0 {
                    return Err("Total holders n must be greater than 0".to_string());
                }
                if threshold_k > total_holders_n {
                    return Err("Threshold k cannot exceed total holders n".to_string());
                }
            },
            ProcessMetadata::Holder { holder_id } => {
                if holder_id.is_empty() {
                    return Err("Holder ID cannot be empty".to_string());
                }
            },
            ProcessMetadata::Requester { requester_id } => {
                if requester_id.is_empty() {
                    return Err("Requester ID cannot be empty".to_string());
                }
            },
        }
        Ok(())
    }
}

impl ValidateMessage for KFragDistribution {
    fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("KFrag ID cannot be empty".to_string());
        }
        if self.encrypted_data.is_empty() {
            return Err("Encrypted data cannot be empty".to_string());
        }
        if self.holder_id.is_empty() {
            return Err("Holder ID cannot be empty".to_string());
        }
        if self.signature.is_empty() {
            return Err("Signature cannot be empty".to_string());
        }
        Ok(())
    }
}

impl ValidateMessage for KFragReceiptData {
    fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("KFrag ID cannot be empty".to_string());
        }
        if self.encrypted_kfrag.is_empty() {
            return Err("Encrypted KFrag cannot be empty".to_string());
        }
        if self.signature.is_empty() {
            return Err("Signature cannot be empty".to_string());
        }
        if self.owner_id.is_empty() {
            return Err("Owner ID cannot be empty".to_string());
        }
        Ok(())
    }
}

impl ValidateMessage for CFragSubmission {
    fn validate(&self) -> Result<(), String> {
        if self.cfrag_id.is_empty() {
            return Err("CFrag ID cannot be empty".to_string());
        }
        if self.holder_id.is_empty() {
            return Err("Holder ID cannot be empty".to_string());
        }
        if self.cfrag_data.is_empty() {
            return Err("CFrag data cannot be empty".to_string());
        }
        if self.signature.is_empty() {
            return Err("Signature cannot be empty".to_string());
        }
        Ok(())
    }
}