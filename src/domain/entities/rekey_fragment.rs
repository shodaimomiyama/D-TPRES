//! RekeyFragmentEntity - Re-encryption key fragment representation
//!
//! Shamir-split re-encryption key fragments (kFrag) for threshold re-encryption (Phase 3).
//! Distributed to holders for decentralized re-encryption capability.

use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::value_objects::RekeyFragmentStatus;

/// Re-encryption key fragment entity - Split re-encryption key
///
/// Generated in Phase 3, distributed to Holders
/// Contains sensitive key material that must be zeroized
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Zeroize, ZeroizeOnDrop))]
#[non_exhaustive]
pub struct RekeyFragmentEntity {
    // ID系フィールドにzeroize(skip)を適用し、機密データのみをゼロ化対象にすることで、
    // セキュリティとパフォーマンスのバランスを最適化
    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub fragment_id: String,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub secret_id: String,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub access_request_id: String,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub access_control_condition: String,

    // 公開鍵はゼロ化不要だが、統一性のためzeroize(skip)を適用
    // 将来的に公開鍵の扱いが変わっても影響を最小化
    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub owner_public_key: Vec<u8>,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub accessor_public_key: Vec<u8>,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub shamir_index: u8,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub shamir_threshold: u8,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub shamir_total_fragments: u8,

    // kFragjは再暗号化鍵の断片で、最も重要な機密データ
    // zeroize(skip)を付けずにメモリから確実に消去することで、
    // サイドチャネル攻撃やメモリダンプによる漏洩を防ぐ
    pub kfrag_data: Vec<u8>,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub assigned_holder_id: String,

    // 型安全なステータス管理により、
    // 不正な状態遷移を防止
    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub status: RekeyFragmentStatus,

    // 有効期限をOptionalにすることで、永続的なkFragと
    // 時限的なkFragの両方をサポート
    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub expires_at: Option<u64>,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub created_at: u64,

    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub distributed_at: Option<u64>,

    // AOのステートレス環境で同時更新を検出するための楽観的ロック
    #[cfg_attr(not(target_arch = "wasm32"), zeroize(skip))]
    pub version: u64,
}
