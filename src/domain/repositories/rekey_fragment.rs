//! RekeyFragmentEntityリポジトリインターフェース

use crate::domain::entities::{RekeyFragmentEntity, RekeyFragmentStatus};

use super::Repository;

/// kFrag永続化用のRekeyFragmentEntityリポジトリインターフェース
pub trait RekeyFragmentEntityRepository: Repository<RekeyFragmentEntity, String> {
    /// シークレットIDでフラグメントを検索
    fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// アクセス要求IDでフラグメントを検索
    fn find_by_access_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// アクセス制御条件でフラグメントを検索
    fn find_by_access_control_condition(
        &self,
        condition: &str,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 割り当てられたホルダーでフラグメントを検索
    fn find_by_holder(&self, holder_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// ステータスでフラグメントを検索
    fn find_by_status(
        &self,
        status: RekeyFragmentStatus,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 条件とアクセサーのアクティブフラグメントを検索
    /// Phase 4で再暗号化可能なkFragを見つけるために使用
    fn find_active_fragments_for_condition(
        &self,
        access_control_condition: &str,
        accessor_public_key: &[u8],
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// クリーンアップ用に期限切れフラグメントを検索
    fn find_expired_fragments(
        &self,
        current_time: u64,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 検証付きでフラグメントステータスを更新
    fn update_status(
        &self,
        fragment_id: &str,
        status: RekeyFragmentStatus,
    ) -> Result<(), Self::Error>;

    /// Phase 3でフラグメントを配布済みとしてマーク
    fn mark_distributed(&self, fragment_id: &str, distributed_at: u64) -> Result<(), Self::Error>;

    /// Phase 3割り当て用にホルダー負荷分散を取得
    /// (holder_id, fragment_count)タプルのリストを返す
    fn get_holder_load_distribution(&self) -> Result<Vec<(String, u64)>, Self::Error>;
}