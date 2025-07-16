//! AccessRequestEntity - Access request and verification
//!
//! Manages data access requests and EVM verification (Phase 2).
//! Central to access control and permission validation.

use serde::{Deserialize, Serialize};

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

    // 文字列ベースのステータス管理により、
    // 新しい状態の追加が既存データを破壊しない
    pub status: String,

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    fn create_test_access_request_entity() -> AccessRequestEntity {
        AccessRequestEntity {
            request_id: "request_001".to_string(),
            target_data_id: "data_001".to_string(),
            target_secret_id: "secret_001".to_string(),
            requester_process_id: "requester_001".to_string(),
            accessor_public_key: vec![0x01, 0x02, 0x03, 0x04],
            owner_public_key: vec![0x05, 0x06, 0x07, 0x08],
            evm_verification: create_test_evm_verification_data(),
            proof_pkg: Some(create_test_proof_pkg_data()),
            status: "pending".to_string(),
            created_at: 1642000000,
            evm_verified_at: None,
            completed_at: None,
            timeout_at: 1642000300, // 5分後
            version: 1,
        }
    }

    fn create_test_evm_verification_data() -> EvmVerificationData {
        EvmVerificationData {
            tx_hash: "0x1234567890abcdef".to_string(),
            contract_address: "0x0987654321fedcba".to_string(),
            verification_event: vec![0x09, 0x0a, 0x0b, 0x0c],
            block_height: 1000000,
            verified_at: 1642000100,
        }
    }

    fn create_test_proof_pkg_data() -> ProofPkgData {
        ProofPkgData {
            proof_pkg_id: "proof_001".to_string(),
            proof_package: vec![0x0d, 0x0e, 0x0f, 0x10],
            created_at: 1642000050,
            verified: true,
        }
    }

    #[test]
    fn test_access_request_entity_serialization() {
        let request = create_test_access_request_entity();
        
        // JSON シリアライゼーション/デシリアライゼーション
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AccessRequestEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request, deserialized);
    }

    #[test]
    fn test_access_request_entity_clone_and_equality() {
        let request = create_test_access_request_entity();
        let cloned = request.clone();
        
        assert_eq!(request, cloned);
    }

    #[test]
    fn test_status_transition_validation() {
        let mut request = create_test_access_request_entity();
        
        // 有効なステータス遷移のテスト
        let valid_statuses = vec!["pending", "evm_verified", "approved", "rejected", "completed"];
        
        for status in valid_statuses {
            request.status = status.to_string();
            assert_eq!(request.status, status);
        }
    }

    #[test]
    fn test_timeout_validation() {
        let request = create_test_access_request_entity();
        
        // タイムアウトが作成時刻よりも後であることを確認
        assert!(request.timeout_at > request.created_at);
        
        // タイムアウトが正しく設定されていることを確認
        assert!(request.timeout_at > 0);
    }

    #[test]
    fn test_evm_verification_data_validation() {
        let evm_verification = create_test_evm_verification_data();
        
        // EVM検証データの整合性チェック
        assert!(!evm_verification.tx_hash.is_empty());
        assert!(!evm_verification.contract_address.is_empty());
        assert!(!evm_verification.verification_event.is_empty());
        assert!(evm_verification.block_height > 0);
        assert!(evm_verification.verified_at > 0);
        
        // トランザクションハッシュが0xで始まることを確認
        assert!(evm_verification.tx_hash.starts_with("0x"));
        assert!(evm_verification.contract_address.starts_with("0x"));
    }

    #[test]
    fn test_proof_pkg_data_validation() {
        let proof_pkg = create_test_proof_pkg_data();
        
        // ProofPkgデータの整合性チェック
        assert!(!proof_pkg.proof_pkg_id.is_empty());
        assert!(!proof_pkg.proof_package.is_empty());
        assert!(proof_pkg.created_at > 0);
        assert!(proof_pkg.verified);
    }

    #[test]
    fn test_access_request_lifecycle_simulation() {
        let mut request = create_test_access_request_entity();
        
        // アクセスリクエストのライフサイクルシミュレーション
        
        // Phase 1: ペンディング状態
        assert_eq!(request.status, "pending");
        assert!(request.evm_verified_at.is_none());
        assert!(request.completed_at.is_none());
        
        // Phase 2: EVM検証完了
        request.status = "evm_verified".to_string();
        request.evm_verified_at = Some(1642000100);
        request.version += 1;
        
        assert_eq!(request.status, "evm_verified");
        assert!(request.evm_verified_at.is_some());
        assert!(request.evm_verified_at.unwrap() > request.created_at);
        
        // Phase 3: 承認
        request.status = "approved".to_string();
        request.version += 1;
        
        assert_eq!(request.status, "approved");
        
        // Phase 4: 完了
        request.status = "completed".to_string();
        request.completed_at = Some(1642000200);
        request.version += 1;
        
        assert_eq!(request.status, "completed");
        assert!(request.completed_at.is_some());
        assert!(request.completed_at.unwrap() > request.evm_verified_at.unwrap());
    }

    #[test]
    fn test_timeout_enforcement() {
        let mut request = create_test_access_request_entity();
        
        // タイムアウトのシミュレーション
        let current_time = 1642000400; // timeout_atよりも後
        
        // タイムアウトチェック
        let is_timed_out = current_time > request.timeout_at;
        assert!(is_timed_out);
        
        // タイムアウト時のステータス更新
        if is_timed_out {
            request.status = "rejected".to_string();
            request.version += 1;
        }
        
        assert_eq!(request.status, "rejected");
    }

    #[test]
    fn test_optional_proof_pkg_handling() {
        let mut request = create_test_access_request_entity();
        
        // ProofPkgがNoneの場合のテスト
        request.proof_pkg = None;
        
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AccessRequestEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request, deserialized);
        assert!(request.proof_pkg.is_none());
    }

    #[test]
    fn test_public_key_validation() {
        let request = create_test_access_request_entity();
        
        // 公開鍵の整合性チェック
        assert!(!request.accessor_public_key.is_empty());
        assert!(!request.owner_public_key.is_empty());
        
        // 公開鍵の長さが正しいことを確認（仮定として最小長をチェック）
        assert!(request.accessor_public_key.len() > 0);
        assert!(request.owner_public_key.len() > 0);
    }

    #[test]
    fn test_version_increment_simulation() {
        let mut request = create_test_access_request_entity();
        
        // 楽観的ロックのバージョン更新シミュレーション
        let original_version = request.version;
        
        // ステータス更新時のバージョンインクリメント
        request.status = "evm_verified".to_string();
        request.evm_verified_at = Some(1642000100);
        request.version += 1;
        
        assert_eq!(request.version, original_version + 1);
    }

    #[test]
    fn test_large_event_data_handling() {
        let mut request = create_test_access_request_entity();
        
        // 大きなイベントデータの処理テスト
        let large_event_data = vec![0u8; 1024 * 32]; // 32KB
        request.evm_verification.verification_event = large_event_data;
        
        // シリアライゼーション/デシリアライゼーションが成功することを確認
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AccessRequestEntity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(request, deserialized);
        assert_eq!(request.evm_verification.verification_event.len(), 1024 * 32);
    }

    #[test]
    fn test_data_consistency_validation() {
        let request = create_test_access_request_entity();
        
        // データの整合性チェック
        assert!(!request.request_id.is_empty());
        assert!(!request.target_data_id.is_empty());
        assert!(!request.target_secret_id.is_empty());
        assert!(!request.requester_process_id.is_empty());
        assert!(!request.status.is_empty());
        assert!(request.created_at > 0);
        assert!(request.version > 0);
    }
}
