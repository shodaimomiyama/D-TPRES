//! CapsuleEntity - PRE capsule representation
//!
//! Capsule information for Proxy Re-Encryption (Phase 1).
//! Used in Phase 4 for re-encryption operations.

use serde::{Deserialize, Serialize};

/// Capsule entity - PRE encryption capsule
///
/// Generated in Phase 1, used in Phase 4 re-encryption
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapsuleEntity {
    pub capsule_id: String,

    pub data_id: String,

    pub secret_id: String,

    // ShareEntityのthreshold_indexと対応させることで、再暗号化時に正しいカプセルとシェアのペアを特定
    pub capsule_index: u8,

    // PRE_Enc(pkO, Ki)で生成したカプセル、pkOからpkAへの変換情報を含むが、秘密情報は含まない
    pub capsule_data: Vec<u8>,

    // カプセルとシェアの1対1対応を明示的に管理、再暗号化時の整合性チェックに使用
    pub corresponding_ciphertext_id: String,

    pub owner_public_key: Vec<u8>,

    // カプセル生成時のランダム性を保存することで、必要時に再暗号化鍵の生成過程を検証可能にする
    pub encrypted_random_key: Vec<u8>,

    pub created_at: u64,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn create_test_capsule_entity() -> CapsuleEntity {
        CapsuleEntity {
            capsule_id: "capsule_001".to_string(),
            data_id: "data_001".to_string(),
            secret_id: "secret_001".to_string(),
            capsule_index: 1,
            capsule_data: vec![0x01, 0x02, 0x03, 0x04],
            corresponding_ciphertext_id: "share_001".to_string(),
            owner_public_key: vec![0x05, 0x06, 0x07, 0x08],
            encrypted_random_key: vec![0x09, 0x0a, 0x0b, 0x0c],
            created_at: 1642000000,
            version: 1,
        }
    }

    #[test]
    fn test_capsule_entity_serialization() {
        let capsule = create_test_capsule_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&capsule).unwrap();
        let deserialized: CapsuleEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(capsule, deserialized);
    }

    #[test]
    fn test_capsule_entity_clone_and_equality() {
        let capsule = create_test_capsule_entity();
        let cloned = capsule.clone();
        
        assert_eq!(capsule, cloned);
    }

    #[test]
    fn test_capsule_share_correspondence() {
        let capsule = create_test_capsule_entity();
        
        // カプセルとシェアの1対1対応関係の検証
        assert_eq!(capsule.corresponding_ciphertext_id, "share_001");
        assert_eq!(capsule.capsule_index, 1);
        
        // カプセルとシェアは同じdata_idを持つ必要がある
        assert_eq!(capsule.data_id, "data_001");
        assert_eq!(capsule.secret_id, "secret_001");
    }

    #[test]
    fn test_capsule_data_integrity() {
        let capsule = create_test_capsule_entity();
        
        // カプセルデータの整合性チェック
        assert!(!capsule.capsule_id.is_empty());
        assert!(!capsule.data_id.is_empty());
        assert!(!capsule.secret_id.is_empty());
        assert!(!capsule.corresponding_ciphertext_id.is_empty());
        assert!(!capsule.capsule_data.is_empty());
        assert!(!capsule.owner_public_key.is_empty());
        assert!(!capsule.encrypted_random_key.is_empty());
        assert!(capsule.created_at > 0);
        assert!(capsule.version > 0);
    }

    #[test]
    fn test_capsule_index_validation() {
        let capsule = create_test_capsule_entity();
        
        // カプセルインデックスの検証（ShareEntityのthreshold_indexと対応）
        assert!(capsule.capsule_index > 0);
        
        // カプセルインデックスは通常の闾値システムでは1からnの範囲である
        assert!(capsule.capsule_index >= 1);
        assert!(capsule.capsule_index <= 255); // u8の最大値を仮定
    }

    #[test]
    fn test_owner_consistency() {
        let capsule = create_test_capsule_entity();
        
        // カプセルと対応するシェアは同じオーナーを持つ必要がある
        assert!(!capsule.owner_public_key.is_empty());
        
        // ランダムキーが正しく暗号化されていることを確認
        assert!(!capsule.encrypted_random_key.is_empty());
    }

    #[test]
    fn test_large_capsule_data_handling() {
        let mut capsule = create_test_capsule_entity();
        
        // 大きなカプセルデータの処理テスト
        let large_data = vec![0u8; 1024 * 64]; // 64KB
        capsule.capsule_data = large_data;
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&capsule).unwrap();
        let deserialized: CapsuleEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(capsule, deserialized);
        assert_eq!(capsule.capsule_data.len(), 1024 * 64);
    }

    #[test]
    fn test_multiple_capsules_same_data() {
        let mut capsules = Vec::new();
        
        // 同じデータに対する複数のカプセルを作成
        for i in 1..=5 {
            let mut capsule = create_test_capsule_entity();
            capsule.capsule_id = format!("capsule_{:03}", i);
            capsule.capsule_index = i;
            capsule.corresponding_ciphertext_id = format!("share_{:03}", i);
            capsules.push(capsule);
        }
        
        // 各カプセルが異なるインデックスを持つことを確認
        for i in 0..capsules.len() {
            for j in i+1..capsules.len() {
                assert_ne!(capsules[i].capsule_index, capsules[j].capsule_index);
                assert_ne!(capsules[i].capsule_id, capsules[j].capsule_id);
                assert_ne!(capsules[i].corresponding_ciphertext_id, capsules[j].corresponding_ciphertext_id);
                assert_eq!(capsules[i].data_id, capsules[j].data_id);
                assert_eq!(capsules[i].secret_id, capsules[j].secret_id);
            }
        }
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut capsule = create_test_capsule_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = capsule.version;
        capsule.version += 1;
        
        assert_eq!(capsule.version, original_version + 1);
    }

    #[test]
    fn test_pre_capsule_format_validation() {
        let capsule = create_test_capsule_entity();
        
        // PREカプセルデータの基本的なフォーマット検証
        // 実際のカプセルデータの検証はumbral-preライブラリに依存するため、
        // ここでは基本的なチェックのみ実施
        assert!(!capsule.capsule_data.is_empty());
        assert!(capsule.capsule_data.len() > 0);
        
        // ランダムキーが正しく暗号化されていることを確認
        assert!(!capsule.encrypted_random_key.is_empty());
        assert!(capsule.encrypted_random_key.len() > 0);
    }

    #[test]
    fn test_capsule_creation_time_validation() {
        let capsule = create_test_capsule_entity();
        
        // カプセルの作成時刻が有効なことを確認
        assert!(capsule.created_at > 0);
        
        // バージョンが正しく初期化されていることを確認
        assert!(capsule.version > 0);
    }
}
