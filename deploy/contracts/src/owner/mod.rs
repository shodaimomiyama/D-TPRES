/// Owner-Process モジュール
///
/// D-TPRES Owner-Process の機能を提供するモジュールです。
///
/// ## 責務
///
/// Owner-Process は PRD に定義された以下の責務を担います：
///
/// ### PHASE 1 (秘密の分割と初期配布)
/// - O-Browser からの kFrag を受信・検証・保存
///
/// ### PHASE 2 (キーフラグメントの分散管理)
/// - RandAO を利用して n個の Holder-Process を選出
/// - 各 Holder-Process に kFragⱼ と署名を送信
/// - 配布状況の追跡・管理
///
/// ## アーキテクチャ
///
/// このモジュールは以下のサブモジュールで構成されます：
///
/// - `storage`: Owner専用のストレージ操作
/// - `handlers`: Owner-Process のメッセージハンドラ
///
/// ## セキュリティ要件
///
/// - すべての秘密データは `Zeroize` トレイトを実装
/// - 定数時間操作による情報漏洩の防止
/// - プロセス役割の厳格な検証
/// - メモリ安全性の確保
///
/// ## 使用方法
///
/// ```rust
/// use crate::owner::handlers::{
///     handle_distribute_kfrags,
///     handle_update_holder_assignment,
/// };
/// use crate::owner::storage::{
///     load_owner_state,
///     save_owner_state,
/// };
/// ```

pub mod storage;
pub mod handlers;

// 公開API
pub use storage::{
    OwnerProcessState,
    OwnerStorageResult,
    load_owner_state,
    save_owner_state,
    store_kfrag,
    mark_kfrag_distributed,
    assign_holder,
    update_holder_assignment_status,
    get_kfrags_by_holder,
    get_undistributed_kfrags,
    get_holder_assignment_stats,
    update_owner_config,
    cleanup_sensitive_data,
    verify_owner_role,
};

pub use handlers::{
    RandAOSelection,
    KFragDistributionResult,
    handle_distribute_kfrags,
    handle_update_holder_assignment,
    handle_distribution_confirmation,
    query_owner_metadata,
    query_kfrags,
    query_holder_assignments,
    query_distribution_stats,
};

/// Owner-Process の主要機能を統合したファサード
///
/// このファサードは Owner-Process の主要操作を簡潔な API で提供します。
pub struct OwnerProcessFacade;

impl OwnerProcessFacade {
    /// kFrag の配布を実行
    ///
    /// この関数は以下の処理を順次実行します：
    /// 1. プロセス役割の検証
    /// 2. kFrag の検証とストレージへの保存
    /// 3. RandAO による Holder 選出
    /// 4. Holder への kFrag 配布
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `kfrags` - 配布する kFrag のリスト
    ///
    /// # 戻り値
    ///
    /// 配布結果を含む Response または ContractError
    pub fn distribute_kfrags(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        kfrags: Vec<crate::msg::KFragDistribution>,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_distribute_kfrags(deps, env, info, kfrags)
    }

    /// Holder 割り当ての更新
    ///
    /// 指定された Holder に対して新しい kFrag を割り当てます。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `holder_id` - 割り当て対象の Holder ID
    /// * `kfrag_ids` - 割り当てる kFrag ID のリスト
    pub fn update_holder_assignment(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        holder_id: String,
        kfrag_ids: Vec<String>,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_update_holder_assignment(deps, env, info, holder_id, kfrag_ids)
    }

    /// 配布確認の処理
    ///
    /// Holder-Process からの配布確認を受信し、状態を更新します。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `holder_id` - 確認を送信した Holder の ID
    /// * `kfrag_id` - 対象の kFrag ID
    /// * `success` - 配布成功フラグ
    pub fn handle_distribution_confirmation(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        holder_id: String,
        kfrag_id: String,
        success: bool,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_distribution_confirmation(deps, env, info, holder_id, kfrag_id, success)
    }

    /// Owner メタデータの取得
    ///
    /// Owner-Process の基本的なメタデータ（閾値、Holder 数など）を取得します。
    pub fn get_metadata(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_owner_metadata(deps)
    }

    /// kFrag 情報の取得
    ///
    /// 指定された条件に基づいて kFrag の情報を取得します。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `holder_id` - 特定の Holder の kFrag のみを取得する場合は Some(holder_id)
    pub fn get_kfrags(
        deps: cosmwasm_std::Deps,
        holder_id: Option<String>,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_kfrags(deps, holder_id)
    }

    /// Holder 割り当て情報の取得
    ///
    /// すべての Holder の割り当て状況を取得します。
    pub fn get_holder_assignments(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_holder_assignments(deps)
    }

    /// 配布統計情報の取得
    ///
    /// kFrag の配布状況に関する統計情報を取得します。
    pub fn get_distribution_stats(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_distribution_stats(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::state::{OwnerMetadata, OwnerConfig, ProcessRole, OWNER_METADATA, OWNER_CONFIG};
    use crate::msg::KFragDistribution;

    fn setup_test_owner_process(deps: cosmwasm_std::DepsMut) {
        let metadata = OwnerMetadata {
            threshold_k: 3,
            total_holders_n: 5,
            capsule_txid: "test_capsule".to_string(),
            requester_pubkey: "test_pubkey".to_string(),
            creation_time: 1000,
        };

        let config = OwnerConfig {
            process_role: ProcessRole::Owner,
            encryption_key: "test_key".to_string(),
            authorized_holders: vec![],
        };

        OWNER_METADATA.save(deps.storage, &metadata).unwrap();
        OWNER_CONFIG.save(deps.storage, &config).unwrap();
    }

    #[test]
    fn test_facade_distribute_kfrags() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);

        setup_test_owner_process(deps.as_mut());

        let kfrags = vec![
            KFragDistribution {
                id: "kfrag_1".to_string(),
                encrypted_data: vec![1, 2, 3, 4],
                holder_id: "holder_1".to_string(),
                holder_process_id: "holder_process_1".to_string(),
                signature: vec![5, 6, 7, 8],
            },
        ];

        let result = OwnerProcessFacade::distribute_kfrags(
            deps.as_mut(),
            env,
            info,
            kfrags,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_get_metadata() {
        let mut deps = mock_dependencies();
        setup_test_owner_process(deps.as_mut());

        let result = OwnerProcessFacade::get_metadata(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_get_kfrags() {
        let mut deps = mock_dependencies();
        setup_test_owner_process(deps.as_mut());

        let result = OwnerProcessFacade::get_kfrags(deps.as_ref(), None);
        assert!(result.is_ok());

        let result_with_holder = OwnerProcessFacade::get_kfrags(
            deps.as_ref(),
            Some("holder_1".to_string()),
        );
        assert!(result_with_holder.is_ok());
    }

    #[test]
    fn test_facade_get_holder_assignments() {
        let mut deps = mock_dependencies();
        setup_test_owner_process(deps.as_mut());

        let result = OwnerProcessFacade::get_holder_assignments(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_get_distribution_stats() {
        let mut deps = mock_dependencies();
        setup_test_owner_process(deps.as_mut());

        let result = OwnerProcessFacade::get_distribution_stats(deps.as_ref());
        assert!(result.is_ok());
    }
}