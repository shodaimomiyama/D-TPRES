/// Requester-Process専用ストレージ操作
///
/// このモジュールは D-TPRES Requester-Process で使用される
/// すべてのストレージ操作を提供します。
///
/// ## 主要機能
///
/// ### 復元セッション管理
/// - セッションの作成、取得、更新、削除
/// - セッション状態の追跡と管理
/// - タイムアウト処理と期限切れセッションのクリーンアップ
///
/// ### cFrag収集管理
/// - cFragコレクションの作成と管理
/// - cFragの追加と検証
/// - 閾値達成状況の監視
///
/// ### 統計・監視機能
/// - 収集統計の計算と追跡
/// - パフォーマンス指標の管理
/// - 成功率とレスポンス時間の記録

use cosmwasm_std::{Deps, DepsMut, Env, StdResult, StdError, Timestamp};
use serde::{Deserialize, Serialize};
use crate::state::{
    CFragCollection, RecoverySession, ThresholdInfo, RequesterMetadata,
    CollectionStatus, ProcessRole, CollectedCFrag,
    CFRAG_COLLECTION, RECOVERY_SESSIONS, THRESHOLD_TRACKER, REQUESTER_METADATA,
};

/// Requester-Process の状態を表現する構造体
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequesterProcessState {
    pub metadata: RequesterMetadata,
    pub active_sessions: Vec<String>,
    pub threshold_info: ThresholdInfo,
    pub total_completed_sessions: u32,
    pub total_failed_sessions: u32,
    pub last_activity: u64,
}

/// ストレージ操作の結果型
pub type RequesterStorageResult<T> = Result<T, RequesterStorageError>;

/// Requester-Process ストレージエラー
#[derive(thiserror::Error, Debug)]
pub enum RequesterStorageError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Collection not found: {session_id}")]
    CollectionNotFound { session_id: String },

    #[error("Invalid session state: {reason}")]
    InvalidSessionState { reason: String },

    #[error("Threshold already met for session: {session_id}")]
    ThresholdAlreadyMet { session_id: String },

    #[error("Duplicate cFrag submission: {cfrag_id}")]
    DuplicateCFrag { cfrag_id: String },

    #[error("Invalid cFrag data: {reason}")]
    InvalidCFragData { reason: String },

    #[error("Storage operation failed: {reason}")]
    StorageFailure { reason: String },

    #[error("Unauthorized access - not a Requester process")]
    Unauthorized,
}

impl From<StdError> for RequesterStorageError {
    fn from(err: StdError) -> Self {
        RequesterStorageError::StorageFailure {
            reason: err.to_string(),
        }
    }
}

/// Requester-Process 状態の読み込み
///
/// ストレージから Requester-Process の完全な状態を読み込みます。
pub fn load_requester_state(deps: Deps, env: Env) -> RequesterStorageResult<RequesterProcessState> {
    let metadata = REQUESTER_METADATA.load(deps.storage)?;

    let threshold_info = THRESHOLD_TRACKER.load(deps.storage)
        .unwrap_or_else(|_| ThresholdInfo {
            required_threshold: 0,
            current_collected: 0,
            last_updated: env.block.time.seconds(),
        });

    let active_sessions = get_active_sessions_list(deps)?;

    Ok(RequesterProcessState {
        metadata: metadata.clone(),
        active_sessions,
        threshold_info,
        total_completed_sessions: calculate_completed_sessions(deps)?,
        total_failed_sessions: calculate_failed_sessions(deps)?,
        last_activity: env.block.time.seconds(),
    })
}

/// Requester-Process 状態の保存
///
/// 更新された状態をストレージに保存します。
pub fn save_requester_state(
    deps: DepsMut,
    env: Env,
    state: &RequesterProcessState,
) -> RequesterStorageResult<()> {
    // メタデータの更新
    let mut updated_metadata = state.metadata.clone();
    updated_metadata.active_sessions = state.active_sessions.clone();
    REQUESTER_METADATA.save(deps.storage, &updated_metadata)?;

    // 閾値情報の更新
    let mut updated_threshold = state.threshold_info.clone();
    updated_threshold.last_updated = env.block.time.seconds();
    THRESHOLD_TRACKER.save(deps.storage, &updated_threshold)?;

    Ok(())
}

/// 復元セッションの作成
///
/// 新しい復元セッションを作成し、ストレージに保存します。
pub fn create_recovery_session(
    deps: DepsMut,
    env: Env,
    session_id: String,
    capsule_data: Vec<u8>,
) -> RequesterStorageResult<RecoverySession> {
    // セッションIDの重複チェック
    if RECOVERY_SESSIONS.has(deps.storage, session_id.clone()) {
        return Err(RequesterStorageError::InvalidSessionState {
            reason: format!("Session {} already exists", session_id),
        });
    }

    let session = RecoverySession {
        session_id: session_id.clone(),
        capsule_data,
        collected_cfrags: vec![],
        threshold_met: false,
        recovery_completed: false,
    };

    RECOVERY_SESSIONS.save(deps.storage, session_id.clone(), &session)?;

    // アクティブセッションリストの更新
    update_active_sessions(deps, env, &session_id, true)?;

    Ok(session)
}

/// 復元セッションの取得
pub fn get_recovery_session(
    deps: Deps,
    session_id: &str,
) -> RequesterStorageResult<RecoverySession> {
    RECOVERY_SESSIONS.load(deps.storage, session_id.to_string())
        .map_err(|_| RequesterStorageError::SessionNotFound {
            session_id: session_id.to_string(),
        })
}

/// 復元セッションの更新
pub fn update_recovery_session(
    deps: DepsMut,
    session_id: &str,
    session: &RecoverySession,
) -> RequesterStorageResult<()> {
    if !RECOVERY_SESSIONS.has(deps.storage, session_id.to_string()) {
        return Err(RequesterStorageError::SessionNotFound {
            session_id: session_id.to_string(),
        });
    }

    RECOVERY_SESSIONS.save(deps.storage, session_id.to_string(), session)?;
    Ok(())
}

/// 復元セッションの削除
pub fn delete_recovery_session(
    deps: DepsMut,
    env: Env,
    session_id: &str,
) -> RequesterStorageResult<()> {
    if !RECOVERY_SESSIONS.has(deps.storage, session_id.to_string()) {
        return Err(RequesterStorageError::SessionNotFound {
            session_id: session_id.to_string(),
        });
    }

    RECOVERY_SESSIONS.remove(deps.storage, session_id.to_string());

    // cFragコレクションも削除
    if CFRAG_COLLECTION.has(deps.storage, session_id.to_string()) {
        CFRAG_COLLECTION.remove(deps.storage, session_id.to_string());
    }

    // アクティブセッションリストから削除
    update_active_sessions(deps, env, session_id, false)?;

    Ok(())
}

/// cFragコレクションの作成
pub fn create_cfrag_collection(
    deps: DepsMut,
    env: Env,
    session_id: String,
    threshold: u32,
) -> RequesterStorageResult<CFragCollection> {
    // コレクションIDの重複チェック
    if CFRAG_COLLECTION.has(deps.storage, session_id.clone()) {
        return Err(RequesterStorageError::InvalidSessionState {
            reason: format!("Collection for session {} already exists", session_id),
        });
    }

    let collection = CFragCollection {
        session_id: session_id.clone(),
        collected_cfrags: vec![],
        target_threshold: threshold,
        collection_started: env.block.time.seconds(),
        status: CollectionStatus::InProgress,
    };

    CFRAG_COLLECTION.save(deps.storage, session_id, &collection)?;

    // 閾値情報の更新
    update_threshold_info(deps, env, threshold, 0)?;

    Ok(collection)
}

/// cFragコレクションの取得
pub fn get_cfrag_collection(
    deps: Deps,
    session_id: &str,
) -> RequesterStorageResult<CFragCollection> {
    CFRAG_COLLECTION.load(deps.storage, session_id.to_string())
        .map_err(|_| RequesterStorageError::CollectionNotFound {
            session_id: session_id.to_string(),
        })
}

/// cFragをコレクションに追加
pub fn add_cfrag_to_collection(
    deps: DepsMut,
    env: Env,
    session_id: &str,
    cfrag: CollectedCFrag,
) -> RequesterStorageResult<CFragCollection> {
    let mut collection = get_cfrag_collection(deps.as_ref(), session_id)?;

    // 重複チェック
    if collection.collected_cfrags.iter().any(|c| c.cfrag_id == cfrag.cfrag_id) {
        return Err(RequesterStorageError::DuplicateCFrag {
            cfrag_id: cfrag.cfrag_id,
        });
    }

    // 閾値に達している場合は新規追加を拒否
    if collection.status == CollectionStatus::ThresholdMet ||
       collection.status == CollectionStatus::Completed {
        return Err(RequesterStorageError::ThresholdAlreadyMet {
            session_id: session_id.to_string(),
        });
    }

    collection.collected_cfrags.push(cfrag);

    // 閾値チェック
    if collection.has_reached_threshold() {
        collection.status = CollectionStatus::ThresholdMet;
    }

    CFRAG_COLLECTION.save(deps.storage, session_id.to_string(), &collection)?;

    // 閾値情報の更新
    update_threshold_info(
        deps,
        env,
        collection.target_threshold,
        collection.collected_cfrags.len() as u32,
    )?;

    Ok(collection)
}

/// 閾値達成状況の確認
pub fn check_threshold_status(
    deps: Deps,
    session_id: &str,
) -> RequesterStorageResult<bool> {
    let collection = get_cfrag_collection(deps, session_id)?;
    Ok(collection.has_reached_threshold())
}

/// 収集統計の取得
pub fn get_collection_statistics(
    deps: Deps,
    env: Env,
) -> RequesterStorageResult<CollectionStatistics> {
    let total_sessions = calculate_total_sessions(deps)?;
    let completed_sessions = calculate_completed_sessions(deps)?;
    let failed_sessions = calculate_failed_sessions(deps)?;
    let active_sessions = get_active_sessions_list(deps)?.len() as u32;

    let success_rate = if total_sessions > 0 {
        (completed_sessions as f64 / total_sessions as f64) * 100.0
    } else {
        0.0
    };

    Ok(CollectionStatistics {
        total_sessions,
        completed_sessions,
        failed_sessions,
        active_sessions,
        success_rate,
        average_collection_time: calculate_average_collection_time(deps)?,
        last_updated: env.block.time.seconds(),
    })
}

/// 期限切れセッションのクリーンアップ
pub fn cleanup_expired_sessions(
    mut deps: DepsMut,
    env: Env,
    expiry_seconds: u64,
) -> RequesterStorageResult<u32> {
    let current_time = env.block.time.seconds();
    let mut cleaned_count = 0;

    // すべてのセッションを確認
    let all_sessions: Vec<String> = RECOVERY_SESSIONS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(key, _)| key))
        .collect::<StdResult<Vec<_>>>()?;

    for session_id in all_sessions {
        if let Ok(collection) = get_cfrag_collection(deps.as_ref(), &session_id) {
            let session_age = current_time - collection.collection_started;

            if session_age > expiry_seconds &&
               collection.status != CollectionStatus::Completed {
                delete_recovery_session(deps.branch(), env.clone(), &session_id)?;
                cleaned_count += 1;
            }
        }
    }

    Ok(cleaned_count)
}

/// プロセス役割の検証
pub fn verify_requester_role(deps: Deps) -> RequesterStorageResult<()> {
    let metadata = REQUESTER_METADATA.load(deps.storage)?;

    if metadata.process_role != ProcessRole::Requester {
        return Err(RequesterStorageError::Unauthorized);
    }

    Ok(())
}

/// Requester統計情報の取得
pub fn get_requester_statistics(
    deps: Deps,
    current_time: Timestamp,
) -> RequesterStorageResult<RequesterStatistics> {
    let metadata = REQUESTER_METADATA.load(deps.storage)?;
    let threshold_info = THRESHOLD_TRACKER.load(deps.storage)
        .unwrap_or_else(|_| ThresholdInfo {
            required_threshold: 0,
            current_collected: 0,
            last_updated: current_time.seconds(),
        });

    let active_sessions = get_active_sessions_list(deps)?;
    let collection_stats = get_collection_statistics(deps,
        Env { block: cosmwasm_std::BlockInfo {
            height: 0,
            time: current_time,
            chain_id: "test".to_string(),
        }, transaction: None, contract: cosmwasm_std::ContractInfo {
            address: cosmwasm_std::Addr::unchecked("test"),
        }
    })?;

    Ok(RequesterStatistics {
        requester_id: metadata.requester_id,
        total_active_sessions: active_sessions.len() as u32,
        current_threshold_progress: if threshold_info.required_threshold > 0 {
            (threshold_info.current_collected as f64 / threshold_info.required_threshold as f64) * 100.0
        } else {
            0.0
        },
        collection_success_rate: collection_stats.success_rate,
        average_session_duration: collection_stats.average_collection_time,
        last_activity: threshold_info.last_updated,
    })
}

/// 閾値情報の更新
pub fn update_threshold_info(
    deps: DepsMut,
    env: Env,
    required_threshold: u32,
    current_collected: u32,
) -> RequesterStorageResult<()> {
    let threshold_info = ThresholdInfo {
        required_threshold,
        current_collected,
        last_updated: env.block.time.seconds(),
    };

    THRESHOLD_TRACKER.save(deps.storage, &threshold_info)?;
    Ok(())
}

/// アクティブセッション一覧の取得
pub fn get_active_sessions(deps: Deps) -> RequesterStorageResult<Vec<String>> {
    get_active_sessions_list(deps)
}

/// セッション完了のマーク
pub fn mark_session_completed(
    deps: DepsMut,
    env: Env,
    session_id: &str,
) -> RequesterStorageResult<()> {
    // コレクションの状態を完了に更新
    let mut collection = get_cfrag_collection(deps.as_ref(), session_id)?;
    collection.status = CollectionStatus::Completed;
    CFRAG_COLLECTION.save(deps.storage, session_id.to_string(), &collection)?;

    // 復元セッションの状態を完了に更新
    let mut session = get_recovery_session(deps.as_ref(), session_id)?;
    session.recovery_completed = true;
    RECOVERY_SESSIONS.save(deps.storage, session_id.to_string(), &session)?;

    // アクティブセッションリストから削除
    update_active_sessions(deps, env, session_id, false)?;

    Ok(())
}

/// cFrag送信の検証
pub fn validate_cfrag_submission(
    cfrag_data: &[u8],
    signature: &[u8],
    holder_id: &str,
) -> RequesterStorageResult<()> {
    if cfrag_data.is_empty() {
        return Err(RequesterStorageError::InvalidCFragData {
            reason: "cFrag data cannot be empty".to_string(),
        });
    }

    if signature.is_empty() {
        return Err(RequesterStorageError::InvalidCFragData {
            reason: "Signature cannot be empty".to_string(),
        });
    }

    if holder_id.is_empty() {
        return Err(RequesterStorageError::InvalidCFragData {
            reason: "Holder ID cannot be empty".to_string(),
        });
    }

    // TODO: 実際の署名検証ロジックを実装
    // verify_signature(cfrag_data, signature, holder_id)?;

    Ok(())
}

// プライベートヘルパー関数

fn get_active_sessions_list(deps: Deps) -> RequesterStorageResult<Vec<String>> {
    let metadata = REQUESTER_METADATA.load(deps.storage)?;
    Ok(metadata.active_sessions)
}

fn update_active_sessions(
    deps: DepsMut,
    _env: Env,
    session_id: &str,
    add: bool,
) -> RequesterStorageResult<()> {
    let mut metadata = REQUESTER_METADATA.load(deps.storage)?;

    if add {
        if !metadata.active_sessions.contains(&session_id.to_string()) {
            metadata.active_sessions.push(session_id.to_string());
        }
    } else {
        metadata.active_sessions.retain(|id| id != session_id);
    }

    REQUESTER_METADATA.save(deps.storage, &metadata)?;
    Ok(())
}

fn calculate_total_sessions(deps: Deps) -> RequesterStorageResult<u32> {
    let sessions: Vec<_> = RECOVERY_SESSIONS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(sessions.len() as u32)
}

fn calculate_completed_sessions(deps: Deps) -> RequesterStorageResult<u32> {
    let completed_count = CFRAG_COLLECTION
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            if let Ok((_, collection)) = item {
                if collection.status == CollectionStatus::Completed {
                    Some(())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .count();

    Ok(completed_count as u32)
}

fn calculate_failed_sessions(deps: Deps) -> RequesterStorageResult<u32> {
    let failed_count = CFRAG_COLLECTION
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            if let Ok((_, collection)) = item {
                if collection.status == CollectionStatus::Failed {
                    Some(())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .count();

    Ok(failed_count as u32)
}

fn calculate_average_collection_time(deps: Deps) -> RequesterStorageResult<f64> {
    let collections: Vec<_> = CFRAG_COLLECTION
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            if let Ok((_, collection)) = item {
                if collection.status == CollectionStatus::Completed {
                    Some(collection)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    if collections.is_empty() {
        return Ok(0.0);
    }

    // 実際の実装では完了時刻を記録する必要がある
    // 現在は簡略化してデフォルト値を返す
    Ok(300.0) // 5分の平均時間（秒）
}

// 統計情報の構造体定義

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CollectionStatistics {
    pub total_sessions: u32,
    pub completed_sessions: u32,
    pub failed_sessions: u32,
    pub active_sessions: u32,
    pub success_rate: f64,
    pub average_collection_time: f64,
    pub last_updated: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RequesterStatistics {
    pub requester_id: String,
    pub total_active_sessions: u32,
    pub current_threshold_progress: f64,
    pub collection_success_rate: f64,
    pub average_session_duration: f64,
    pub last_activity: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use crate::state::{RequesterMetadata, ProcessRole, CollectedCFrag};

    fn setup_test_requester_metadata(deps: DepsMut) {
        let metadata = RequesterMetadata {
            requester_id: "test_requester".to_string(),
            process_role: ProcessRole::Requester,
            active_sessions: vec![],
            initialization_time: 1000,
        };
        REQUESTER_METADATA.save(deps.storage, &metadata).unwrap();
    }

    #[test]
    fn test_load_save_requester_state() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        let state = load_requester_state(deps.as_ref(), env.clone()).unwrap();
        assert_eq!(state.metadata.requester_id, "test_requester");

        let save_result = save_requester_state(deps.as_mut(), env, &state);
        assert!(save_result.is_ok());
    }

    #[test]
    fn test_create_recovery_session() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        let capsule_data = vec![1, 2, 3, 4];
        let session = create_recovery_session(
            deps.as_mut(),
            env,
            "test_session".to_string(),
            capsule_data.clone(),
        ).unwrap();

        assert_eq!(session.session_id, "test_session");
        assert_eq!(session.capsule_data, capsule_data);
        assert!(!session.threshold_met);
        assert!(!session.recovery_completed);
    }

    #[test]
    fn test_create_cfrag_collection() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        let collection = create_cfrag_collection(
            deps.as_mut(),
            env,
            "test_session".to_string(),
            3,
        ).unwrap();

        assert_eq!(collection.session_id, "test_session");
        assert_eq!(collection.target_threshold, 3);
        assert_eq!(collection.collected_cfrags.len(), 0);
        assert_eq!(collection.status, CollectionStatus::InProgress);
    }

    #[test]
    fn test_add_cfrag_to_collection() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        // コレクション作成
        let _collection = create_cfrag_collection(
            deps.as_mut(),
            env.clone(),
            "test_session".to_string(),
            2,
        ).unwrap();

        // cFrag追加
        let cfrag = CollectedCFrag {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4],
            collected_at: env.block.time.seconds(),
            verified: true,
        };

        let updated_collection = add_cfrag_to_collection(
            deps.as_mut(),
            env,
            "test_session",
            cfrag,
        ).unwrap();

        assert_eq!(updated_collection.collected_cfrags.len(), 1);
        assert_eq!(updated_collection.status, CollectionStatus::InProgress);
    }

    #[test]
    fn test_threshold_reached() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        // 閾値2のコレクション作成
        let _collection = create_cfrag_collection(
            deps.as_mut(),
            env.clone(),
            "test_session".to_string(),
            2,
        ).unwrap();

        // 1つ目のcFrag追加
        let cfrag1 = CollectedCFrag {
            cfrag_id: "cfrag_1".to_string(),
            holder_id: "holder_1".to_string(),
            cfrag_data: vec![1, 2, 3, 4],
            collected_at: env.block.time.seconds(),
            verified: true,
        };
        add_cfrag_to_collection(deps.as_mut(), env.clone(), "test_session", cfrag1).unwrap();

        // 2つ目のcFrag追加（閾値達成）
        let cfrag2 = CollectedCFrag {
            cfrag_id: "cfrag_2".to_string(),
            holder_id: "holder_2".to_string(),
            cfrag_data: vec![5, 6, 7, 8],
            collected_at: env.block.time.seconds(),
            verified: true,
        };
        let final_collection = add_cfrag_to_collection(
            deps.as_mut(),
            env,
            "test_session",
            cfrag2,
        ).unwrap();

        assert_eq!(final_collection.collected_cfrags.len(), 2);
        assert_eq!(final_collection.status, CollectionStatus::ThresholdMet);
        assert!(final_collection.has_reached_threshold());
    }

    #[test]
    fn test_verify_requester_role() {
        let mut deps = mock_dependencies();

        setup_test_requester_metadata(deps.as_mut());

        let result = verify_requester_role(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_collection_statistics() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        let stats = get_collection_statistics(deps.as_ref(), env).unwrap();
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.active_sessions, 0);
    }

    #[test]
    fn test_validate_cfrag_submission() {
        let cfrag_data = vec![1, 2, 3, 4];
        let signature = vec![5, 6, 7, 8];
        let holder_id = "holder_1";

        let result = validate_cfrag_submission(&cfrag_data, &signature, holder_id);
        assert!(result.is_ok());

        // 空のデータでテスト
        let empty_result = validate_cfrag_submission(&[], &signature, holder_id);
        assert!(empty_result.is_err());
    }

    #[test]
    fn test_mark_session_completed() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_requester_metadata(deps.as_mut());

        // セッションとコレクション作成
        create_recovery_session(
            deps.as_mut(),
            env.clone(),
            "test_session".to_string(),
            vec![1, 2, 3, 4],
        ).unwrap();

        create_cfrag_collection(
            deps.as_mut(),
            env.clone(),
            "test_session".to_string(),
            2,
        ).unwrap();

        // セッション完了
        let result = mark_session_completed(deps.as_mut(), env, "test_session");
        assert!(result.is_ok());

        // 状態確認
        let collection = get_cfrag_collection(deps.as_ref(), "test_session").unwrap();
        assert_eq!(collection.status, CollectionStatus::Completed);

        let session = get_recovery_session(deps.as_ref(), "test_session").unwrap();
        assert!(session.recovery_completed);
    }
}