//! マルチロールプロセス永続化用のProcessEntityリポジトリインターフェース

use crate::domain::entities::{
    HolderData, OwnerData, PerformanceMetrics, ProcessEntity, ProcessRole, RequesterData,
    SecretIndex,
};

use super::Repository;

/// ロール固有のクエリ操作を持つProcessEntityリポジトリ
pub trait ProcessEntityRepository: Repository<ProcessEntity, String> {
    // AOコンテキスト内でプロセス名は一意
    fn find_by_name(&self, name: &str) -> Result<Option<ProcessEntity>, Self::Error>;

    fn find_by_active_role(&self, role: ProcessRole) -> Result<Vec<ProcessEntity>, Self::Error>;

    fn find_processes_with_owner_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;

    fn find_processes_with_holder_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;

    fn find_processes_with_requester_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error>;

    // Phase 3のkFrag配布には信頼性の高いHolderが必要
    fn find_holders_by_reliability_desc(
        &self,
        limit: usize,
    ) -> Result<Vec<ProcessEntity>, Self::Error>;

    // 効率的なkFrag配布のため負荷分散
    fn find_holders_by_load_asc(&self, max_load: u64) -> Result<Vec<ProcessEntity>, Self::Error>;

    fn find_by_crypto_operation(&self, operation: &str) -> Result<Vec<ProcessEntity>, Self::Error>;

    // AOステートレス環境で全エンティティ再読み込みを回避
    fn update_performance_metrics(
        &self,
        process_id: &str,
        metrics: &PerformanceMetrics,
    ) -> Result<(), Self::Error>;

    fn update_owner_data(
        &self,
        process_id: &str,
        owner_data: &OwnerData,
    ) -> Result<(), Self::Error>;

    fn update_holder_data(
        &self,
        process_id: &str,
        holder_data: &HolderData,
    ) -> Result<(), Self::Error>;

    fn update_requester_data(
        &self,
        process_id: &str,
        requester_data: &RequesterData,
    ) -> Result<(), Self::Error>;

    // Ownerのsecret_indicesに既存更新または新規追加
    fn add_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
        index: &SecretIndex,
    ) -> Result<(), Self::Error>;

    fn get_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
    ) -> Result<Option<SecretIndex>, Self::Error>;

    fn list_secret_indices(
        &self,
        process_id: &str,
    ) -> Result<Vec<(String, SecretIndex)>, Self::Error>;

    // 効率化のためSecretStatus::Activeでフィルタ
    fn list_active_secret_indices(
        &self,
        process_id: &str,
    ) -> Result<Vec<(String, SecretIndex)>, Self::Error>;

    fn update_secret_index(
        &self,
        process_id: &str,
        secret_id: &str,
        index: &SecretIndex,
    ) -> Result<(), Self::Error>;
}
