//! RekeyFragmentEntity - Re-encryption key fragment representation
//!
//! kFrag data generated during Phase 3 and stored by Holders.
//! Used for proxy re-encryption in Phase 4.

use serde::{Deserialize, Serialize};

/// Re-encryption key fragment entity - kFrag for threshold re-encryption
///
/// Generated in Phase 3, stored by Holders, used in Phase 4 re-encryption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RekeyFragmentEntity {
    pub kfrag_id: String,

    pub secret_id: String,

    pub holder_id: String,

    // kFragのバイナリデータ、Umbral-PREライブラリで生成
    pub kfrag_data: Vec<u8>,

    // アクセス制御条件と直接関連付けることで、
    // 異なる条件に対する個別の再暗号化鍵を生成可能にする
    pub access_control_condition: String,

    // 対応するShareEntityのIDを保持し、
    // 再暗号化時の正しいシェアとkFragのペアリングを保証
    pub corresponding_share_id: String,

    // Owner's公開鍵を保持することで、
    // kFragの正当性を後から検証可能にする
    pub owner_public_key: Vec<u8>,

    // Holder's公開鍵を保持することで、
    // 再暗号化時のHolder認証を効率化
    pub holder_public_key: Vec<u8>,

    pub created_at: u64,

    // 使用回数を追跡することで、
    // 頻繁に使用されるkFragの識別と最適配置を可能にする
    pub usage_count: u64,

    // 文字列ベースのステータス管理により、
    // 新しい状態の追加が既存データを破壊しない
    pub status: String,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rekey_fragment_entity() -> RekeyFragmentEntity {
        RekeyFragmentEntity {
            kfrag_id: "kfrag_001".to_string(),
            secret_id: "secret_001".to_string(),
            holder_id: "holder_001".to_string(),
            kfrag_data: vec![0x01, 0x02, 0x03, 0x04],
            access_control_condition: "evm_verified".to_string(),
            corresponding_share_id: "share_001".to_string(),
            owner_public_key: vec![0x05, 0x06, 0x07, 0x08],
            holder_public_key: vec![0x09, 0x0a, 0x0b, 0x0c],
            created_at: 1642000000,
            usage_count: 0,
            status: "active".to_string(),
            version: 1,
        }
    }

    #[test]
    fn test_rekey_fragment_entity_serialization() {
        let fragment = create_test_rekey_fragment_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&fragment).unwrap();
        let deserialized: RekeyFragmentEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(fragment, deserialized);
    }

    #[test]
    fn test_rekey_fragment_entity_clone_and_equality() {
        let fragment = create_test_rekey_fragment_entity();
        let cloned = fragment.clone();
        
        assert_eq!(fragment, cloned);
    }

    #[test]
    fn test_kfrag_share_correspondence() {
        let fragment = create_test_rekey_fragment_entity();
        
        // kFragとShareの対応関係の検証
        assert_eq!(fragment.corresponding_share_id, "share_001");
        assert_eq!(fragment.secret_id, "secret_001");
        
        // 対応関係が正しく設定されていることを確認
        assert!(!fragment.corresponding_share_id.is_empty());
        assert!(!fragment.secret_id.is_empty());
    }

    #[test]
    fn test_access_control_condition_validation() {
        let fragment = create_test_rekey_fragment_entity();
        
        // アクセス制御条件の検証
        assert!(!fragment.access_control_condition.is_empty());
        assert_eq!(fragment.access_control_condition, "evm_verified");
        
        // 一般的なアクセス制御条件のテスト
        let valid_conditions = vec!["evm_verified", "time_condition", "multi_sig", "threshold"];
        assert!(valid_conditions.contains(&fragment.access_control_condition.as_str()));
    }

    #[test]
    fn test_holder_validation() {
        let fragment = create_test_rekey_fragment_entity();
        
        // Holder情報の検証
        assert!(!fragment.holder_id.is_empty());
        assert!(!fragment.holder_public_key.is_empty());
        
        // Holder公開鍵の妥当性チェック
        assert!(fragment.holder_public_key.len() > 0);
    }

    #[test]
    fn test_kfrag_data_validation() {
        let fragment = create_test_rekey_fragment_entity();
        
        // kFragデータの検証
        assert!(!fragment.kfrag_data.is_empty());
        assert!(fragment.kfrag_data.len() > 0);
        
        // Owner公開鍵の妥当性チェック
        assert!(!fragment.owner_public_key.is_empty());
        assert!(fragment.owner_public_key.len() > 0);
    }

    #[test]
    fn test_usage_count_tracking() {
        let mut fragment = create_test_rekey_fragment_entity();
        
        // 使用回数追跡のテスト
        assert_eq!(fragment.usage_count, 0);
        
        // 使用回数の増加シミュレーション
        fragment.usage_count += 1;
        fragment.version += 1;
        
        assert_eq!(fragment.usage_count, 1);
        assert_eq!(fragment.version, 2);
    }

    #[test]
    fn test_status_management() {
        let mut fragment = create_test_rekey_fragment_entity();
        
        // ステータス管理のテスト
        let valid_statuses = vec!["active", "inactive", "expired", "revoked"];
        
        for status in valid_statuses {
            fragment.status = status.to_string();
            assert_eq!(fragment.status, status);
        }
    }

    #[test]
    fn test_lifecycle_simulation() {
        let mut fragment = create_test_rekey_fragment_entity();
        
        // kFragのライフサイクルシミュレーション
        
        // Phase 1: 生成直後
        assert_eq!(fragment.status, "active");
        assert_eq!(fragment.usage_count, 0);
        
        // Phase 2: 使用開始
        fragment.usage_count += 1;
        fragment.version += 1;
        
        assert_eq!(fragment.usage_count, 1);
        
        // Phase 3: 無効化
        fragment.status = "inactive".to_string();
        fragment.version += 1;
        
        assert_eq!(fragment.status, "inactive");
    }

    #[test]
    fn test_large_kfrag_data_handling() {
        let mut fragment = create_test_rekey_fragment_entity();
        
        // 大きなkFragデータの処理テスト
        let large_kfrag_data = vec![0u8; 1024 * 8]; // 8KB
        fragment.kfrag_data = large_kfrag_data;
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&fragment).unwrap();
        let deserialized: RekeyFragmentEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(fragment, deserialized);
        assert_eq!(fragment.kfrag_data.len(), 1024 * 8);
    }

    #[test]
    fn test_multiple_conditions_support() {
        let mut fragments = Vec::new();
        
        // 異なるアクセス制御条件を持つkFragの生成
        let conditions = vec!["evm_verified", "time_condition", "multi_sig"];
        
        for (i, condition) in conditions.iter().enumerate() {
            let mut fragment = create_test_rekey_fragment_entity();
            fragment.kfrag_id = format!("kfrag_{:03}", i + 1);
            fragment.access_control_condition = condition.to_string();
            fragments.push(fragment);
        }
        
        // 各kFragが異なる条件を持つことを確認
        for i in 0..fragments.len() {
            for j in i+1..fragments.len() {
                assert_ne!(fragments[i].access_control_condition, fragments[j].access_control_condition);
                assert_ne!(fragments[i].kfrag_id, fragments[j].kfrag_id);
                assert_eq!(fragments[i].secret_id, fragments[j].secret_id);
            }
        }
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut fragment = create_test_rekey_fragment_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = fragment.version;
        
        // 使用回数更新時のバージョンインクリメント
        fragment.usage_count += 1;
        fragment.version += 1;
        
        assert_eq!(fragment.version, original_version + 1);
    }

    #[test]
    fn test_data_consistency_validation() {
        let fragment = create_test_rekey_fragment_entity();
        
        // データの整合性チェック
        assert!(!fragment.kfrag_id.is_empty());
        assert!(!fragment.secret_id.is_empty());
        assert!(!fragment.holder_id.is_empty());
        assert!(!fragment.access_control_condition.is_empty());
        assert!(!fragment.corresponding_share_id.is_empty());
        assert!(!fragment.status.is_empty());
        assert!(fragment.created_at > 0);
        assert!(fragment.version > 0);
    }
}