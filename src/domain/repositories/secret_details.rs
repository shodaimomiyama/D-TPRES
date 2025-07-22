//! SecretDetailsEntityのRepositoryインターフェース定義
//!
//! 秘密管理詳細情報の永続化操作を提供

use async_trait::async_trait;
use std::collections::HashMap;

use crate::domain::entities::{AccessRecord, SecretDetailsEntity};

use super::Repository;

/// SecretDetailsEntityリポジトリインターフェース
///
/// 秘密管理詳細情報の永続化操作を提供
#[async_trait]
pub trait SecretDetailsEntityRepository: Repository<SecretDetailsEntity, String> {
    /// 秘密ID別検索
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 指定秘密の詳細情報
    async fn find_by_secret_id(&self, secret_id: &str)
        -> Result<Option<SecretDetailsEntity>, Self::Error>;

    /// アクセス制御条件別検索
    ///
    /// # 引数
    /// - `condition`: アクセス制御条件
    ///
    /// # 戻り値
    /// 指定条件を持つ秘密詳細のリスト
    async fn find_by_access_control_condition(
        &self,
        condition: &str,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// 期限切れ秘密検索
    ///
    /// # 引数
    /// - `current_time`: 現在時刻（Unix timestamp）
    ///
    /// # 戻り値
    /// 期限切れの秘密詳細リスト
    async fn find_expired_secrets(
        &self,
        current_time: u64,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// アクセス履歴追加
    ///
    /// # 引数
    /// - `details_id`: 詳細エンティティID
    /// - `access_record`: 追加するアクセス記録
    ///
    /// # 使用シーン
    /// - Phase 2でアクセス要求が発生した際
    /// - Phase 5で復号が完了した際
    async fn add_access_record(
        &self,
        details_id: &str,
        access_record: &AccessRecord,
    ) -> Result<(), Self::Error>;

    /// kFrag群更新
    ///
    /// # 引数
    /// - `details_id`: 詳細エンティティID
    /// - `condition`: アクセス制御条件
    /// - `kfrag_ids`: kFragエンティティIDリスト
    ///
    /// # 使用シーン
    /// - Phase 3でkFragが生成・配布された際
    async fn update_kfrags_for_condition(
        &self,
        details_id: &str,
        condition: &str,
        kfrag_ids: &[String],
    ) -> Result<(), Self::Error>;

    /// メタデータ更新
    ///
    /// # 引数
    /// - `details_id`: 詳細エンティティID
    /// - `metadata`: 新しいメタデータ
    ///
    /// # 実装注意点
    /// - 既存のメタデータにマージ
    async fn update_metadata(
        &self,
        details_id: &str,
        metadata: &HashMap<String, String>,
    ) -> Result<(), Self::Error>;

    /// アクティブな秘密詳細取得
    ///
    /// # 戻り値
    /// 期限切れでない秘密詳細のリスト
    ///
    /// # 使用シーン
    /// - 定期的な状態確認
    /// - 統計情報の取得
    async fn find_active_details(&self) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// アクセス頻度別ランキング取得
    ///
    /// # 引数
    /// - `limit`: 取得する最大件数
    /// - `time_range`: 集計期間（秒）
    ///
    /// # 戻り値
    /// アクセス頻度降順の秘密詳細リスト
    async fn find_by_access_frequency_desc(
        &self,
        limit: usize,
        time_range: u64,
    ) -> Result<Vec<SecretDetailsEntity>, Self::Error>;

    /// 条件別kFrag統計取得
    ///
    /// # 戻り値
    /// 条件名とkFrag数のタプルリスト
    ///
    /// # 使用シーン
    /// - システム状態の監視
    /// - リソース使用状況の把握
    async fn get_kfrag_statistics(&self) -> Result<Vec<(String, usize)>, Self::Error>;
}