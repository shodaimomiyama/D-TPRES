//! RekeyFragmentEntity - Re-encryption key fragment representation
//!
//! Shamir-split re-encryption key fragments (kFrag) for threshold re-encryption (Phase 3).
//! Distributed to holders for decentralized re-encryption capability.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Re-encryption key fragment entity - Split re-encryption key
///
/// Generated in Phase 3, distributed to Holders
/// Contains sensitive key material that must be zeroized
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct RekeyFragmentEntity {
    // ID系フィールドにzeroize(skip)を適用し、機密データのみをゼロ化対象にすることで、
    // セキュリティとパフォーマンスのバランスを最適化
    #[zeroize(skip)]
    pub fragment_id: String,

    #[zeroize(skip)]
    pub secret_id: String,

    #[zeroize(skip)]
    pub access_request_id: String,

    #[zeroize(skip)]
    pub access_control_condition: String,

    // 公開鍵はゼロ化不要だが、統一性のためzeroize(skip)を適用
    // 将来的に公開鍵の扱いが変わっても影響を最小化
    #[zeroize(skip)]
    pub owner_public_key: Vec<u8>,

    #[zeroize(skip)]
    pub accessor_public_key: Vec<u8>,

    #[zeroize(skip)]
    pub shamir_index: u8,

    #[zeroize(skip)]
    pub shamir_threshold: u8,

    #[zeroize(skip)]
    pub shamir_total_fragments: u8,

    // kFragjは再暗号化鍵の断片で、最も重要な機密データ
    // zeroize(skip)を付けずにメモリから確実に消去することで、
    // サイドチャネル攻撃やメモリダンプによる漏洩を防ぐ
    pub kfrag_data: Vec<u8>,

    #[zeroize(skip)]
    pub assigned_holder_id: String,

    // 文字列ベースのステータス管理により、
    // 新しい状態の追加が既存データを破壊しない
    #[zeroize(skip)]
    pub status: String,

    // 有効期限をOptionalにすることで、永続的なkFragと
    // 時限的なkFragの両方をサポート
    #[zeroize(skip)]
    pub expires_at: Option<u64>,

    #[zeroize(skip)]
    pub created_at: u64,

    #[zeroize(skip)]
    pub distributed_at: Option<u64>,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    #[zeroize(skip)]
    pub version: u64,
}
