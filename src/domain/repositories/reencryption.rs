//! ReencryptionEntityのRepositoryインターフェース定義
//!
//! プロキシ再暗号化処理の永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::{CFragData, ReencryptionEntity, ReencryptionStatus};

use super::Repository;

/// ReencryptionEntityリポジトリインターフェース
///
/// プロキシ再暗号化処理の永続化操作を提供
#[async_trait]
pub trait ReencryptionEntityRepository: Repository<ReencryptionEntity, String> {
    /// アクセス要求ID別検索
    ///
    /// # 引数
    /// - `request_id`: アクセス要求識別子
    ///
    /// # 戻り値
    /// 指定要求に関連する全再暗号化処理
    async fn find_by_access_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 要求者プロセスID別検索
    ///
    /// # 引数
    /// - `process_id`: 要求者プロセスID
    ///
    /// # 戻り値
    /// 指定要求者の全再暗号化処理
    async fn find_by_requester_process_id(
        &self,
        process_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 対象カプセルID別検索
    ///
    /// # 引数
    /// - `capsule_id`: カプセル識別子
    ///
    /// # 戻り値
    /// 指定カプセルに対する再暗号化処理
    async fn find_by_target_capsule_id(
        &self,
        capsule_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 状態別検索
    ///
    /// # 引数
    /// - `status`: 状態（ReencryptionStatus）
    ///
    /// # 戻り値
    /// 指定状態の再暗号化処理リスト
    async fn find_by_status(
        &self,
        status: ReencryptionStatus,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// アクティブ再暗号化検索
    ///
    /// # 戻り値
    /// 処理中の再暗号化リスト
    ///
    /// # 使用シーン
    /// 定期的な状態確認、タイムアウト処理
    async fn find_active_reencryptions(&self) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// タイムアウト再暗号化検索
    ///
    /// # 引数
    /// - `current_time`: 現在時刻
    ///
    /// # 戻り値
    /// タイムアウトした再暗号化処理のリスト
    async fn find_timed_out_reencryptions(
        &self,
        current_time: u64,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 状態更新
    ///
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `status`: 新しい状態
    ///
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    /// - 閾値達成時の自動状態更新
    async fn update_status(
        &self,
        reencryption_id: &str,
        status: ReencryptionStatus,
    ) -> Result<(), Self::Error>;

    /// cFrag追加
    ///
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `cfrag`: 新しいcFragデータ
    ///
    /// # 使用シーン
    /// Phase 4でHolderからcFragを受信した際
    ///
    /// # 実装注意点
    /// - 重複チェック
    /// - 閾値達成チェック
    async fn add_cfrag(
        &self,
        reencryption_id: &str,
        cfrag: &CFragData,
    ) -> Result<(), Self::Error>;

    /// 閾値達成チェック
    ///
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    ///
    /// # 戻り値
    /// - `true`: 必要数のcFragが収集済み
    /// - `false`: まだ不足
    async fn check_threshold_met(&self, reencryption_id: &str) -> Result<bool, Self::Error>;

    /// 完了マーキング
    ///
    /// # 引数
    /// - `reencryption_id`: 再暗号化識別子
    /// - `completed_at`: 完了時刻
    ///
    /// # 使用シーン
    /// Phase 4で閾値数のcFragが収集できた際
    async fn mark_completed(&self, reencryption_id: &str, completed_at: u64)
        -> Result<(), Self::Error>;
}