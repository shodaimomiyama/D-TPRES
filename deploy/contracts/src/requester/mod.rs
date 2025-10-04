/// Requester-Process モジュール
///
/// D-TPRES Requester-Process の機能を提供するモジュールです。
///
/// ## 責務
///
/// Requester-Process は PRD に定義された以下の責務を担います：
///
/// ### PHASE 3-1 (復元セッション開始)
/// - R-Browser からの復元要求受信・セッション作成
/// - k-of-n 閾値管理および収集状況の追跡
/// - 復元セッションの状態管理とタイムアウト処理
///
/// ### PHASE 3-2 (cFrag収集)
/// - 複数のHolder-ProcessからcFragを順次収集
/// - cFragの署名検証と真正性確認
/// - 閾値達成の監視と進捗管理
///
/// ### PHASE 3-3 (データ送信準備)
/// - CapsuleとcFragのセットをR-Browser向けに準備
/// - k個以上のcFragが集まった場合の通知処理
/// - 復元データの最終検証と配信
///
/// ## アーキテクチャ
///
/// このモジュールは以下のサブモジュールで構成されます：
///
/// - `storage`: Requester専用のストレージ操作（セッション、cFrag収集、閾値管理）
/// - `handlers`: Requester-Process のメッセージハンドラ（開始、収集、完了）
///
/// ## セキュリティ要件
///
/// - セッション管理における適切な認証・認可
/// - cFragの署名検証による真正性確認
/// - プロセス役割の厳格な検証（Requester のみ）
/// - セッション状態の安全な永続化
/// - 不完了セッションの適切なクリーンアップ
///
/// ## パフォーマンス考慮
///
/// - 複数Holderとの並行通信最適化
/// - cFrag収集の効率化とバッチ処理
/// - セッション状態の高速アクセス
/// - 統計情報の効率的な集計
/// - 長時間実行セッションのメモリ管理
///
/// ## 閾値管理機能
///
/// - **k-of-n閾値管理**: 必要なcFrag数の追跡
/// - **収集状況監視**: リアルタイムでの進捗確認
/// - **自動完了検知**: 閾値達成時の自動処理移行
/// - **タイムアウト処理**: 長時間未完了セッションの処理
///
/// ## 使用方法
///
/// ```rust
/// use crate::requester::handlers::{
///     handle_start_cfrag_collection,
///     handle_collect_cfrag,
///     handle_initiate_recovery,
/// };
/// use crate::requester::storage::{
///     load_requester_state,
///     save_requester_state,
///     create_recovery_session,
///     collect_cfrag_data,
/// };
/// ```

pub mod storage;
pub mod handlers;

// 公開API
pub use storage::{
    RequesterProcessState,
    RequesterStorageResult,
    load_requester_state,
    save_requester_state,
    create_recovery_session,
    get_recovery_session,
    update_recovery_session,
    delete_recovery_session,
    create_cfrag_collection,
    get_cfrag_collection,
    add_cfrag_to_collection,
    check_threshold_status,
    get_collection_statistics,
    cleanup_expired_sessions,
    verify_requester_role,
    get_requester_statistics,
    update_threshold_info,
    get_active_sessions,
    mark_session_completed,
    validate_cfrag_submission,
};

pub use handlers::{
    CFragCollectionResult,
    RecoveryInitiationResult,
    SessionManagementResult,
    handle_start_cfrag_collection,
    handle_collect_cfrag,
    handle_initiate_recovery,
    query_requester_metadata,
    query_cfrag_collection,
    query_recovery_session,
    query_threshold_info,
    query_collection_statistics,
};

/// Requester-Process の主要機能を統合したファサード
///
/// このファサードは Requester-Process の主要操作を簡潔な API で提供します。
/// 復元セッション管理、cFrag収集、閾値チェックなどの機能を統合します。
pub struct RequesterProcessFacade;

impl RequesterProcessFacade {
    /// cFrag収集セッションの開始
    ///
    /// この関数は以下の処理を順次実行します：
    /// 1. プロセス役割の検証（Requester のみ許可）
    /// 2. セッションパラメータの検証
    /// 3. 新しい復元セッションの作成
    /// 4. 閾値情報の初期化
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `session_id` - 作成するセッションの ID
    /// * `threshold` - 必要な cFrag の閾値
    ///
    /// # 戻り値
    ///
    /// セッション作成結果を含む Response または ContractError
    pub fn start_cfrag_collection(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        session_id: String,
        threshold: u32,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_start_cfrag_collection(deps, env, info, session_id, threshold)
    }

    /// cFrag の収集と蓄積
    ///
    /// Holder-Process から送信された cFrag を検証・保存し、閾値達成を監視します。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `session_id` - 対象セッションの ID
    /// * `cfrag_data` - 収集する cFrag データ
    ///
    /// # 戻り値
    ///
    /// 収集結果と閾値状況を含む Response または ContractError
    pub fn collect_cfrag(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        session_id: String,
        cfrag_data: crate::msg::CFragSubmission,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_collect_cfrag(deps, env, info, session_id, cfrag_data)
    }

    /// 復元処理の開始
    ///
    /// 閾値に達したセッションで復元処理を開始し、R-Browser への配信準備を行います。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `env` - 実行環境情報
    /// * `info` - メッセージ情報
    /// * `session_id` - 復元対象セッションの ID
    /// * `capsule_data` - Capsule データ
    pub fn initiate_recovery(
        deps: cosmwasm_std::DepsMut,
        env: cosmwasm_std::Env,
        info: cosmwasm_std::MessageInfo,
        session_id: String,
        capsule_data: Vec<u8>,
    ) -> Result<cosmwasm_std::Response, crate::contract::ContractError> {
        handlers::handle_initiate_recovery(deps, env, info, session_id, capsule_data)
    }

    /// Requester メタデータの取得
    ///
    /// Requester-Process の基本的なメタデータ（アクティブセッション数、統計情報など）を取得します。
    pub fn get_metadata(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_requester_metadata(deps)
    }

    /// cFrag収集状況の取得
    ///
    /// 指定されたセッションの cFrag 収集状況を取得します。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `session_id` - 確認対象のセッション ID
    pub fn get_cfrag_collection(
        deps: cosmwasm_std::Deps,
        session_id: String,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_cfrag_collection(deps, session_id)
    }

    /// 復元セッション情報の取得
    ///
    /// 指定されたセッションの詳細情報と復元状況を取得します。
    ///
    /// # 引数
    ///
    /// * `deps` - CosmWasm の依存関係
    /// * `session_id` - 確認対象のセッション ID
    pub fn get_recovery_session(
        deps: cosmwasm_std::Deps,
        session_id: String,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_recovery_session(deps, session_id)
    }

    /// 閾値情報の取得
    ///
    /// 現在の閾値設定と収集状況の統計情報を取得します。
    pub fn get_threshold_info(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_threshold_info(deps)
    }

    /// 収集統計情報の取得
    ///
    /// セッション成功率、平均収集時間、パフォーマンス指標などを取得します。
    pub fn get_collection_statistics(
        deps: cosmwasm_std::Deps,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        handlers::query_collection_statistics(deps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::state::{RequesterMetadata, ProcessRole, REQUESTER_METADATA};
    use crate::msg::CFragSubmission;

    fn setup_test_requester_process(deps: cosmwasm_std::DepsMut) {
        let metadata = RequesterMetadata {
            requester_id: "test_requester".to_string(),
            process_role: ProcessRole::Requester,
            active_sessions: vec![],
            initialization_time: 1000,
        };

        REQUESTER_METADATA.save(deps.storage, &metadata).unwrap();
    }

    #[test]
    fn test_facade_start_cfrag_collection() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::start_cfrag_collection(
            deps.as_mut(),
            env,
            info,
            "session_1".to_string(),
            3,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_collect_cfrag() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("holder", &[]);

        setup_test_requester_process(deps.as_mut());

        let cfrag_data = CFragSubmission {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            signature: vec![1; 64], // 64バイトの署名
        };

        let result = RequesterProcessFacade::collect_cfrag(
            deps.as_mut(),
            env,
            info,
            "session_1".to_string(),
            cfrag_data,
        );

        // セッションが存在しない場合はエラーになることを確認
        assert!(result.is_err());
    }

    #[test]
    fn test_facade_initiate_recovery() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("browser", &[]);

        setup_test_requester_process(deps.as_mut());

        let capsule_data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let result = RequesterProcessFacade::initiate_recovery(
            deps.as_mut(),
            env,
            info,
            "session_1".to_string(),
            capsule_data,
        );

        // セッションが存在しない場合はエラーになることを確認
        assert!(result.is_err());
    }

    #[test]
    fn test_facade_get_metadata() {
        let mut deps = mock_dependencies();
        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::get_metadata(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_get_cfrag_collection() {
        let mut deps = mock_dependencies();
        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::get_cfrag_collection(
            deps.as_ref(),
            "session_1".to_string(),
        );

        // セッションが存在しない場合はエラーが返される
        assert!(result.is_err());
    }

    #[test]
    fn test_facade_get_recovery_session() {
        let mut deps = mock_dependencies();
        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::get_recovery_session(
            deps.as_ref(),
            "session_1".to_string(),
        );

        // セッションが存在しない場合はエラーが返される
        assert!(result.is_err());
    }

    #[test]
    fn test_facade_get_threshold_info() {
        let mut deps = mock_dependencies();
        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::get_threshold_info(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_get_collection_statistics() {
        let mut deps = mock_dependencies();
        setup_test_requester_process(deps.as_mut());

        let result = RequesterProcessFacade::get_collection_statistics(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_session_lifecycle() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_process(deps.as_mut());

        // 1. セッション開始
        let start_info = mock_info("requester", &[]);
        let start_result = RequesterProcessFacade::start_cfrag_collection(
            deps.as_mut(),
            env.clone(),
            start_info,
            "test_session".to_string(),
            3,
        );
        assert!(start_result.is_ok());

        // 2. メタデータ確認
        let metadata_result = RequesterProcessFacade::get_metadata(deps.as_ref());
        assert!(metadata_result.is_ok());

        // 3. 閾値情報確認
        let threshold_result = RequesterProcessFacade::get_threshold_info(deps.as_ref());
        assert!(threshold_result.is_ok());
    }
}