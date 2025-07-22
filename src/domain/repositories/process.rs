//! ProcessEntityのRepositoryインターフェース定義
//!
//! マルチロール対応プロセスの永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::{
    HolderData, OwnerData, PerformanceMetrics, ProcessEntity, ProcessRole, RequesterData,
    SecretIndex,
};

use super::Repository;

/// ProcessEntityリポジトリインターフェース
///
/// マルチロール対応プロセスの永続化操作を提供
#[async_trait]
pub trait ProcessEntityRepository: Repository<ProcessEntity, String> {
    /// 名前による検索
    ///
    /// # 引数
    /// - `name`: プロセス名
    ///
    /// # 使用例
    /// ```rust
    /// let process = repo.find_by_name("AliceOwnerProcess").await?;
    /// ```
    async fn find_by_name(&self, name: &str) -> Result<Option<ProcessEntity>, Self::Error>;

    /// アクティブロール別検索
    ///
    /// # 引数
    /// - `role`: ロール（ProcessRole）
    ///
    /// # 戻り値
    /// 指定ロールを持つプロセスのリスト
    async fn find_by_active_role(&self, role: ProcessRole) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// Owner機能を持つプロセス検索
    ///
    /// # 戻り値
    /// owner_dataが設定されているプロセスのリスト
    async fn find_processes_with_owner_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// Holder機能を持つプロセス検索
    ///
    /// # 戻り値
    /// holder_dataが設定されているプロセスのリスト
    async fn find_processes_with_holder_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// Requester機能を持つプロセス検索
    ///
    /// # 戻り値
    /// requester_dataが設定されているプロセスのリスト
    async fn find_processes_with_requester_capability(
        &self,
    ) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// 信頼性スコア順検索（Holder選択用）
    ///
    /// # 引数
    /// - `limit`: 取得する最大件数
    ///
    /// # 戻り値
    /// 信頼性スコア降順でソートされたHolderプロセス
    ///
    /// # 使用シーン
    /// Phase 3でkFrag配布先Holderを選択する際に使用
    async fn find_holders_by_reliability_desc(
        &self,
        limit: usize,
    ) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// 負荷状況別検索（Holder選択用）
    ///
    /// # 引数
    /// - `max_load`: 最大許容負荷
    ///
    /// # 戻り値
    /// 負荷が閾値以下のHolderプロセス（負荷昇順）
    async fn find_holders_by_load_asc(
        &self,
        max_load: u64,
    ) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// 暗号操作対応別検索
    ///
    /// # 引数
    /// - `operation`: 暗号操作名（"shamir_split", "pre_encrypt"等）
    ///
    /// # 戻り値
    /// 指定操作をサポートするプロセスのリスト
    async fn find_by_crypto_operation(&self, operation: &str) -> Result<Vec<ProcessEntity>, Self::Error>;

    /// パフォーマンスメトリクス更新
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `metrics`: 新しいメトリクス
    ///
    /// # 実装注意点
    /// - 部分更新として実装
    /// - メトリクスのみ更新し、他フィールドは変更しない
    async fn update_performance_metrics(
        &self,
        process_id: &str,
        metrics: &PerformanceMetrics,
    ) -> Result<(), Self::Error>;

    /// OwnerData更新
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `owner_data`: 新しいOwnerData
    ///
    /// # 使用シーン
    /// - 新しい秘密の管理開始時
    /// - kFrag生成完了時
    async fn update_owner_data(
        &self,
        process_id: &str,
        owner_data: &OwnerData,
    ) -> Result<(), Self::Error>;

    /// HolderData更新
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `holder_data`: 新しいHolderData
    ///
    /// # 使用シーン
    /// - kFrag受信時
    /// - 再暗号化完了時
    async fn update_holder_data(
        &self,
        process_id: &str,
        holder_data: &HolderData,
    ) -> Result<(), Self::Error>;

    /// RequesterData更新
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `requester_data`: 新しいRequesterData
    ///
    /// # 使用シーン
    /// - アクセス要求作成時
    /// - cFrag収集完了時
    async fn update_requester_data(
        &self,
        process_id: &str,
        requester_data: &RequesterData,
    ) -> Result<(), Self::Error>;

    /// 秘密インデックス追加
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `secret_id`: 秘密識別子
    /// - `index`: 追加する秘密インデックス
    ///
    /// # 使用シーン
    /// - Phase 1で新しい秘密を分割した後
    ///
    /// # 実装注意点
    /// - OwnerDataのsecret_indicesに追加
    /// - 既存の秘密IDの場合は更新
    async fn add_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
        index: &SecretIndex,
    ) -> Result<(), Self::Error>;

    /// 秘密インデックス取得
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 指定秘密のインデックス情報
    ///
    /// # 使用シーン
    /// - 秘密の存在確認
    /// - 詳細Entity IDの取得
    async fn get_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
    ) -> Result<Option<SecretIndex>, Self::Error>;

    /// 秘密インデックス一覧取得
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    ///
    /// # 戻り値
    /// プロセスが管理する全秘密のインデックス情報
    ///
    /// # 使用シーン
    /// - 管理秘密の一覧表示
    /// - 統計情報の取得
    async fn list_secret_indices(
        &self,
        process_id: &str,
    ) -> Result<Vec<(String, SecretIndex)>, Self::Error>;

    /// アクティブな秘密インデックス取得
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    ///
    /// # 戻り値
    /// ステータスが"active"の秘密インデックスのみ
    async fn list_active_secret_indices(
        &self,
        process_id: &str,
    ) -> Result<Vec<(String, SecretIndex)>, Self::Error>;

    /// 秘密インデックス更新
    ///
    /// # 引数
    /// - `process_id`: 対象プロセスID
    /// - `secret_id`: 秘密識別子
    /// - `index`: 更新後のインデックス
    ///
    /// # 使用シーン
    /// - アクセス要求の追加/削除
    /// - ステータス変更
    async fn update_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
        index: &SecretIndex,
    ) -> Result<(), Self::Error>;
}