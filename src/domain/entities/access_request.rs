//! AccessRequestEntity - Access request and verification
//!
//! Manages data access requests and EVM verification (Phase 2).
//! Central to access control and permission validation.

use serde::{Deserialize, Serialize};

use super::value_objects::AccessRequestStatus;

/// Access request entity - Data access request and verification
///
/// Created in Phase 2, goes through EVM verification to proceed to Phase 3
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccessRequestEntity {
    pub request_id: String,

    pub target_data_id: String,

    pub target_secret_id: String,

    pub requester_process_id: String,

    pub accessor_public_key: Vec<u8>,

    pub owner_public_key: Vec<u8>,

    // EVM検証結果を直接含めることで、外部参照を減らし、
    // AOのステートレス環境でのメッセージ処理を効率化
    pub evm_verification: EvmVerificationData,

    // elciao生成のProofPkgはオプショナルにすることで、
    // EVM検証方式の将来的な変更に柔軟に対応
    pub proof_pkg: Option<ProofPkgData>,

    // 型安全なステータス管理により、
    // 不正な状態遷移を防止
    pub status: AccessRequestStatus,

    pub created_at: u64,

    pub evm_verified_at: Option<u64>,

    pub completed_at: Option<u64>,

    // タイムアウトを明示的に設定することで、
    // リソースリークを防ぎ、ネットワークの健全性を維持
    pub timeout_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// EVM verification data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvmVerificationData {
    pub tx_hash: String,

    pub contract_address: String,

    // イベントデータをバイト配列で保存することで、
    // 様々なイベント形式に対応し、後方互換性を維持
    pub verification_event: Vec<u8>,

    // ブロック高を保存してフォークへの対処を可能にし、
    // 検証の確定性を後から確認可能にする
    pub block_height: u64,

    pub verified_at: u64,
}

/// ProofPkg data - Verification package by elciao
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofPkgData {
    pub proof_pkg_id: String,

    // BlockHeader + Receipt + pkAを単一のバイト配列にパックすることで、
    // 転送効率を上げ、検証時のパース処理を最小化
    pub proof_package: Vec<u8>,

    pub created_at: u64,

    // 検証状態をboolで管理し、再検証の必要性を即座に判断可能にする
    pub verified: bool,
}
