//! ProcessEntity - Multi-role process representation for AO
//!
//! Each AO process can have multiple roles (Owner/Holder/Requester) simultaneously.
//! Designed for AO's stateless execution environment with lightweight secret indexing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process entity - Complete state representation of an AO process
///
/// # Features
/// - Multi-role support (Owner/Holder/Requester can coexist)
/// - AO environment native properties
/// - Integrated performance metrics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessEntity {
    pub process_id: String,

    pub process_name: String,

    // 単一プロセスが複数の役割を担うことでネットワーク効率を向上 例: ["owner", "holder", "requester"] - 自身の秘密を管理しながら他者のkFragも保持
    pub active_roles: Vec<String>,

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
    pub supported_crypto_operations: Vec<String>,

    pub performance_metrics: PerformanceMetrics,

    pub created_at: u64,

    pub updated_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

/// Owner functionality data - Manages skO and secret splitting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecretIndex {
    pub secret_id: String,

    // 文字列ベースのステータス管理により、新しい状態の追加が既存データを破壊しない
    pub status: String,

    pub entity_references: EntityReferences,

    pub last_updated: u64,

    // 頻繁に参照される閾値情報をインデックスに含めることで、詳細エンティティのロードを回避し、レスポンス時間を短縮
    pub shamir_threshold: u8,

    pub shamir_total_shares: u8,
}

/// Entity references - Holds only IDs of related entities
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolderFragmentInfo {
    pub fragment_id: String,

    pub secret_id: String,

    pub access_control_condition: String,

    pub received_at: u64,

    // 使用回数を追跡することで、ホットなフラグメントの識別と最適配置を可能にする
    pub usage_count: u64,

    // 文字列ベースのステータス管理により、新しい状態の追加が既存データを破壊しない
    pub status: String,
}

/// Requester functionality data - Access requests and cFrag collection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub successful_operations: u64,

    pub failed_operations: u64,

    // レスポンス時間を追跡することで、パフォーマンス劣化の早期検出と最適化ポイントの特定を可能にする
    pub average_response_time_ms: u64,

    pub last_updated_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json;

    fn create_test_process_entity() -> ProcessEntity {
        ProcessEntity {
            process_id: "test_process_001".to_string(),
            process_name: "Test Process".to_string(),
            active_roles: vec!["owner".to_string()],
            owner_data: Some(OwnerData {
                owner_public_key: vec![1, 2, 3, 4],
                secret_indices: HashMap::new(),
                owner_config: HashMap::new(),
            }),
            holder_data: None,
            requester_data: None,
            configuration: HashMap::new(),
            supported_crypto_operations: vec!["shamir".to_string(), "pre".to_string()],
            performance_metrics: PerformanceMetrics {
                successful_operations: 0,
                failed_operations: 0,
                average_response_time_ms: 0,
                last_updated_at: 1642000000,
            },
            created_at: 1642000000,
            updated_at: 1642000000,
            version: 1,
        }
    }

    #[test]
    fn test_process_entity_serialization() {
        let process = create_test_process_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&process).unwrap();
        let deserialized: ProcessEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(process, deserialized);
    }

    #[test]
    fn test_process_entity_clone_and_equality() {
        let process = create_test_process_entity();
        let cloned = process.clone();
        
        assert_eq!(process, cloned);
    }

    #[test]
    fn test_multi_role_consistency() {
        let mut process = create_test_process_entity();
        
        // 複数ロールを持つプロセスのテスト
        process.active_roles = vec!["owner".to_string(), "holder".to_string()];
        process.holder_data = Some(HolderData {
            held_fragments: HashMap::new(),
            fragments_by_condition: HashMap::new(),
            reliability_score: 0.9,
            completed_reencryptions: 0,
            max_fragment_capacity: 100,
            current_load: 0,
        });
        
        // ownerロールがアクティブな場合、owner_dataが存在する必要がある
        assert!(process.owner_data.is_some());
        
        // holderロールがアクティブな場合、holder_dataが存在する必要がある
        assert!(process.holder_data.is_some());
    }

    #[test]
    fn test_optimistic_locking() {
        let mut process1 = create_test_process_entity();
        let mut process2 = process1.clone();
        
        // 楽観的ロックのテスト
        process1.version = 2;
        process2.version = 2;
        
        // 同じバージョンを持つプロセスは等価
        assert_eq!(process1.version, process2.version);
        
        // バージョンが異なる場合、等価でない
        process1.version = 3;
        assert_ne!(process1.version, process2.version);
    }

    #[test]
    fn test_performance_metrics_calculation() {
        let mut metrics = PerformanceMetrics {
            successful_operations: 10,
            failed_operations: 2,
            average_response_time_ms: 150,
            last_updated_at: 1642000000,
        };
        
        // 成功率の計算（手動計算）
        let total_operations = metrics.successful_operations + metrics.failed_operations;
        let success_rate = metrics.successful_operations as f64 / total_operations as f64;
        
        assert_eq!(total_operations, 12);
        assert!((success_rate - 0.8333333333333334).abs() < 0.0001);
    }

    #[test]
    fn test_secret_index_validation() {
        let secret_index = SecretIndex {
            secret_id: "secret_001".to_string(),
            status: "active".to_string(),
            entity_references: EntityReferences {
                share_ids: vec!["share_001".to_string()],
                capsule_ids: vec!["capsule_001".to_string()],
                active_requests: vec![],
                details_entity_id: "details_001".to_string(),
            },
            last_updated: 1642000000,
            shamir_threshold: 3,
            shamir_total_shares: 5,
        };
        
        // 閾値パラメータの検証
        assert!(secret_index.shamir_threshold <= secret_index.shamir_total_shares);
        assert!(secret_index.shamir_threshold > 0);
        assert!(secret_index.shamir_total_shares > 0);
    }

    #[test]
    fn test_holder_fragment_info_validation() {
        let fragment_info = HolderFragmentInfo {
            fragment_id: "fragment_001".to_string(),
            secret_id: "secret_001".to_string(),
            access_control_condition: "evm_verified".to_string(),
            received_at: 1642000000,
            usage_count: 0,
            status: "active".to_string(),
        };
        
        // フラグメント情報のバリデーション
        assert!(!fragment_info.fragment_id.is_empty());
        assert!(!fragment_info.secret_id.is_empty());
        assert!(!fragment_info.access_control_condition.is_empty());
        assert!(fragment_info.received_at > 0);
    }

    #[test]
    fn test_requester_data_validation() {
        let requester_data = RequesterData {
            active_requests: vec!["request_001".to_string()],
            active_reencryptions: vec!["reenc_001".to_string()],
            completed_requests: 50,
            success_rate: 0.85,
            average_processing_time_ms: 2000,
            requester_config: HashMap::new(),
        };
        
        // リクエスターデータのバリデーション
        assert!(requester_data.success_rate >= 0.0 && requester_data.success_rate <= 1.0);
        assert!(requester_data.average_processing_time_ms > 0);
        assert!(requester_data.completed_requests >= 0);
    }

    #[test]
    fn test_large_data_serialization() {
        let mut process = create_test_process_entity();
        
        // 大容量データのシリアライゼーション性能テスト
        let mut large_config = HashMap::new();
        for i in 0..1000 {
            large_config.insert(format!("key_{}", i), format!("value_{}", i));
        }
        process.configuration = large_config;
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&process).unwrap();
        let deserialized: ProcessEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(process.configuration.len(), 1000);
        assert_eq!(process, deserialized);
    }

    #[test]
    fn test_empty_fields_handling() {
        let process = ProcessEntity {
            process_id: "test_process_001".to_string(),
            process_name: "Test Process".to_string(),
            active_roles: vec![],
            owner_data: None,
            holder_data: None,
            requester_data: None,
            configuration: HashMap::new(),
            supported_crypto_operations: vec![],
            performance_metrics: PerformanceMetrics {
                successful_operations: 0,
                failed_operations: 0,
                average_response_time_ms: 0,
                last_updated_at: 1642000000,
            },
            created_at: 1642000000,
            updated_at: 1642000000,
            version: 1,
        };
        
        // 空のフィールドを持つプロセスのシリアライゼーション
        let json = serde_json::to_string(&process).unwrap();
        let deserialized: ProcessEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(process, deserialized);
        assert!(process.active_roles.is_empty());
        assert!(process.owner_data.is_none());
        assert!(process.holder_data.is_none());
        assert!(process.requester_data.is_none());
    }
}
