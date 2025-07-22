//! RekeyFragmentEntityのRepositoryインターフェース定義
//!
//! 再暗号化キーフラグメントの永続化操作を提供

use async_trait::async_trait;

use crate::domain::entities::{RekeyFragmentEntity, RekeyFragmentStatus};

use super::Repository;

/// RekeyFragmentEntityリポジトリインターフェース
///
/// 再暗号化キーフラグメントの永続化操作を提供
#[async_trait]
pub trait RekeyFragmentEntityRepository: Repository<RekeyFragmentEntity, String> {
    /// 秘密ID別検索
    ///
    /// # 引数
    /// - `secret_id`: 秘密識別子
    ///
    /// # 戻り値
    /// 指定秘密に関連する全kFrag
    async fn find_by_secret_id(&self, secret_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// アクセス要求ID別検索
    ///
    /// # 引数
    /// - `request_id`: アクセス要求識別子
    ///
    /// # 戻り値
    /// 指定要求に関連する全kFrag
    async fn find_by_access_request_id(
        &self,
        request_id: &str,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// アクセス制御条件別検索
    ///
    /// # 引数
    /// - `condition`: アクセス制御条件
    ///
    /// # 戻り値
    /// 指定条件に関連する全kFrag
    async fn find_by_access_control_condition(
        &self,
        condition: &str,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// Holder別検索
    ///
    /// # 引数
    /// - `holder_id`: HolderプロセスID
    ///
    /// # 戻り値
    /// 指定Holderが保持する全kFrag
    async fn find_by_holder(&self, holder_id: &str) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 状態別検索
    ///
    /// # 引数
    /// - `status`: 状態（RekeyFragmentStatus）
    ///
    /// # 戻り値
    /// 指定状態のkFragリスト
    async fn find_by_status(
        &self,
        status: RekeyFragmentStatus,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// アクティブフラグメント検索
    ///
    /// # 引数
    /// - `access_control_condition`: アクセス制御条件
    /// - `accessor_public_key`: アクセス者公開鍵
    ///
    /// # 戻り値
    /// 使用可能なアクティブkFragのリスト
    ///
    /// # 使用シーン
    /// Phase 4で再暗号化可能なkFragを探す際
    async fn find_active_fragments_for_condition(
        &self,
        access_control_condition: &str,
        accessor_public_key: &[u8],
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 期限切れフラグメント検索
    ///
    /// # 引数
    /// - `current_time`: 現在時刻
    ///
    /// # 戻り値
    /// 期限切れのkFragリスト
    async fn find_expired_fragments(
        &self,
        current_time: u64,
    ) -> Result<Vec<RekeyFragmentEntity>, Self::Error>;

    /// 状態更新
    ///
    /// # 引数
    /// - `fragment_id`: フラグメント識別子
    /// - `status`: 新しい状態
    ///
    /// # 実装注意点
    /// - 状態遷移の妥当性チェック
    async fn update_status(
        &self,
        fragment_id: &str,
        status: RekeyFragmentStatus,
    ) -> Result<(), Self::Error>;

    /// 配布マーキング
    ///
    /// # 引数
    /// - `fragment_id`: フラグメント識別子
    /// - `distributed_at`: 配布時刻
    ///
    /// # 使用シーン
    /// Phase 3でHolderへの配布が完了した際
    async fn mark_distributed(&self, fragment_id: &str, distributed_at: u64) -> Result<(), Self::Error>;

    /// Holder負荷分散情報取得
    ///
    /// # 戻り値
    /// HolderIDと保持kFrag数のタプルリスト
    ///
    /// # 使用シーン
    /// Phase 3で新規kFrag配布先を決定する際
    async fn get_holder_load_distribution(&self) -> Result<Vec<(String, u64)>, Self::Error>;
}