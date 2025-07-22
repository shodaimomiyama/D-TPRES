//! ReencryptionEntityリポジトリインターフェース

use crate::domain::entities::{CFragData, ReencryptionEntity, ReencryptionStatus};

use super::Repository;

/// プロキシ再暗号化永続化用のReencryptionEntityリポジトリインターフェース
pub trait ReencryptionEntityRepository: Repository<ReencryptionEntity, String> {
    /// アクセス要求IDで再暗号化を検索
    fn find_by_access_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// リクエスタープロセスIDで再暗号化を検索
    fn find_by_requester_process_id(
        &self,
        process_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// ターゲットカプセルIDで再暗号化を検索
    fn find_by_target_capsule_id(
        &self,
        capsule_id: &str,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// ステータスで再暗号化を検索
    fn find_by_status(
        &self,
        status: ReencryptionStatus,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 監視用にアクティブな再暗号化を検索
    fn find_active_reencryptions(&self) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// タイムアウトした再暗号化を検索
    fn find_timed_out_reencryptions(
        &self,
        current_time: u64,
    ) -> Result<Vec<ReencryptionEntity>, Self::Error>;

    /// 検証と閾値達成時の自動更新付きでステータスを更新
    fn update_status(
        &self,
        reencryption_id: &str,
        status: ReencryptionStatus,
    ) -> Result<(), Self::Error>;

    /// Phase 4でHolderからcFragを追加
    /// 重複チェックと閾値検証を含む
    fn add_cfrag(&self, reencryption_id: &str, cfrag: &CFragData) -> Result<(), Self::Error>;

    /// 閾値cFragが収集されたか確認
    fn check_threshold_met(&self, reencryption_id: &str) -> Result<bool, Self::Error>;

    /// Phase 4で閾値cFrag収集時に完了としてマーク
    fn mark_completed(&self, reencryption_id: &str, completed_at: u64) -> Result<(), Self::Error>;
}
