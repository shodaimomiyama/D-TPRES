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
