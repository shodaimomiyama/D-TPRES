use cosmwasm_std::{Deps, DepsMut, StdError, StdResult, Timestamp};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::state::{
    OwnerKFragData, OwnerMetadata, OwnerConfig, HolderAssignment, AssignmentStatus,
    OWNER_KFRAGS, OWNER_METADATA, OWNER_CONFIG, HOLDER_ASSIGNMENTS,
    ProcessRole, StorageError
};

/// Owner-Process専用ストレージ操作モジュール
///
/// このモジュールは Owner-Process のみが使用する専用ストレージ操作を提供します。
/// KV_STORAGE_SPEC.md に従い、3段階パターン（状態ロード→処理→状態保存）を実装します。

/// Owner プロセスの状態管理構造体
#[derive(Debug, Clone)]
pub struct OwnerProcessState {
    pub metadata: OwnerMetadata,
    pub config: OwnerConfig,
    pub kfrags: Vec<OwnerKFragData>,
    #[allow(dead_code)]
    pub assignments: Vec<HolderAssignment>,
    pub last_updated: u64,
}

impl Zeroize for OwnerProcessState {
    fn zeroize(&mut self) {
        self.kfrags.zeroize();
        // metadata, config, assignments, last_updated はスキップ
    }
}

impl ZeroizeOnDrop for OwnerProcessState {}

/// Owner専用ストレージ操作の結果
#[derive(Debug)]
pub struct OwnerStorageResult<T> {
    pub data: T,
    pub affected_records: usize,
}

/// Owner-Process 状態の完全ロード
pub fn load_owner_state(deps: Deps, current_time: Timestamp) -> StdResult<OwnerProcessState> {
    // メタデータをロード
    let metadata = OWNER_METADATA.load(deps.storage)
        .map_err(|_| StdError::generic_err("Owner metadata not found"))?;

    // 設定をロード
    let config = OWNER_CONFIG.load(deps.storage)
        .map_err(|_| StdError::generic_err("Owner config not found"))?;

    // すべてのkFragsをロード
    let kfrags: StdResult<Vec<_>> = OWNER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    // すべてのHolder割り当てをロード
    let assignments: StdResult<Vec<_>> = HOLDER_ASSIGNMENTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, assignment)| assignment))
        .collect();

    Ok(OwnerProcessState {
        metadata,
        config,
        kfrags: kfrags?,
        assignments: assignments?,
        last_updated: current_time.seconds(),
    })
}

/// Owner-Process 状態の完全保存
pub fn save_owner_state(
    deps: DepsMut,
    state: &OwnerProcessState
) -> StdResult<OwnerStorageResult<()>> {
    let mut affected_records = 0;

    // メタデータを保存
    OWNER_METADATA.save(deps.storage, &state.metadata)?;
    affected_records += 1;

    // 設定を保存
    OWNER_CONFIG.save(deps.storage, &state.config)?;
    affected_records += 1;

    // kFragsを保存（既存のものは上書き）
    for kfrag in &state.kfrags {
        OWNER_KFRAGS.save(deps.storage, kfrag.kfrag_id.clone(), kfrag)?;
        affected_records += 1;
    }

    // Holder割り当てを保存
    for assignment in &state.assignments {
        HOLDER_ASSIGNMENTS.save(deps.storage, assignment.holder_id.clone(), assignment)?;
        affected_records += 1;
    }

    Ok(OwnerStorageResult {
        data: (),
        affected_records,
    })
}

/// kFragの保存（単体）
pub fn store_kfrag(
    deps: DepsMut,
    kfrag_id: String,
    encrypted_kfrag: Vec<u8>,
    target_holder: String,
    current_time: Timestamp,
) -> StdResult<OwnerStorageResult<String>> {
    let kfrag_data = OwnerKFragData {
        kfrag_id: kfrag_id.clone(),
        encrypted_kfrag,
        target_holder,
        created_at: current_time.seconds(),
        distributed: false,
    };

    OWNER_KFRAGS.save(deps.storage, kfrag_id.clone(), &kfrag_data)?;

    Ok(OwnerStorageResult {
        data: kfrag_id,
        affected_records: 1,
    })
}

/// kFragの配布状態更新
pub fn mark_kfrag_distributed(
    deps: DepsMut,
    kfrag_id: String,
) -> StdResult<OwnerStorageResult<bool>> {
    let mut kfrag_data = OWNER_KFRAGS.load(deps.storage, kfrag_id.clone())
        .map_err(|_| StdError::generic_err(format!("KFrag not found: {}", kfrag_id)))?;

    kfrag_data.distributed = true;
    OWNER_KFRAGS.save(deps.storage, kfrag_id, &kfrag_data)?;

    Ok(OwnerStorageResult {
        data: true,
        affected_records: 1,
    })
}

/// Holder割り当ての記録
pub fn assign_holder(
    deps: DepsMut,
    holder_id: String,
    kfrag_ids: Vec<String>,
    current_time: Timestamp,
) -> StdResult<OwnerStorageResult<String>> {
    // 既存の割り当てがあるかチェック
    let mut assignment = HOLDER_ASSIGNMENTS.may_load(deps.storage, holder_id.clone())?
        .unwrap_or(HolderAssignment {
            holder_id: holder_id.clone(),
            assigned_kfrags: Vec::new(),
            assignment_time: current_time.seconds(),
            status: AssignmentStatus::Pending,
        });

    // kFrag IDsを追加（重複排除）
    for kfrag_id in kfrag_ids {
        if !assignment.assigned_kfrags.contains(&kfrag_id) {
            assignment.assigned_kfrags.push(kfrag_id);
        }
    }

    // 最新の割り当て時刻を更新
    assignment.assignment_time = current_time.seconds();

    HOLDER_ASSIGNMENTS.save(deps.storage, holder_id.clone(), &assignment)?;

    Ok(OwnerStorageResult {
        data: holder_id,
        affected_records: 1,
    })
}

/// Holder割り当て状態の更新
pub fn update_holder_assignment_status(
    deps: DepsMut,
    holder_id: String,
    status: AssignmentStatus,
) -> StdResult<OwnerStorageResult<AssignmentStatus>> {
    let mut assignment = HOLDER_ASSIGNMENTS.load(deps.storage, holder_id.clone())
        .map_err(|_| StdError::generic_err(format!("Holder assignment not found: {}", holder_id)))?;

    assignment.status = status.clone();
    HOLDER_ASSIGNMENTS.save(deps.storage, holder_id, &assignment)?;

    Ok(OwnerStorageResult {
        data: status,
        affected_records: 1,
    })
}

/// kFragの検索（Holder ID別）
pub fn get_kfrags_by_holder(
    deps: Deps,
    holder_id: String,
) -> StdResult<Vec<OwnerKFragData>> {
    let kfrags: StdResult<Vec<_>> = OWNER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((_, kfrag)) => {
                    if kfrag.target_holder == holder_id {
                        Some(Ok(kfrag))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect();

    kfrags
}

/// 未配布のkFrags一覧取得
pub fn get_undistributed_kfrags(deps: Deps) -> StdResult<Vec<OwnerKFragData>> {
    let kfrags: StdResult<Vec<_>> = OWNER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((_, kfrag)) => {
                    if !kfrag.distributed {
                        Some(Ok(kfrag))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect();

    kfrags
}

/// Holder割り当て統計情報
#[derive(Debug)]
pub struct HolderAssignmentStats {
    pub total_holders: usize,
    pub pending_assignments: usize,
    pub confirmed_assignments: usize,
    pub failed_assignments: usize,
    pub total_kfrags_assigned: usize,
}

/// Holder割り当て統計の取得
pub fn get_holder_assignment_stats(deps: Deps) -> StdResult<HolderAssignmentStats> {
    let assignments: StdResult<Vec<_>> = HOLDER_ASSIGNMENTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, assignment)| assignment))
        .collect();

    let assignments = assignments?;

    let total_holders = assignments.len();
    let pending_assignments = assignments.iter()
        .filter(|a| matches!(a.status, AssignmentStatus::Pending))
        .count();
    let confirmed_assignments = assignments.iter()
        .filter(|a| matches!(a.status, AssignmentStatus::Confirmed))
        .count();
    let failed_assignments = assignments.iter()
        .filter(|a| matches!(a.status, AssignmentStatus::Failed))
        .count();
    let total_kfrags_assigned = assignments.iter()
        .map(|a| a.assigned_kfrags.len())
        .sum();

    Ok(HolderAssignmentStats {
        total_holders,
        pending_assignments,
        confirmed_assignments,
        failed_assignments,
        total_kfrags_assigned,
    })
}

/// Owner設定の更新
pub fn update_owner_config(
    deps: DepsMut,
    authorized_holders: Option<Vec<String>>,
    encryption_key: Option<String>,
) -> StdResult<OwnerStorageResult<OwnerConfig>> {
    let mut config = OWNER_CONFIG.load(deps.storage)?;

    if let Some(holders) = authorized_holders {
        config.authorized_holders = holders;
    }

    if let Some(key) = encryption_key {
        config.encryption_key = key;
    }

    OWNER_CONFIG.save(deps.storage, &config)?;

    Ok(OwnerStorageResult {
        data: config,
        affected_records: 1,
    })
}

/// 全kFragデータのクリーンアップ（セキュリティ対応）
pub fn cleanup_sensitive_data(deps: DepsMut) -> StdResult<OwnerStorageResult<usize>> {
    let mut cleaned_count = 0;

    // すべてのkFragsを取得し、メモリクリア済みのものに置き換え
    let kfrag_keys: StdResult<Vec<_>> = OWNER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(key, _)| key))
        .collect();

    for key in kfrag_keys? {
        // 空のVecに置き換えることでメモリクリア
        let mut kfrag = OWNER_KFRAGS.load(deps.storage, key.clone())?;
        kfrag.encrypted_kfrag.zeroize(); // 明示的にゼロクリア
        kfrag.encrypted_kfrag = Vec::new();

        OWNER_KFRAGS.save(deps.storage, key, &kfrag)?;
        cleaned_count += 1;
    }

    Ok(OwnerStorageResult {
        data: cleaned_count,
        affected_records: cleaned_count,
    })
}

/// プロセス役割の検証
pub fn verify_owner_role(deps: Deps) -> Result<(), StorageError> {
    let config = OWNER_CONFIG.load(deps.storage)
        .map_err(|_| StorageError::DataNotFound {
            key: "owner_config".to_string()
        })?;

    if !matches!(config.process_role, ProcessRole::Owner) {
        return Err(StorageError::Unauthorized);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    #[test]
    fn test_store_and_retrieve_kfrag() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 初期設定
        let metadata = OwnerMetadata {
            threshold_k: 3,
            total_holders_n: 5,
            capsule_txid: "test_capsule".to_string(),
            requester_pubkey: "test_pubkey".to_string(),
            creation_time: env.block.time.seconds(),
        };
        let config = OwnerConfig {
            process_role: ProcessRole::Owner,
            encryption_key: "test_key".to_string(),
            authorized_holders: vec![],
        };

        OWNER_METADATA.save(deps.as_mut().storage, &metadata).unwrap();
        OWNER_CONFIG.save(deps.as_mut().storage, &config).unwrap();

        // kFrag保存テスト
        let result = store_kfrag(
            deps.as_mut(),
            "test_kfrag_1".to_string(),
            vec![1, 2, 3, 4],
            "holder_1".to_string(),
            env.block.time,
        ).unwrap();

        assert_eq!(result.data, "test_kfrag_1");
        assert_eq!(result.affected_records, 1);

        // 保存されたkFragを取得
        let kfrag = OWNER_KFRAGS.load(deps.as_ref().storage, "test_kfrag_1".to_string()).unwrap();
        assert_eq!(kfrag.kfrag_id, "test_kfrag_1");
        assert_eq!(kfrag.target_holder, "holder_1");
        assert!(!kfrag.distributed);
    }

    #[test]
    fn test_holder_assignment() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Holder割り当てテスト
        let result = assign_holder(
            deps.as_mut(),
            "holder_1".to_string(),
            vec!["kfrag_1".to_string(), "kfrag_2".to_string()],
            env.block.time,
        ).unwrap();

        assert_eq!(result.data, "holder_1");
        assert_eq!(result.affected_records, 1);

        // 割り当て状態の更新テスト
        let update_result = update_holder_assignment_status(
            deps.as_mut(),
            "holder_1".to_string(),
            AssignmentStatus::Confirmed,
        ).unwrap();

        assert!(matches!(update_result.data, AssignmentStatus::Confirmed));
    }

    #[test]
    fn test_undistributed_kfrags() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // kFragを追加
        store_kfrag(
            deps.as_mut(),
            "kfrag_1".to_string(),
            vec![1, 2, 3],
            "holder_1".to_string(),
            env.block.time,
        ).unwrap();

        store_kfrag(
            deps.as_mut(),
            "kfrag_2".to_string(),
            vec![4, 5, 6],
            "holder_2".to_string(),
            env.block.time,
        ).unwrap();

        // 1つを配布済みにマーク
        mark_kfrag_distributed(deps.as_mut(), "kfrag_1".to_string()).unwrap();

        // 未配布のkFragsを取得
        let undistributed = get_undistributed_kfrags(deps.as_ref()).unwrap();
        assert_eq!(undistributed.len(), 1);
        assert_eq!(undistributed[0].kfrag_id, "kfrag_2");
    }
}