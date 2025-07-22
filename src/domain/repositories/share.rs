//! ShareEntityのRepositoryインターフェース定義
//!
//! Shamirデータシェアの永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::ShareEntity;

use super::Repository;

/// ShareEntityリポジトリインターフェース
///
/// Shamirデータシェアの永続化操作を提供
#[async_trait]
pub trait ShareEntityRepository: Repository<ShareEntity, String> {
    /// データID別検索
    ///
    /// # 引数
    /// - `data_id`: データグループ識別子
    ///
    /// # 戻り値
    /// 同一データグループの全シェア
    async fn find_by_data_id(&self, data_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;

    /// 秘密ID別検索
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 同一秘密から生成された全シェア
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;

    /// オーナー別検索
    ///
    /// # 引数
    /// - `owner_public_key`: オーナー公開鍵
    ///
    /// # 戻り値
    /// 指定オーナーが所有する全シェア
    async fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<ShareEntity>, Self::Error>;

    /// 閾値インデックス指定検索
    ///
    /// # 引数
    /// - `data_id`: データグループ識別子
    /// - `index`: 閾値インデックス（1からn）
    ///
    /// # 戻り値
    /// 指定インデックスのシェア
    async fn find_by_threshold_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<ShareEntity>, Self::Error>;

    /// Shamir再構築用シェア収集
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    /// - `threshold`: 必要な閾値数（k）
    ///
    /// # 戻り値
    /// 再構築に必要な最小数のシェア
    ///
    /// # 実装注意点
    /// - 利用可能なシェアから最適なk個を選択
    /// - 最終アクセス時刻等を考慮した選択
    async fn collect_shares_for_reconstruction(
        &self,
        secret_id: &str,
        threshold: u8,
    ) -> Result<Vec<ShareEntity>, Self::Error>;

    /// 完全性検証用データ取得
    ///
    /// # 引数
    /// - `share_id`: シェア識別子
    ///
    /// # 戻り値
    /// - `true`: 整合性検証成功
    /// - `false`: 整合性検証失敗
    ///
    /// # 実装注意点
    /// - integrity_hashを使用した検証
    async fn verify_share_integrity(&self, share_id: &str) -> Result<bool, Self::Error>;

    /// 最終アクセス時刻更新
    ///
    /// # 引数
    /// - `share_id`: シェア識別子
    /// - `accessed_at`: アクセス時刻
    ///
    /// # 使用シーン
    /// - シェア読み込み時
    /// - 再構築処理時
    async fn update_last_accessed(&self, share_id: &str, accessed_at: u64) -> Result<(), Self::Error>;

    /// 古いシェア検索（クリーンアップ用）
    ///
    /// # 引数
    /// - `threshold_date`: 閾値日時（Unix timestamp）
    ///
    /// # 戻り値
    /// 指定日時より古いシェアのリスト
    async fn find_older_than(&self, threshold_date: u64) -> Result<Vec<ShareEntity>, Self::Error>;
}