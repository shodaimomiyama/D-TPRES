//! AccessRequestEntityリポジトリインターフェース

use crate::domain::entities::{
    AccessRequestEntity, AccessRequestStatus, EvmVerificationData, ProofPkgData,
};

use super::Repository;

/// アクセス要求永続化用のAccessRequestEntityリポジトリインターフェース
pub trait AccessRequestEntityRepository: Repository<AccessRequestEntity, String> {
    /// リクエスターで要求を検索
    fn find_by_requester(
        &self,
        requester_process_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// ターゲットデータIDで要求を検索
    fn find_by_target_data_id(
        &self,
        data_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// ターゲットシークレットIDで要求を検索
    fn find_by_target_secret_id(
        &self,
        secret_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// ステータスで要求を検索
    fn find_by_status(
        &self,
        status: AccessRequestStatus,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// アクセサー公開鍵で要求を検索
    fn find_by_accessor_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// Phase 3移行用にEVM検証済み要求を検索
    fn find_evm_verified_requests(&self) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// タイムアウトした要求を検索
    fn find_timed_out_requests(
        &self,
        current_time: u64,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// 検証とタイムスタンプ更新付きで要求ステータスを更新
    fn update_status(
        &self,
        request_id: &str,
        status: AccessRequestStatus,
    ) -> Result<(), Self::Error>;

    /// Phase 2でEVM検証データを更新
    fn update_evm_verification(
        &self,
        request_id: &str,
        verification_data: &EvmVerificationData,
    ) -> Result<(), Self::Error>;

    /// elciao生成後にProofPkgデータを設定
    fn set_proof_pkg(
        &self,
        request_id: &str,
        proof_pkg: &ProofPkgData,
    ) -> Result<(), Self::Error>;

    /// Phase 5で要求を完了としてマーク
    fn mark_completed(&self, request_id: &str, completed_at: u64) -> Result<(), Self::Error>;
}