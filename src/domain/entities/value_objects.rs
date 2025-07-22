//! Union型の値オブジェクト定義
//!
//! D-TPRESシステムで使用される列挙型を定義。
//! 全てのUnion型は純粋なデータ構造として実装。

use serde::{Deserialize, Serialize};

/// プロセスが持つことができるロール
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessRole {
    Owner,
    Holder,
    Requester,
}

/// 秘密の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretStatus {
    Active,
    Archived,
    Expired,
}

/// アクセス要求の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessRequestStatus {
    Pending,
    EvmVerified,
    Approved,
    Rejected,
    Completed,
}

/// 再暗号化キーフラグメントの状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RekeyFragmentStatus {
    Created,
    Distributed,
    Active,
    Consumed,
    Expired,
}

/// 再暗号化処理の状態
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReencryptionStatus {
    Initiated,
    Collecting,
    ThresholdMet,
    Completed,
    Failed,
}

/// サポートされる暗号操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoOperation {
    ShamirSplit,
    PreEncrypt,
    ReEncrypt,
    VerifyProof,
}

/// アクセス結果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessResult {
    Granted,
    Denied,
    Expired,
}

/// 暗号化ワークフローのフェーズ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoPhase {
    /// Phase 0: プロセス生成・鍵準備
    Initialize,
    /// Phase 1: 秘密分割・公開ストレージ
    SecretSharing,
    /// Phase 2: アクセス要求・EVM検証
    AccessRequest,
    /// Phase 3: 再暗号化鍵のkFrag分割
    KeyFragmentation,
    /// Phase 4: k-of-n プロキシ再暗号化
    ProxyReencryption,
    /// Phase 5: クライアント復号・秘密復元
    SecretRecovery,
}
