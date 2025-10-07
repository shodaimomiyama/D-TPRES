//! SecretDetailsEntityリポジトリインターフェース

use std::collections::HashMap;

use crate::domain::entities::{AccessRecord, SecretDetailsEntity};

use super::Repository;

/// シークレット管理詳細用のSecretDetailsEntityリポジトリインターフェース
pub trait SecretDetailsEntityRepository: Repository<SecretDetailsEntity, String> {
    /// シークレットIDで詳細を検索
    fn find_by_secret_id(
        &self,
        secret_id: &str,
    ) -> Result<Option<SecretDetailsEntity>, Self::Error>;

    /// アクセス制御条件で詳細を検索
    fn find_by_access_control_condition(
        &self,
        condition: &str,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// 期限切れのシークレットを検索
    fn find_expired_secrets(
        &self,
        current_time: u64,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// Phase 2またはPhase 5でアクセス記録を追加
    fn add_access_record(
        &self,
        details_id: &str,
        access_record: &AccessRecord,
    ) -> Result<(), Self::Error>;

    /// Phase 3で条件のkFragを更新
    fn update_kfrags_for_condition(
        &self,
        details_id: &str,
        condition: &str,
        kfrag_ids: &[String],
    ) -> Result<(), Self::Error>;

    /// 既存とマージしてメタデータを更新
    fn update_metadata(
        &self,
        details_id: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<(), Self::Error>;

    /// 監視用にアクティブ（期限切れでない）詳細を検索
    fn find_active_details(&self) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// 頻度順でトップアクセスのシークレットを検索
    fn find_by_access_frequency_desc(
        &self,
        limit: usize,
        time_range: u64,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// 監視用に条件別kFrag統計を取得
    fn get_kfrag_statistics(&self) -> Result<Vec<(String, usize)>, Self::Error>;
}
