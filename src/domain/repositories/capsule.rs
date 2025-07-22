//! CapsuleEntityのRepositoryインターフェース定義
//!
//! PREカプセルの永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::CapsuleEntity;

use super::Repository;

/// CapsuleEntityリポジトリインターフェース
///
/// PREカプセルの永続化操作を提供
#[async_trait]
pub trait CapsuleEntityRepository: Repository<CapsuleEntity, String> {
    /// データID別検索
    ///
    /// # 引数
    /// - `data_id`: データグループ識別子
    ///
    /// # 戻り値
    /// 同一データグループの全カプセル
    async fn find_by_data_id(&self, data_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// 秘密ID別検索
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 同一秘密に関連する全カプセル
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// カプセルインデックス指定検索
    ///
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// - `index`: カプセルインデックス
    ///
    /// # 戻り値
    /// 指定インデックスのカプセル
    ///
    /// # 使用シーン
    /// Phase 4で特定のShareに対応するCapsuleを取得
    async fn find_by_capsule_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<CapsuleEntity>, Self::Error>;

    /// オーナー別検索
    ///
    /// # 引数
    /// - `owner_public_key`: オーナー公開鍵
    ///
    /// # 戻り値
    /// 指定オーナーが所有する全カプセル
    async fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// 対応する暗号文ID別検索
    ///
    /// # 引数
    /// - `ciphertext_id`: 暗号文識別子（ShareEntity ID）
    ///
    /// # 戻り値
    /// 対応するカプセル
    async fn find_by_ciphertext_id(
        &self,
        ciphertext_id: &str,
    ) -> Result<Option<CapsuleEntity>, Self::Error>;

    /// データサイズ範囲検索
    ///
    /// # 引数
    /// - `min_size`: 最小サイズ（バイト）
    /// - `max_size`: 最大サイズ（バイト）
    ///
    /// # 戻り値
    /// サイズ範囲内のカプセルのリスト
    ///
    /// # 使用シーン
    /// ストレージ管理、統計分析
    async fn find_by_size_range(
        &self,
        min_size: usize,
        max_size: usize,
    ) -> Result<Vec<CapsuleEntity>, Self::Error>;
}