//! ShareEntity - Shamir data share representation
//!
//! Data fragments split using Shamir Secret Sharing (Phase 1).
//! Contains encrypted fragments that can reconstruct the original secret.

use serde::{Deserialize, Serialize};

/// Data share entity - Shamir-split data fragment
///
/// Generated in PRD Phase 1, required for secret reconstruction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShareEntity {
    pub share_id: String,

    // 同一秘密から生成されたシェアをグループ化するため
    // 復元時に正しいシェアの組み合わせを特定する必要がある
    pub data_id: String,

    pub secret_id: String,

    // 1からnまでの連番で、Shamir多項式の評価点を表す
    // 各シェアが異なる点で評価されることで線形独立性を保証
    pub threshold_index: u8,

    // k-of-n閾値秘密分散のパラメータ
    // 最小k個のシェアがあれば秘密を復元可能
    pub shamir_threshold: u8,

    // 総シェア数n
    // 冗長性とアクセス制御のバランスを取るため設定
    pub shamir_total_shares: u8,

    // Shamir秘密分散で生成したシェアf(i)を鍵Kiで暗号化: Ci = AES_GCM(Ki, f(i))
    // 各シェアを個別に暗号化することで、単一のシェアが漏洩しても秘密が復元できない
    pub encrypted_fragment: Vec<u8>,

    pub fragment_size: usize,

    // シェアの所有者を特定し、アクセス権限を検証するため
    pub owner_public_key: Vec<u8>,

    // シェアの改竄を検出するためのSHA-256ハッシュ
    // Arweaveの不変性に加えて、アプリケーション層でも完全性を保証
    pub integrity_hash: Vec<u8>,

    pub created_at: u64,

    // アクセスパターンの分析とキャッシュ戦略の最適化に使用
    pub last_accessed_at: Option<u64>,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    pub version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn create_test_share_entity() -> ShareEntity {
        ShareEntity {
            share_id: "share_001".to_string(),
            data_id: "data_001".to_string(),
            secret_id: "secret_001".to_string(),
            threshold_index: 1,
            shamir_threshold: 3,
            shamir_total_shares: 5,
            encrypted_fragment: vec![0x01, 0x02, 0x03, 0x04],
            fragment_size: 4,
            owner_public_key: vec![0x05, 0x06, 0x07, 0x08],
            integrity_hash: vec![0x09, 0x0a, 0x0b, 0x0c],
            created_at: 1642000000,
            last_accessed_at: Some(1642000000),
            version: 1,
        }
    }

    #[test]
    fn test_share_entity_serialization() {
        let share = create_test_share_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&share).unwrap();
        let deserialized: ShareEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(share, deserialized);
    }

    #[test]
    fn test_share_entity_clone_and_equality() {
        let share = create_test_share_entity();
        let cloned = share.clone();
        
        assert_eq!(share, cloned);
    }

    #[test]
    fn test_threshold_parameter_validation() {
        let share = create_test_share_entity();
        
        // 閾値パラメータの検証
        assert!(share.shamir_threshold <= share.shamir_total_shares);
        assert!(share.shamir_threshold > 0);
        assert!(share.shamir_total_shares > 0);
        
        // threshold_indexは1からtotal_sharesの範囲内である必要がある
        assert!(share.threshold_index >= 1);
        assert!(share.threshold_index <= share.shamir_total_shares);
    }

    #[test]
    fn test_threshold_index_boundary_values() {
        let mut share = create_test_share_entity();
        
        // 境界値テスト: threshold_index = 1
        share.threshold_index = 1;
        assert_eq!(share.threshold_index, 1);
        
        // 境界値テスト: threshold_index = total_shares
        share.threshold_index = share.shamir_total_shares;
        assert_eq!(share.threshold_index, share.shamir_total_shares);
    }

    #[test]
    fn test_integrity_hash_validation() {
        let share = create_test_share_entity();
        
        // 整合性ハッシュの検証（SHA-256ハッシュは32バイト）
        // 実際のハッシュ値の検証は暗号化ライブラリに依存するため、ここでは長さのみチェック
        assert!(!share.integrity_hash.is_empty());
        
        // フラグメントが空でないことを確認
        assert!(!share.encrypted_fragment.is_empty());
        
        // フラグメントサイズが実際のサイズと一致することを確認
        assert_eq!(share.fragment_size, share.encrypted_fragment.len());
    }

    #[test]
    fn test_data_consistency_validation() {
        let share = create_test_share_entity();
        
        // データの整合性チェック
        assert!(!share.share_id.is_empty());
        assert!(!share.data_id.is_empty());
        assert!(!share.secret_id.is_empty());
        assert!(!share.owner_public_key.is_empty());
        assert!(share.created_at > 0);
        assert!(share.version > 0);
    }

    #[test]
    fn test_same_data_id_shares_consistency() {
        let share1 = create_test_share_entity();
        let mut share2 = create_test_share_entity();
        
        // 同じdata_idを持つシェアは同じ閾値パラメータを持つ必要がある
        share2.share_id = "share_002".to_string();
        share2.threshold_index = 2;
        
        assert_eq!(share1.data_id, share2.data_id);
        assert_eq!(share1.shamir_threshold, share2.shamir_threshold);
        assert_eq!(share1.shamir_total_shares, share2.shamir_total_shares);
        assert_ne!(share1.threshold_index, share2.threshold_index);
    }

    #[test]
    fn test_large_fragment_handling() {
        let mut share = create_test_share_entity();
        
        // 大きなフラグメントの処理テスト
        let large_fragment = vec![0u8; 1024 * 1024]; // 1MB
        share.encrypted_fragment = large_fragment;
        share.fragment_size = share.encrypted_fragment.len();
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&share).unwrap();
        let deserialized: ShareEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(share, deserialized);
        assert_eq!(share.fragment_size, 1024 * 1024);
    }

    #[test]
    fn test_optional_last_accessed_at() {
        let mut share = create_test_share_entity();
        
        // last_accessed_atがNoneの場合のテスト
        share.last_accessed_at = None;
        
        let json = serde_json::to_string(&share).unwrap();
        let deserialized: ShareEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(share, deserialized);
        assert!(share.last_accessed_at.is_none());
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut share = create_test_share_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = share.version;
        share.version += 1;
        share.last_accessed_at = Some(1642000100);
        
        assert_eq!(share.version, original_version + 1);
        assert!(share.last_accessed_at.unwrap() > share.created_at);
    }

    #[test]
    fn test_multiple_shares_different_indices() {
        let mut shares = Vec::new();
        
        // 同じ秘密に対する複数のシェアを作成
        for i in 1..=5 {
            let mut share = create_test_share_entity();
            share.share_id = format!("share_{:03}", i);
            share.threshold_index = i;
            shares.push(share);
        }
        
        // 各シェアが異なるインデックスを持つことを確認
        for i in 0..shares.len() {
            for j in i+1..shares.len() {
                assert_ne!(shares[i].threshold_index, shares[j].threshold_index);
                assert_ne!(shares[i].share_id, shares[j].share_id);
                assert_eq!(shares[i].data_id, shares[j].data_id);
            }
        }
    }
}
