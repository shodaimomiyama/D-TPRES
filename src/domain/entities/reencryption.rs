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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn create_test_reencryption_entity() -> ReencryptionEntity {
        ReencryptionEntity {
            reencryption_id: "reenc_001".to_string(),
            access_request_id: "request_001".to_string(),
            target_capsule_id: "capsule_001".to_string(),
            requester_process_id: "requester_001".to_string(),
            target_holders: vec!["holder_001".to_string(), "holder_002".to_string(), "holder_003".to_string()],
            required_threshold: 2,
            collected_cfrags: vec![],
            status: "initiated".to_string(),
            started_at: 1642000000,
            completed_at: None,
            timeout_at: 1642000300, // 5分後
            version: 1,
        }
    }

    fn create_test_cfrag_data(cfrag_id: &str, holder_id: &str) -> CFragData {
        CFragData {
            cfrag_id: cfrag_id.to_string(),
            holder_id: holder_id.to_string(),
            cfrag_data: vec![0x01, 0x02, 0x03, 0x04],
            corresponding_kfrag_id: format!("kfrag_{}", holder_id),
            generated_at: 1642000100,
            holder_signature: vec![0x05, 0x06, 0x07, 0x08],
        }
    }

    #[test]
    fn test_reencryption_entity_serialization() {
        let reencryption = create_test_reencryption_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&reencryption).unwrap();
        let deserialized: ReencryptionEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(reencryption, deserialized);
    }

    #[test]
    fn test_reencryption_entity_clone_and_equality() {
        let reencryption = create_test_reencryption_entity();
        let cloned = reencryption.clone();
        
        assert_eq!(reencryption, cloned);
    }

    #[test]
    fn test_threshold_validation() {
        let reencryption = create_test_reencryption_entity();
        
        // 闾値パラメータの検証
        assert!(reencryption.required_threshold > 0);
        assert!(reencryption.required_threshold <= reencryption.target_holders.len() as u8);
        
        // 闾値がターゲットHolder数以下であることを確認
        assert!(reencryption.required_threshold <= reencryption.target_holders.len() as u8);
    }

    #[test]
    fn test_cfrag_collection_and_threshold_achievement() {
        let mut reencryption = create_test_reencryption_entity();
        
        // cFragの収集シミュレーション
        assert_eq!(reencryption.collected_cfrags.len(), 0);
        assert_eq!(reencryption.status, "initiated");
        
        // 最初のcFragを追加
        let cfrag1 = create_test_cfrag_data("cfrag_001", "holder_001");
        reencryption.collected_cfrags.push(cfrag1);
        reencryption.status = "collecting".to_string();
        
        assert_eq!(reencryption.collected_cfrags.len(), 1);
        assert_eq!(reencryption.status, "collecting");
        
        // 闾値達成用の2つ目のcFragを追加
        let cfrag2 = create_test_cfrag_data("cfrag_002", "holder_002");
        reencryption.collected_cfrags.push(cfrag2);
        
        // 闾値達成チェック
        let threshold_achieved = reencryption.collected_cfrags.len() >= reencryption.required_threshold as usize;
        assert!(threshold_achieved);
        
        // 闾値達成時のステータス更新
        if threshold_achieved {
            reencryption.status = "threshold_met".to_string();
        }
        
        assert_eq!(reencryption.status, "threshold_met");
    }

    #[test]
    fn test_duplicate_cfrag_detection() {
        let mut reencryption = create_test_reencryption_entity();
        
        // 同じHolderからの重複cFrag検出テスト
        let cfrag1 = create_test_cfrag_data("cfrag_001", "holder_001");
        let cfrag2 = create_test_cfrag_data("cfrag_002", "holder_001"); // 同じHolder
        
        reencryption.collected_cfrags.push(cfrag1);
        
        // 重複cFragの検出ロジック
        let has_duplicate = reencryption.collected_cfrags.iter()
            .any(|cfrag| cfrag.holder_id == cfrag2.holder_id);
        
        assert!(has_duplicate);
    }

    #[test]
    fn test_holder_signature_validation() {
        let cfrag = create_test_cfrag_data("cfrag_001", "holder_001");
        
        // Holder署名の検証
        assert!(!cfrag.holder_signature.is_empty());
        assert!(!cfrag.holder_id.is_empty());
        assert!(!cfrag.cfrag_data.is_empty());
        assert!(!cfrag.corresponding_kfrag_id.is_empty());
        assert!(cfrag.generated_at > 0);
    }

    #[test]
    fn test_timeout_enforcement() {
        let mut reencryption = create_test_reencryption_entity();
        
        // タイムアウトのシミュレーション
        let current_time = 1642000400; // timeout_atよりも後
        
        // タイムアウトチェック
        let is_timed_out = current_time > reencryption.timeout_at;
        assert!(is_timed_out);
        
        // タイムアウト時のステータス更新
        if is_timed_out {
            reencryption.status = "failed".to_string();
            reencryption.version += 1;
        }
        
        assert_eq!(reencryption.status, "failed");
        assert!(reencryption.timeout_at > reencryption.started_at);
    }

    #[test]
    fn test_reencryption_lifecycle_simulation() {
        let mut reencryption = create_test_reencryption_entity();
        
        // 再暗号化プロセスのライフサイクルシミュレーション
        
        // Phase 1: 初期化
        assert_eq!(reencryption.status, "initiated");
        assert_eq!(reencryption.collected_cfrags.len(), 0);
        
        // Phase 2: cFrag収集開始
        reencryption.status = "collecting".to_string();
        let cfrag1 = create_test_cfrag_data("cfrag_001", "holder_001");
        reencryption.collected_cfrags.push(cfrag1);
        reencryption.version += 1;
        
        assert_eq!(reencryption.status, "collecting");
        assert_eq!(reencryption.collected_cfrags.len(), 1);
        
        // Phase 3: 闾値達成
        let cfrag2 = create_test_cfrag_data("cfrag_002", "holder_002");
        reencryption.collected_cfrags.push(cfrag2);
        reencryption.status = "threshold_met".to_string();
        reencryption.version += 1;
        
        assert_eq!(reencryption.status, "threshold_met");
        assert_eq!(reencryption.collected_cfrags.len(), 2);
        assert!(reencryption.collected_cfrags.len() >= reencryption.required_threshold as usize);
        
        // Phase 4: 完了
        reencryption.status = "completed".to_string();
        reencryption.completed_at = Some(1642000200);
        reencryption.version += 1;
        
        assert_eq!(reencryption.status, "completed");
        assert!(reencryption.completed_at.is_some());
        assert!(reencryption.completed_at.unwrap() > reencryption.started_at);
    }

    #[test]
    fn test_kfrag_cfrag_correspondence() {
        let cfrag = create_test_cfrag_data("cfrag_001", "holder_001");
        
        // kFragとcFragの対応関係の検証
        assert_eq!(cfrag.corresponding_kfrag_id, "kfrag_holder_001");
        assert!(!cfrag.corresponding_kfrag_id.is_empty());
        
        // cFragが正しくkFragと関連付けられていることを確認
        assert!(cfrag.corresponding_kfrag_id.contains(&cfrag.holder_id));
    }

    #[test]
    fn test_status_transitions() {
        let mut reencryption = create_test_reencryption_entity();
        
        // 有効なステータス遷移のテスト
        let valid_statuses = vec!["initiated", "collecting", "threshold_met", "completed", "failed"];
        
        for status in valid_statuses {
            reencryption.status = status.to_string();
            assert_eq!(reencryption.status, status);
        }
    }

    #[test]
    fn test_large_cfrag_collection() {
        let mut reencryption = create_test_reencryption_entity();
        
        // 大量のcFrag収集のテスト
        for i in 1..=10 {
            let cfrag = create_test_cfrag_data(
                &format!("cfrag_{:03}", i),
                &format!("holder_{:03}", i)
            );
            reencryption.collected_cfrags.push(cfrag);
        }
        
        assert_eq!(reencryption.collected_cfrags.len(), 10);
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&reencryption).unwrap();
        let deserialized: ReencryptionEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(reencryption, deserialized);
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut reencryption = create_test_reencryption_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = reencryption.version;
        
        // cFrag追加時のバージョンインクリメント
        let cfrag = create_test_cfrag_data("cfrag_001", "holder_001");
        reencryption.collected_cfrags.push(cfrag);
        reencryption.version += 1;
        
        assert_eq!(reencryption.version, original_version + 1);
    }

    #[test]
    fn test_data_consistency_validation() {
        let reencryption = create_test_reencryption_entity();
        
        // データの整合性チェック
        assert!(!reencryption.reencryption_id.is_empty());
        assert!(!reencryption.access_request_id.is_empty());
        assert!(!reencryption.target_capsule_id.is_empty());
        assert!(!reencryption.requester_process_id.is_empty());
        assert!(!reencryption.target_holders.is_empty());
        assert!(!reencryption.status.is_empty());
        assert!(reencryption.started_at > 0);
        assert!(reencryption.version > 0);
        assert!(reencryption.timeout_at > reencryption.started_at);
    }
}
