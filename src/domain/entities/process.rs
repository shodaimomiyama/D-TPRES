//! ProcessEntity - Multi-role process representation for AO
//!
//! Each AO process can have multiple roles (Owner/Holder/Requester) simultaneously.
//! Designed for AO's stateless execution environment with lightweight secret indexing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::value_objects::{CryptoOperation, ProcessRole, RekeyFragmentStatus, SecretStatus};

/// Process entity - Complete state representation of an AO process
///
/// # Features
/// - Multi-role support (Owner/Holder/Requester can coexist)
/// - AO environment native properties
/// - Integrated performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProcessEntity {
    pub process_id: String,

    pub process_name: String,

    // 単一プロセスが複数の役割を担うことでネットワーク効率を向上 例: [Owner, Holder, Requester] - 自身の秘密を管理しながら他者のkFragも保持
    pub active_roles: Vec<ProcessRole>,

    // Phase 0, 1, 3で使用されるOwner機能
    // skO（秘密鍵）の管理と秘密分割を担当
    pub owner_data: Option<OwnerData>,

    // Phase 3, 4で使用されるHolder機能
    // kFragの保管と再暗号化の実行を担当
    pub holder_data: Option<HolderData>,

    // Phase 2, 4, 5で使用されるRequester機能
    // アクセス要求の発行とcFrag収集を担当
    pub requester_data: Option<RequesterData>,

    // プロセス固有の設定をKey-Value形式で柔軟に管理
    // 例: {"max_concurrent_requests": "10", "timeout_seconds": "300"}
    pub configuration: HashMap<String, String>,

    // プロセスがサポートする暗号操作を明示
    // 能力ベースのルーティングとロードバランシングに使用
    pub supported_crypto_operations: Vec<CryptoOperation>,

    pub performance_metrics: PerformanceMetrics,

    pub created_at: u64,

    pub updated_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// Owner functionality data - Manages skO and secret splitting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OwnerData {
    // skO（秘密鍵）に対応する公開鍵
    // 秘密鍵自体は保存せず、必要時にセキュアストレージから取得
    pub owner_public_key: Vec<u8>,

    // 軽量な秘密インデックス情報のみを保持、
    pub secret_indices: HashMap<String, SecretIndex>,

    // Owner固有の設定
    // デフォルト値を設定することで、秘密ごとの設定の繰り返しを避ける
    pub owner_config: HashMap<String, String>,
}

/// Secret index - Lightweight secret management information
/// Minimal information held in ProcessEntity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SecretIndex {
    pub secret_id: String,

    // 型安全なステータス管理により、不正な状態遷移を防止
    pub status: SecretStatus,

    pub entity_references: EntityReferences,

    pub last_updated: u64,

    // 頻繁に参照される閾値情報をインデックスに含めることで、詳細エンティティのロードを回避し、レスポンス時間を短縮
    pub shamir_threshold: u8,

    pub shamir_total_shares: u8,
}

/// Entity references - Holds only IDs of related entities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EntityReferences {
    pub share_ids: Vec<String>,

    pub capsule_ids: Vec<String>,

    // アクティブなリクエストのみを保持することで、履歴データによるメモリ圧迫を防ぎ、検索効率を向上
    pub active_requests: Vec<String>,

    // 詳細情報への参照により遅延ロードを可能にし、頻繁なリスト操作時のメモリ使用量を最小化
    pub details_entity_id: String,
}

/// Holder functionality data - kFrag storage and re-encryption execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HolderData {
    // Fragment情報を直接保持することで、再暗号化時の外部エンティティ参照を減らし、処理速度を向上
    pub held_fragments: HashMap<String, HolderFragmentInfo>,

    // 条件別インデックスにより、特定条件に対するフラグメント検索をO(1)で実現
    pub fragments_by_condition: HashMap<String, Vec<String>>,

    // 信頼性スコアによりロードバランシング時のHolder選択を最適化し、システム全体の可用性を向上
    pub reliability_score: f64,

    pub completed_reencryptions: u64,

    pub max_fragment_capacity: u64,

    // 負荷状態を数値化することで、動的なフラグメント配置の判断基準を提供
    pub current_load: u64,
}

/// Holder fragment information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HolderFragmentInfo {
    pub fragment_id: String,

    pub secret_id: String,

    pub access_control_condition: String,

    pub received_at: u64,

    // 使用回数を追跡することで、ホットなフラグメントの識別と最適配置を可能にする
    pub usage_count: u64,

    // 型安全なステータス管理により、不正な状態遷移を防止
    pub status: RekeyFragmentStatus,
}

/// Requester functionality data - Access requests and cFrag collection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RequesterData {
    // アクティブなリクエストのみを保持することで、履歴データによるメモリ圧迫を防ぎ、検索効率を向上
    pub active_requests: Vec<String>,

    pub active_reencryptions: Vec<String>,

    pub completed_requests: u64,

    // 成功率を追跡することで、信頼性の低いRequesterの早期検出と対策を可能にする
    pub success_rate: f64,

    // 平均処理時間により、タイムアウト値の動的調整とSLA管理を実現
    pub average_processing_time_ms: u64,

    // 設定を外部化することで、コード変更なしに動作パラメータを調整可能にする
    pub requester_config: HashMap<String, String>,
}

/// Performance metrics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PerformanceMetrics {
    pub successful_operations: u64,

    pub failed_operations: u64,

    // レスポンス時間を追跡することで、パフォーマンス劣化の早期検出と最適化ポイントの特定を可能にする
    pub average_response_time_ms: u64,

    pub last_updated_at: u64,
}
