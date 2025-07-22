//! ShareEntityリポジトリインターフェース

use crate::domain::entities::ShareEntity;

use super::Repository;

/// Shamirシェア永続化用のShareEntityリポジトリインターフェース
pub trait ShareEntityRepository: Repository<ShareEntity, String> {
    /// データIDでシェアを検索
    fn find_by_data_id(&self, data_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;

    /// シークレットIDでシェアを検索
    fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<ShareEntity>, Self::Error>;

    /// 所有者でシェアを検索
    fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<ShareEntity>, Self::Error>;

    /// 闾値インデックスでシェアを検索
    fn find_by_threshold_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<ShareEntity>, Self::Error>;

    /// Shamir再構成用にシェアを収集
    /// 可用性とアクセスパターンを考慮し最適なk個を選択
    fn collect_shares_for_reconstruction(
        &self,
        secret_id: &str,
        threshold: u8,
    ) -> Result<Vec<ShareEntity>, Self::Error>;

    /// ハッシュを使用してシェアの整合性を検証
    fn verify_share_integrity(&self, share_id: &str) -> Result<bool, Self::Error>;

    /// 最終アクセスタイムスタンプを更新
    fn update_last_accessed(&self, share_id: &str, accessed_at: u64) -> Result<(), Self::Error>;

    /// クリーンアップ用に闾値日付より古いシェアを検索
    fn find_older_than(&self, threshold_date: u64) -> Result<Vec<ShareEntity>, Self::Error>;
}