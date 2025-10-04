/// Requester-Process メッセージハンドラ
///
/// このモジュールは D-TPRES Requester-Process で受信する
/// すべてのメッセージを処理するハンドラを提供します。
///
/// ## 主要機能
///
/// ### セッション管理ハンドラ
/// - cFrag収集セッションの開始と管理
/// - 復元セッションの初期化と追跡
/// - セッション状態の更新と完了処理
///
/// ### cFrag収集ハンドラ
/// - Holder-ProcessからのcFrag受信と検証
/// - 閾値達成の監視と自動処理移行
/// - 収集統計の更新と追跡
///
/// ### クエリハンドラ
/// - セッション状況の照会
/// - 統計情報の提供
/// - 閾値進捗の確認

use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, StdError,
};
use serde::{Deserialize, Serialize};

use crate::contract::ContractError;
use crate::msg::{
    CFragSubmission, RequesterMetadataResponse, CFragCollectionResponse,
    RecoverySessionResponse, ThresholdInfoResponse,
};
use crate::state::{
    CollectedCFrag, ProcessRole, REQUESTER_METADATA,
};
use crate::requester::storage::{
    RequesterStorageError, RequesterStorageResult,
    load_requester_state, save_requester_state,
    create_recovery_session, get_recovery_session, update_recovery_session,
    create_cfrag_collection, get_cfrag_collection, add_cfrag_to_collection,
    check_threshold_status, get_collection_statistics, verify_requester_role,
    get_requester_statistics, mark_session_completed, validate_cfrag_submission,
    CollectionStatistics, RequesterStatistics,
};

/// cFrag収集結果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CFragCollectionResult {
    pub session_id: String,
    pub collected_count: u32,
    pub threshold_met: bool,
    pub status: String,
}

/// 復元開始結果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecoveryInitiationResult {
    pub session_id: String,
    pub recovery_started: bool,
    pub message: String,
}

/// セッション管理結果
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionManagementResult {
    pub session_id: String,
    pub operation: String,
    pub success: bool,
    pub details: String,
}

/// cFrag収集セッションの開始
///
/// R-Browserからの復元要求を受信し、新しい収集セッションを作成します。
///
/// ## 処理フロー
/// 1. プロセス役割の検証（Requester のみ許可）
/// 2. セッションパラメータの検証
/// 3. 復元セッションの作成
/// 4. cFragコレクションの初期化
/// 5. 閾値情報の設定
pub fn handle_start_cfrag_collection(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    session_id: String,
    threshold: u32,
) -> Result<Response, ContractError> {
    // 役割検証
    verify_requester_role(deps.as_ref())?;

    // パラメータ検証
    if session_id.is_empty() {
        return Err(ContractError::InvalidInput {
            msg: "Session ID cannot be empty".to_string(),
        });
    }

    if threshold == 0 {
        return Err(ContractError::InvalidInput {
            msg: "Threshold must be greater than 0".to_string(),
        });
    }

    // 空のCapsuleデータで復元セッション作成（後で更新される）
    let _session = create_recovery_session(
        deps.branch(),
        env.clone(),
        session_id.clone(),
        vec![], // 初期は空、後で復元時に設定
    )?;

    // cFragコレクションの作成
    let collection = create_cfrag_collection(
        deps,
        env,
        session_id.clone(),
        threshold,
    )?;

    let result = CFragCollectionResult {
        session_id: session_id.clone(),
        collected_count: 0,
        threshold_met: false,
        status: collection.status.to_string(),
    };

    Ok(Response::new()
        .add_attribute("action", "start_cfrag_collection")
        .add_attribute("session_id", &session_id)
        .add_attribute("threshold", threshold.to_string())
        .add_attribute("sender", info.sender.as_str())
        .set_data(to_binary(&result)?))
}

/// cFragの収集と蓄積
///
/// Holder-Processから送信されたcFragを検証・保存し、閾値達成を監視します。
///
/// ## 処理フロー
/// 1. セッションの存在確認
/// 2. cFragデータの検証（署名、形式など）
/// 3. コレクションへの追加
/// 4. 閾値チェックと状態更新
/// 5. 必要に応じて自動復元処理の開始
pub fn handle_collect_cfrag(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    session_id: String,
    cfrag_data: CFragSubmission,
) -> Result<Response, ContractError> {
    // 基本的な検証
    if session_id.is_empty() {
        return Err(ContractError::InvalidInput {
            msg: "Session ID cannot be empty".to_string(),
        });
    }

    // cFragデータの検証
    validate_cfrag_submission(
        &cfrag_data.cfrag_data,
        &cfrag_data.signature,
        &cfrag_data.holder_id,
    )?;

    // セッションの存在確認
    let _session = get_recovery_session(deps.as_ref(), &session_id)?;

    // CollectedCFragの作成
    let collected_cfrag = CollectedCFrag {
        cfrag_id: cfrag_data.cfrag_id.clone(),
        holder_id: cfrag_data.holder_id.clone(),
        cfrag_data: cfrag_data.cfrag_data,
        collected_at: env.block.time.seconds(),
        verified: true, // TODO: 実際の署名検証を実装
    };

    // コレクションに追加
    let updated_collection = add_cfrag_to_collection(
        deps,
        env,
        &session_id,
        collected_cfrag,
    )?;

    let threshold_met = updated_collection.has_reached_threshold();

    let result = CFragCollectionResult {
        session_id: session_id.clone(),
        collected_count: updated_collection.collected_cfrags.len() as u32,
        threshold_met,
        status: updated_collection.status.to_string(),
    };

    let mut response = Response::new()
        .add_attribute("action", "collect_cfrag")
        .add_attribute("session_id", &session_id)
        .add_attribute("cfrag_id", &cfrag_data.cfrag_id)
        .add_attribute("holder_id", &cfrag_data.holder_id)
        .add_attribute("collected_count", result.collected_count.to_string())
        .add_attribute("threshold_met", threshold_met.to_string())
        .add_attribute("sender", info.sender.as_str())
        .set_data(to_binary(&result)?);

    // 閾値達成時の追加メッセージ
    if threshold_met {
        response = response.add_attribute("status", "threshold_reached");
    }

    Ok(response)
}

/// 復元処理の開始
///
/// 閾値に達したセッションで復元処理を開始し、R-Browserへの配信準備を行います。
///
/// ## 処理フロー
/// 1. セッションの状態確認
/// 2. 閾値達成の確認
/// 3. Capsuleデータの設定
/// 4. セッション状態の完了への更新
/// 5. R-Browser向けデータの準備
pub fn handle_initiate_recovery(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    session_id: String,
    capsule_data: Vec<u8>,
) -> Result<Response, ContractError> {
    // 役割検証
    verify_requester_role(deps.as_ref())?;

    // パラメータ検証
    if session_id.is_empty() {
        return Err(ContractError::InvalidInput {
            msg: "Session ID cannot be empty".to_string(),
        });
    }

    if capsule_data.is_empty() {
        return Err(ContractError::InvalidInput {
            msg: "Capsule data cannot be empty".to_string(),
        });
    }

    // セッションの取得と状態確認
    let mut session = get_recovery_session(deps.as_ref(), &session_id)?;

    // 閾値達成の確認
    let threshold_met = check_threshold_status(deps.as_ref(), &session_id)?;
    if !threshold_met {
        return Err(ContractError::InvalidState {
            msg: format!("Threshold not met for session {}", session_id),
        });
    }

    // Capsuleデータの設定
    session.capsule_data = capsule_data;
    session.threshold_met = true;
    session.recovery_completed = true;

    // セッションの更新
    update_recovery_session(deps.branch(), &session_id, &session)?;

    // セッション完了のマーク
    mark_session_completed(deps, env, &session_id)?;

    let result = RecoveryInitiationResult {
        session_id: session_id.clone(),
        recovery_started: true,
        message: "Recovery process initiated successfully".to_string(),
    };

    Ok(Response::new()
        .add_attribute("action", "initiate_recovery")
        .add_attribute("session_id", &session_id)
        .add_attribute("recovery_started", "true")
        .add_attribute("sender", info.sender.as_str())
        .set_data(to_binary(&result)?))
}

/// Requesterメタデータの照会
pub fn query_requester_metadata(deps: Deps) -> StdResult<Binary> {
    let metadata = REQUESTER_METADATA.load(deps.storage)?;
    let response = RequesterMetadataResponse { metadata };
    to_binary(&response)
}

/// cFrag収集状況の照会
pub fn query_cfrag_collection(deps: Deps, session_id: String) -> StdResult<Binary> {
    let collection = get_cfrag_collection(deps, &session_id)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    let response = CFragCollectionResponse { collection };
    to_binary(&response)
}

/// 復元セッション情報の照会
pub fn query_recovery_session(deps: Deps, session_id: String) -> StdResult<Binary> {
    let session = get_recovery_session(deps, &session_id)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    let response = RecoverySessionResponse { session };
    to_binary(&response)
}

/// 閾値情報の照会
pub fn query_threshold_info(deps: Deps) -> StdResult<Binary> {
    let state = load_requester_state(deps, cosmwasm_std::Env {
        block: cosmwasm_std::BlockInfo {
            height: 0,
            time: cosmwasm_std::Timestamp::from_seconds(0),
            chain_id: "test".to_string(),
        },
        transaction: None,
        contract: cosmwasm_std::ContractInfo {
            address: cosmwasm_std::Addr::unchecked("test"),
        },
    }).map_err(|e| StdError::generic_err(e.to_string()))?;

    let response = ThresholdInfoResponse {
        threshold_info: state.threshold_info,
    };
    to_binary(&response)
}

/// 収集統計情報の照会
pub fn query_collection_statistics(deps: Deps) -> StdResult<Binary> {
    let env = cosmwasm_std::Env {
        block: cosmwasm_std::BlockInfo {
            height: 0,
            time: cosmwasm_std::Timestamp::from_seconds(0),
            chain_id: "test".to_string(),
        },
        transaction: None,
        contract: cosmwasm_std::ContractInfo {
            address: cosmwasm_std::Addr::unchecked("test"),
        },
    };

    let stats = get_collection_statistics(deps, env)
        .map_err(|e| StdError::generic_err(e.to_string()))?;

    to_binary(&stats)
}

// RequesterStorageErrorをContractErrorに変換するためのFrom実装
impl From<RequesterStorageError> for ContractError {
    fn from(err: RequesterStorageError) -> Self {
        match err {
            RequesterStorageError::SessionNotFound { session_id } => {
                ContractError::NotFound {
                    msg: format!("Session not found: {}", session_id),
                }
            }
            RequesterStorageError::CollectionNotFound { session_id } => {
                ContractError::NotFound {
                    msg: format!("Collection not found: {}", session_id),
                }
            }
            RequesterStorageError::InvalidSessionState { reason } => {
                ContractError::InvalidState { msg: reason }
            }
            RequesterStorageError::ThresholdAlreadyMet { session_id } => {
                ContractError::InvalidState {
                    msg: format!("Threshold already met for session: {}", session_id),
                }
            }
            RequesterStorageError::DuplicateCFrag { cfrag_id } => {
                ContractError::InvalidInput {
                    msg: format!("Duplicate cFrag submission: {}", cfrag_id),
                }
            }
            RequesterStorageError::InvalidCFragData { reason } => {
                ContractError::InvalidInput { msg: reason }
            }
            RequesterStorageError::StorageFailure { reason } => {
                ContractError::StorageError { msg: reason }
            }
            RequesterStorageError::Unauthorized => {
                ContractError::Unauthorized {
                    msg: "Only Requester processes can perform this operation".to_string(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::state::{RequesterMetadata, ProcessRole, REQUESTER_METADATA};

    fn setup_test_requester_process(deps: DepsMut) {
        let metadata = RequesterMetadata {
            requester_id: "test_requester".to_string(),
            process_role: ProcessRole::Requester,
            active_sessions: vec![],
            initialization_time: 1000,
        };

        REQUESTER_METADATA.save(deps.storage, &metadata).unwrap();
    }

    #[test]
    fn test_handle_start_cfrag_collection() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let result = handle_start_cfrag_collection(
            deps.as_mut(),
            env,
            info,
            "test_session".to_string(),
            3,
        );

        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.attributes.iter().any(|attr|
            attr.key == "action" && attr.value == "start_cfrag_collection"
        ));
    }

    #[test]
    fn test_handle_start_cfrag_collection_empty_session_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let result = handle_start_cfrag_collection(
            deps.as_mut(),
            env,
            info,
            "".to_string(), // 空のセッションID
            3,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::InvalidInput { msg } => {
                assert!(msg.contains("Session ID cannot be empty"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_handle_start_cfrag_collection_zero_threshold() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let result = handle_start_cfrag_collection(
            deps.as_mut(),
            env,
            info,
            "test_session".to_string(),
            0, // 閾値ゼロ
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::InvalidInput { msg } => {
                assert!(msg.contains("Threshold must be greater than 0"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_handle_collect_cfrag_session_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("holder", &[]);

        setup_test_requester_process(deps.as_mut());

        let cfrag_data = CFragSubmission {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4, 5, 6, 7, 8],
            signature: vec![1; 64],
        };

        let result = handle_collect_cfrag(
            deps.as_mut(),
            env,
            info,
            "nonexistent_session".to_string(),
            cfrag_data,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::NotFound { msg } => {
                assert!(msg.contains("Session not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_handle_collect_cfrag_empty_session_id() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("holder", &[]);

        setup_test_requester_process(deps.as_mut());

        let cfrag_data = CFragSubmission {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4],
            signature: vec![1; 64],
        };

        let result = handle_collect_cfrag(
            deps.as_mut(),
            env,
            info,
            "".to_string(), // 空のセッションID
            cfrag_data,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::InvalidInput { msg } => {
                assert!(msg.contains("Session ID cannot be empty"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_handle_initiate_recovery_session_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let capsule_data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let result = handle_initiate_recovery(
            deps.as_mut(),
            env,
            info,
            "nonexistent_session".to_string(),
            capsule_data,
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::NotFound { msg } => {
                assert!(msg.contains("Session not found"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_handle_initiate_recovery_empty_capsule() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("requester", &[]);

        setup_test_requester_process(deps.as_mut());

        let result = handle_initiate_recovery(
            deps.as_mut(),
            env,
            info,
            "test_session".to_string(),
            vec![], // 空のCapsuleデータ
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            ContractError::InvalidInput { msg } => {
                assert!(msg.contains("Capsule data cannot be empty"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_query_requester_metadata() {
        let mut deps = mock_dependencies();

        setup_test_requester_process(deps.as_mut());

        let result = query_requester_metadata(deps.as_ref());
        assert!(result.is_ok());

        let binary = result.unwrap();
        let response: RequesterMetadataResponse = cosmwasm_std::from_binary(&binary).unwrap();
        assert_eq!(response.metadata.requester_id, "test_requester");
    }

    #[test]
    fn test_query_cfrag_collection_not_found() {
        let mut deps = mock_dependencies();

        setup_test_requester_process(deps.as_mut());

        let result = query_cfrag_collection(deps.as_ref(), "nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_query_recovery_session_not_found() {
        let mut deps = mock_dependencies();

        setup_test_requester_process(deps.as_mut());

        let result = query_recovery_session(deps.as_ref(), "nonexistent".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_query_threshold_info() {
        let mut deps = mock_dependencies();

        setup_test_requester_process(deps.as_mut());

        let result = query_threshold_info(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_collection_statistics() {
        let mut deps = mock_dependencies();

        setup_test_requester_process(deps.as_mut());

        let result = query_collection_statistics(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_full_session_flow() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_process(deps.as_mut());

        // 1. セッション開始
        let start_info = mock_info("requester", &[]);
        let start_result = handle_start_cfrag_collection(
            deps.as_mut(),
            env.clone(),
            start_info,
            "full_test_session".to_string(),
            2, // 閾値2
        );
        assert!(start_result.is_ok());

        // 2. 1つ目のcFrag収集
        let collect_info1 = mock_info("holder1", &[]);
        let cfrag1 = CFragSubmission {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4],
            signature: vec![1; 64],
        };
        let collect_result1 = handle_collect_cfrag(
            deps.as_mut(),
            env.clone(),
            collect_info1,
            "full_test_session".to_string(),
            cfrag1,
        );
        assert!(collect_result1.is_ok());

        // 3. 2つ目のcFrag収集（閾値達成）
        let collect_info2 = mock_info("holder2", &[]);
        let cfrag2 = CFragSubmission {
            cfrag_id: "cfrag_2".to_string(),
            holder_id: "holder_2".to_string(),
            cfrag_data: vec![5, 6, 7, 8],
            signature: vec![2; 64],
        };
        let collect_result2 = handle_collect_cfrag(
            deps.as_mut(),
            env.clone(),
            collect_info2,
            "full_test_session".to_string(),
            cfrag2,
        );
        assert!(collect_result2.is_ok());

        // 閾値達成の確認
        let response2 = collect_result2.unwrap();
        assert!(response2.attributes.iter().any(|attr|
            attr.key == "threshold_met" && attr.value == "true"
        ));

        // 4. 復元開始
        let recovery_info = mock_info("requester", &[]);
        let capsule_data = vec![9, 10, 11, 12, 13, 14, 15, 16];
        let recovery_result = handle_initiate_recovery(
            deps.as_mut(),
            env,
            recovery_info,
            "full_test_session".to_string(),
            capsule_data,
        );
        assert!(recovery_result.is_ok());

        let recovery_response = recovery_result.unwrap();
        assert!(recovery_response.attributes.iter().any(|attr|
            attr.key == "recovery_started" && attr.value == "true"
        ));
    }
}