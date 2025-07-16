//! SecretDetailsEntity - Detailed secret management information
//!
//! Holds detailed secret management information separated from ProcessEntity.
//! Supports ProcessEntity lightweight design for AO's stateless environment.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Secret management details entity - Detailed information about a secret
///
/// Separated from ProcessEntity, loaded only when needed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use serde_json;

    fn create_test_secret_details_entity() -> SecretDetailsEntity {
        let mut generated_kfrags = HashMap::new();
        generated_kfrags.insert("evm_verified".to_string(), vec!["kfrag_001".to_string(), "kfrag_002".to_string()]);
        generated_kfrags.insert("time_condition".to_string(), vec!["kfrag_003".to_string()]);
        
        let mut metadata = HashMap::new();
        metadata.insert("encryption_type".to_string(), "AES-256-GCM".to_string());
        metadata.insert("category".to_string(), "personal_data".to_string());
        
        SecretDetailsEntity {
            details_id: "details_001".to_string(),
            secret_id: "secret_001".to_string(),
            access_control_conditions: vec!["evm_verified".to_string(), "time_condition".to_string()],
            generated_kfrags_by_condition: generated_kfrags,
            access_history: vec![create_test_access_record()],
            metadata,
            description: Some("Test secret for demonstration".to_string()),
            expires_at: Some(1642000000 + 86400), // 24時間後
            created_at: 1642000000,
            updated_at: 1642000000,
            version: 1,
        }
    }

    fn create_test_access_record() -> AccessRecord {
        AccessRecord {
            request_id: "request_001".to_string(),
            accessor_process_id: "accessor_001".to_string(),
            accessed_at: 1642000100,
            result: "success".to_string(),
            condition_used: "evm_verified".to_string(),
        }
    }

    #[test]
    fn test_secret_details_entity_serialization() {
        let details = create_test_secret_details_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&details).unwrap();
        let deserialized: SecretDetailsEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(details, deserialized);
    }

    #[test]
    fn test_secret_details_entity_clone_and_equality() {
        let details = create_test_secret_details_entity();
        let cloned = details.clone();
        
        assert_eq!(details, cloned);
    }

    #[test]
    fn test_access_control_conditions_validation() {
        let details = create_test_secret_details_entity();
        
        // アクセス制御条件の検証
        assert!(!details.access_control_conditions.is_empty());
        assert!(details.access_control_conditions.contains(&"evm_verified".to_string()));
        assert!(details.access_control_conditions.contains(&"time_condition".to_string()));
        
        // 条件が空でないことを確認
        for condition in &details.access_control_conditions {
            assert!(!condition.is_empty());
        }
    }

    #[test]
    fn test_kfrag_condition_mapping() {
        let details = create_test_secret_details_entity();
        
        // kFragと条件のマッピング検証
        assert!(details.generated_kfrags_by_condition.contains_key("evm_verified"));
        assert!(details.generated_kfrags_by_condition.contains_key("time_condition"));
        
        // 条件ごとのkFrag数の検証
        let evm_kfrags = details.generated_kfrags_by_condition.get("evm_verified").unwrap();
        assert_eq!(evm_kfrags.len(), 2);
        
        let time_kfrags = details.generated_kfrags_by_condition.get("time_condition").unwrap();
        assert_eq!(time_kfrags.len(), 1);
    }

    #[test]
    fn test_access_history_integrity() {
        let details = create_test_secret_details_entity();
        
        // アクセス履歴の整合性チェック
        assert!(!details.access_history.is_empty());
        
        for record in &details.access_history {
            assert!(!record.request_id.is_empty());
            assert!(!record.accessor_process_id.is_empty());
            assert!(!record.result.is_empty());
            assert!(!record.condition_used.is_empty());
            assert!(record.accessed_at > 0);
        }
    }

    #[test]
    fn test_access_history_chronological_order() {
        let mut details = create_test_secret_details_entity();
        
        // 複数のアクセスレコードを追加
        let record2 = AccessRecord {
            request_id: "request_002".to_string(),
            accessor_process_id: "accessor_002".to_string(),
            accessed_at: 1642000200,
            result: "success".to_string(),
            condition_used: "time_condition".to_string(),
        };
        
        let record3 = AccessRecord {
            request_id: "request_003".to_string(),
            accessor_process_id: "accessor_003".to_string(),
            accessed_at: 1642000300,
            result: "failure".to_string(),
            condition_used: "evm_verified".to_string(),
        };
        
        details.access_history.push(record2);
        details.access_history.push(record3);
        
        // 時系列順序の検証
        for i in 1..details.access_history.len() {
            assert!(details.access_history[i].accessed_at > details.access_history[i-1].accessed_at);
        }
    }

    #[test]
    fn test_expiration_validation() {
        let details = create_test_secret_details_entity();
        
        // 有効期限の検証
        assert!(details.expires_at.is_some());
        assert!(details.expires_at.unwrap() > details.created_at);
        
        // 有効期限が設定されている場合のチェック
        let current_time = 1642000000 + 86400 + 1; // expires_atよりも後
        let is_expired = details.expires_at.map_or(false, |expire_time| current_time > expire_time);
        assert!(is_expired);
    }

    #[test]
    fn test_optional_expiration_handling() {
        let mut details = create_test_secret_details_entity();
        
        // 有効期限なしの秘密のテスト
        details.expires_at = None;
        
        let json = serde_json::to_string(&details).unwrap();
        let deserialized: SecretDetailsEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(details, deserialized);
        assert!(details.expires_at.is_none());
    }

    #[test]
    fn test_optional_description_handling() {
        let mut details = create_test_secret_details_entity();
        
        // 説明なしの秘密のテスト
        details.description = None;
        
        let json = serde_json::to_string(&details).unwrap();
        let deserialized: SecretDetailsEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(details, deserialized);
        assert!(details.description.is_none());
    }

    #[test]
    fn test_metadata_flexibility() {
        let mut details = create_test_secret_details_entity();
        
        // メタデータの柔軟な管理テスト
        details.metadata.insert("priority".to_string(), "high".to_string());
        details.metadata.insert("owner_department".to_string(), "security".to_string());
        details.metadata.insert("retention_period".to_string(), "7_years".to_string());
        
        assert_eq!(details.metadata.get("priority"), Some(&"high".to_string()));
        assert_eq!(details.metadata.get("owner_department"), Some(&"security".to_string()));
        assert_eq!(details.metadata.get("retention_period"), Some(&"7_years".to_string()));
        
        // メタデータのシリアライゼーション
        let json = serde_json::to_string(&details).unwrap();
        let deserialized: SecretDetailsEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(details, deserialized);
    }

    #[test]
    fn test_condition_kfrag_consistency() {
        let details = create_test_secret_details_entity();
        
        // 条件とkFragの整合性チェック
        for condition in &details.access_control_conditions {
            assert!(details.generated_kfrags_by_condition.contains_key(condition));
        }
        
        // アクセス履歴で使用された条件が有効なことを確認
        for record in &details.access_history {
            assert!(details.access_control_conditions.contains(&record.condition_used));
        }
    }

    #[test]
    fn test_access_pattern_analysis() {
        let mut details = create_test_secret_details_entity();
        
        // アクセスパターン分析のためのテストデータ追加
        let evm_access = AccessRecord {
            request_id: "request_002".to_string(),
            accessor_process_id: "accessor_002".to_string(),
            accessed_at: 1642000200,
            result: "success".to_string(),
            condition_used: "evm_verified".to_string(),
        };
        
        let time_access = AccessRecord {
            request_id: "request_003".to_string(),
            accessor_process_id: "accessor_003".to_string(),
            accessed_at: 1642000300,
            result: "success".to_string(),
            condition_used: "time_condition".to_string(),
        };
        
        details.access_history.push(evm_access);
        details.access_history.push(time_access);
        
        // 条件別アクセス回数の集計
        let mut condition_counts = HashMap::new();
        for record in &details.access_history {
            *condition_counts.entry(record.condition_used.clone()).or_insert(0) += 1;
        }
        
        assert_eq!(condition_counts.get("evm_verified"), Some(&2));
        assert_eq!(condition_counts.get("time_condition"), Some(&1));
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut details = create_test_secret_details_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = details.version;
        
        // アクセス履歴追加時のバージョン更新
        let new_record = create_test_access_record();
        details.access_history.push(new_record);
        details.updated_at = 1642000200;
        details.version += 1;
        
        assert_eq!(details.version, original_version + 1);
        assert!(details.updated_at > details.created_at);
    }

    #[test]
    fn test_large_access_history_handling() {
        let mut details = create_test_secret_details_entity();
        
        // 大量のアクセス履歴の処理テスト
        for i in 1..=100 {
            let record = AccessRecord {
                request_id: format!("request_{:03}", i),
                accessor_process_id: format!("accessor_{:03}", i),
                accessed_at: 1642000000 + i,
                result: if i % 10 == 0 { "failure" } else { "success" }.to_string(),
                condition_used: "evm_verified".to_string(),
            };
            details.access_history.push(record);
        }
        
        assert_eq!(details.access_history.len(), 101); // 初期レコード + 100
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&details).unwrap();
        let deserialized: SecretDetailsEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(details, deserialized);
    }

    #[test]
    fn test_data_consistency_validation() {
        let details = create_test_secret_details_entity();
        
        // データの整合性チェック
        assert!(!details.details_id.is_empty());
        assert!(!details.secret_id.is_empty());
        assert!(!details.access_control_conditions.is_empty());
        assert!(details.created_at > 0);
        assert!(details.updated_at > 0);
        assert!(details.version > 0);
        assert!(details.updated_at >= details.created_at);
    }
}
