//! ReencryptionEntity - Proxy re-encryption process management
//!
//! Manages k-of-n proxy re-encryption process (Phase 4).
//! Tracks cFrag collection and threshold achievement.

use serde::{Deserialize, Serialize};

/// Re-encryption entity - Proxy re-encryption process management
///
/// Created in Phase 4, tracks cFrag collection and re-encryption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReencryptionEntity {
    pub reencryption_id: String,

    pub access_request_id: String,

    pub target_capsule_id: String,

    pub requester_process_id: String,

    // 事前にHolderリストを決定することで、再暗号化プロセスの予測可能性を高め、タイムアウト管理を容易にする
    pub target_holders: Vec<String>,

    pub required_threshold: u8,

    // cFragを直接エンティティ内に保持することで、閘値判定と再暗号化完了チェックを高速化
    pub collected_cfrags: Vec<CFragData>,

    // 文字列ベースのステータス管理により、新しい状態の追加が既存データを破壊しない
    pub status: String,

    pub started_at: u64,

    pub completed_at: Option<u64>,

    // タイムアウトを明示的に設定することで、無応答Holderによるプロセス停滞を防ぎ、システム全体の可用性を向上
    pub timeout_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// cFrag data - Re-encrypted fragment
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CFragData {
    pub cfrag_id: String,

    pub holder_id: String,

    // PRE_ReEnc(kFragj, Capsulei)の結果、暗号化データの再暗号化フラグメント
    pub cfrag_data: Vec<u8>,

    // kFragとcFragの対応関係を明示的に管理することで、再暗号化プロセスの監査とデバッグを容易にする
    pub corresponding_kfrag_id: String,

    pub generated_at: u64,

    // Holderの署名を含めることで、悪意あるHolderによる偽のcFrag投入を防ぎ、システムの信頼性を向上
    pub holder_signature: Vec<u8>,
}
