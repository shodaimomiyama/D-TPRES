//! CapsuleEntity - PRE capsule representation
//!
//! Capsule information for Proxy Re-Encryption (Phase 1).
//! Used in Phase 4 for re-encryption operations.

use serde::{Deserialize, Serialize};

/// Capsule entity - PRE encryption capsule
///
/// Generated in Phase 1, used in Phase 4 re-encryption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
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
