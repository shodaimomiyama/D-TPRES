//! SecretDetailsEntity - Detailed secret management information
//!
//! Holds detailed secret management information separated from ProcessEntity.
//! Supports ProcessEntity lightweight design for AO's stateless environment.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Secret management details entity - Detailed information about a secret
///
/// Separated from ProcessEntity, loaded only when needed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecretDetailsEntity {
    pub details_id: String,

    pub secret_id: String,

    // アクセス制御条件をリスト形式で管理することで、
    // 複数の条件のAND/OR組み合わせを柔軟に表現
    pub access_control_conditions: Vec<String>,

    // 条件別にkFragをグループ化することで、
    // 異なるアクセスパターンに対して異なる再暗号化鍵を生成可能にする
    pub generated_kfrags_by_condition: HashMap<String, Vec<String>>,

    // アクセス履歴を保持することで、
    // セキュリティ監査と不正アクセスの検出を可能にする
    pub access_history: Vec<AccessRecord>,

    // 柔軟なメタデータ管理により、
    // アプリケーション固有の情報をスキーマ変更なしに格納
    pub metadata: HashMap<String, String>,

    pub description: Option<String>,

    // 有効期限をOptionalにすることで、
    // 永続的な秘密と時限的な秘密の両方をサポート
    pub expires_at: Option<u64>,

    pub created_at: u64,

    pub updated_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// Access record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AccessRecord {
    pub request_id: String,

    pub accessor_process_id: String,

    pub accessed_at: u64,

    // 文字列ベースの結果管理により、
    // 新しい結果タイプの追加が既存データを破壊しない
    pub result: String,

    // 実際に使用された条件を記録することで、
    // 複数条件下でのアクセスパターンを分析可能にする
    pub condition_used: String,
}
