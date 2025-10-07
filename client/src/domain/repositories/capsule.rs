//! CapsuleEntityリポジトリインターフェース

use crate::domain::entities::CapsuleEntity;

use super::Repository;

/// PREカプセル永続化用のCapsuleEntityリポジトリインターフェース
pub trait CapsuleEntityRepository: Repository<CapsuleEntity, String> {
    /// データIDでカプセルを検索
    fn find_by_data_id(&self, data_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// シークレットIDでカプセルを検索
    fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// インデックスでカプセルを検索
    /// Phase 4で特定のシェアに対応するカプセル取得に使用
    fn find_by_capsule_index(
        &self,
        data_id: &str,
        index: u8,
    ) -> Result<Option<CapsuleEntity>, Self::Error>;

    /// 所有者でカプセルを検索
    fn find_by_owner(&self, owner_public_key: &[u8]) -> Result<Vec<CapsuleEntity>, Self::Error>;

    /// 対応する暗号文IDでカプセルを検索
    fn find_by_ciphertext_id(
        &self,
        ciphertext_id: &str,
    ) -> Result<Option<CapsuleEntity>, Self::Error>;

    /// ストレージ管理用にサイズ範囲でカプセルを検索
    fn find_by_size_range(
        &self,
        min_size: usize,
        max_size: usize,
    ) -> Result<Vec<CapsuleEntity>, Self::Error>;
}
