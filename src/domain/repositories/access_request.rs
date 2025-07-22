//! AccessRequestEntityのRepositoryインターフェース定義
//!
//! アクセス要求の永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::{
    AccessRequestEntity, AccessRequestStatus, EvmVerificationData, ProofPkgData,
};

use super::Repository;

/// AccessRequestEntityリポジトリインターフェース
///
/// アクセス要求の永続化操作を提供
#[async_trait]
pub trait AccessRequestEntityRepository: Repository<AccessRequestEntity, String> {
    /// 要求者別検索
    ///
    /// # 引数
    /// - `requester_process_id`: 要求者プロセスID
    ///
    /// # 戻り値
    /// 指定要求者の全アクセス要求
    async fn find_by_requester(
        &self,
        requester_process_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// 対象データID別検索
    ///
    /// # 引数
    /// - `data_id`: データグループ識別子
    ///
    /// # 戻り値
    /// 指定データへの全アクセス要求
    async fn find_by_target_data_id(
        &self,
        data_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// 対象秘密ID別検索
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 指定秘密への全アクセス要求
    async fn find_by_target_secret_id(
        &self,
        secret_id: &str,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// 状態別検索
    ///
    /// # 引数
    /// - `status`: 状態（AccessRequestStatus）
    ///
    /// # 戻り値
    /// 指定状態のアクセス要求リスト
    async fn find_by_status(
        &self,
        status: AccessRequestStatus,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// アクセス者公開鍵別検索
    ///
    /// # 引数
    /// - `public_key`: アクセス者公開鍵（pkA）
    ///
    /// # 戻り値
    /// 指定公開鍵に関連する全アクセス要求
    async fn find_by_accessor_public_key(
        &self,
        public_key: &[u8],
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// EVM検証済み要求検索
    ///
    /// # 戻り値
    /// EVM検証が完了したアクセス要求のリスト
    ///
    /// # 使用シーン
    /// Phase 3への移行対象となる要求の抽出
    async fn find_evm_verified_requests(&self) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// タイムアウト要求検索
    ///
    /// # 引数
    /// - `current_time`: 現在時刻（Unix timestamp）
    ///
    /// # 戻り値
    /// タイムアウトしたアクセス要求のリスト
    async fn find_timed_out_requests(
        &self,
        current_time: u64,
    ) -> Result<Vec<AccessRequestEntity>, Self::Error>;

    /// 状態更新
    ///
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `status`: 新しい状態
    ///
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    /// - タイムスタンプの自動更新
    async fn update_status(
        &self,
        request_id: &str,
        status: AccessRequestStatus,
    ) -> Result<(), Self::Error>;

    /// EVM検証結果更新
    ///
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `verification_data`: EVM検証データ
    ///
    /// # 使用シーン
    /// Phase 2でEVM検証が完了した際
    async fn update_evm_verification(
        &self,
        request_id: &str,
        verification_data: &EvmVerificationData,
    ) -> Result<(), Self::Error>;

    /// ProofPkg設定
    ///
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `proof_pkg`: ProofPkgデータ
    ///
    /// # 使用シーン
    /// elciaoによるProofPkg生成完了時
    async fn set_proof_pkg(
        &self,
        request_id: &str,
        proof_pkg: &ProofPkgData,
    ) -> Result<(), Self::Error>;

    /// 完了マーキング
    ///
    /// # 引数
    /// - `request_id`: 要求識別子
    /// - `completed_at`: 完了時刻
    ///
    /// # 使用シーン
    /// Phase 5で復号が成功した際
    async fn mark_completed(&self, request_id: &str, completed_at: u64) -> Result<(), Self::Error>;
}