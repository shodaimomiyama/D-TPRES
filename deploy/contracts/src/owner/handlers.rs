use cosmwasm_std::{
    Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    to_json_binary, Binary, CosmosMsg
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::contract::ContractError;
use crate::msg::{
    KFragDistribution, ValidateMessage,
    OwnerMetadataResponse, KFragsResponse, KFragInfo,
    HolderAssignmentsResponse, HolderAssignmentInfo
};
use crate::state::{
    AssignmentStatus,
    OWNER_METADATA, OWNER_KFRAGS, HOLDER_ASSIGNMENTS
};
use crate::ao_integration::{AOIntegration, OwnerAOIntegration};

#[cfg(test)]
use crate::state::OWNER_CONFIG;
use crate::owner::storage::{
    load_owner_state, store_kfrag, mark_kfrag_distributed,
    assign_holder, update_holder_assignment_status, get_kfrags_by_holder,
    get_undistributed_kfrags, get_holder_assignment_stats, verify_owner_role
};

/// Owner-Process専用メッセージハンドラ
///
/// このモジュールは Owner-Process のみが処理するメッセージハンドラを提供します。
/// PRDのPHASE 1-8, PHASE 2-1,2-2 に対応した実装を行います。

/// RandAO選出結果のモック（実装時は実際のRandAO連携）
#[derive(Debug, Clone)]
pub struct RandAOSelection {
    pub selected_holders: Vec<String>,
    pub selection_seed: String,
    pub block_height: u64,
}

/// kFrag配布の結果
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct KFragDistributionResult {
    pub distributed_count: usize,
    pub failed_distributions: Vec<String>,
    #[zeroize(skip)]
    pub selected_holders: Vec<String>,
}

/// Owner-Process: kFrags配布ハンドラ（PHASE 2-1, 2-2）
///
/// PRD仕様:
/// - O-BrowserからkFragsを受信
/// - RandAOを利用してn個のHolder-Processを選出
/// - 各Holder-Processに kFragⱼ と署名を送信
pub fn handle_distribute_kfrags(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    kfrags: Vec<KFragDistribution>,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_owner_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Owner".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 現在の状態をロード
    let _current_state = load_owner_state(deps.as_ref(), env.block.time)
        .map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 3. kFragsのバリデーション
    for kfrag in &kfrags {
        kfrag.validate()
            .map_err(|e| ContractError::ValidationError { msg: e })?;
    }

    // 4. RandAO選出（モック実装）
    let metadata = OWNER_METADATA.load(deps.storage)?;
    let selection_result = perform_randao_selection(
        metadata.total_holders_n,
        env.block.height,
    )?;

    // 5. kFrags保存と配布準備
    let mut distribution_result = KFragDistributionResult {
        distributed_count: 0,
        failed_distributions: Vec::new(),
        selected_holders: selection_result.selected_holders.clone(),
    };

    let mut response_attributes = Vec::new();
    response_attributes.push(("action".to_string(), "distribute_kfrags".to_string()));
    response_attributes.push(("sender".to_string(), info.sender.to_string()));

    for (index, kfrag) in kfrags.iter().enumerate() {
        let target_holder = if index < selection_result.selected_holders.len() {
            selection_result.selected_holders[index].clone()
        } else {
            // フォールバック: 既存のtarget_holderを使用
            kfrag.holder_id.clone()
        };

        match store_kfrag(
            deps.branch(),
            kfrag.id.clone(),
            kfrag.encrypted_data.clone(),
            target_holder.clone(),
            env.block.time,
        ) {
            Ok(_) => {
                // Holder割り当て記録
                if let Err(e) = assign_holder(
                    deps.branch(),
                    target_holder.clone(),
                    vec![kfrag.id.clone()],
                    env.block.time,
                ) {
                    distribution_result.failed_distributions.push(
                        format!("Failed to assign holder {}: {}", target_holder, e)
                    );
                } else {
                    distribution_result.distributed_count += 1;
                    response_attributes.push((
                        format!("kfrag_{}_assigned_to", kfrag.id),
                        target_holder,
                    ));
                }
            }
            Err(e) => {
                distribution_result.failed_distributions.push(
                    format!("Failed to store kFrag {}: {}", kfrag.id, e)
                );
            }
        }
    }

    // 6. AO Network経由でHolder-ProcessesにkFragsを送信
    let mut cosmos_messages: Vec<CosmosMsg> = Vec::new();

    for (index, kfrag) in kfrags.iter().enumerate() {
        let target_holder = if index < selection_result.selected_holders.len() {
            selection_result.selected_holders[index].clone()
        } else {
            kfrag.holder_id.clone()
        };

        // Holder-ProcessのProcess IDを取得（実際の実装では適切なマッピング）
        let holder_process_id = format!("holder_process_{}", target_holder);

        // AO Networkメッセージとして送信
        match OwnerAOIntegration::send_kfrag_to_holder(
            holder_process_id.clone(),
            kfrag.clone(),
            env.contract.address.to_string(),
        ) {
            Ok(cosmos_msg) => {
                cosmos_messages.push(cosmos_msg);
                response_attributes.push((
                    format!("kfrag_{}_sent_to_process", kfrag.id),
                    holder_process_id,
                ));
            }
            Err(e) => {
                distribution_result.failed_distributions.push(
                    format!("Failed to create message for kFrag {}: {}", kfrag.id, e)
                );
            }
        }
    }

    // 7. 配布統計をレスポンスに追加
    response_attributes.push((
        "distributed_count".to_string(),
        distribution_result.distributed_count.to_string()
    ));
    response_attributes.push((
        "failed_count".to_string(),
        distribution_result.failed_distributions.len().to_string()
    ));
    response_attributes.push((
        "randao_seed".to_string(),
        selection_result.selection_seed
    ));
    response_attributes.push((
        "ao_messages_count".to_string(),
        cosmos_messages.len().to_string()
    ));

    let mut response = Response::new();
    for (key, value) in response_attributes {
        response = response.add_attribute(key, value);
    }

    // AO Networkメッセージを追加
    for cosmos_msg in cosmos_messages {
        response = response.add_message(cosmos_msg);
    }

    Ok(response)
}

/// Owner-Process: Holder割り当て更新ハンドラ
pub fn handle_update_holder_assignment(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    holder_id: String,
    kfrag_ids: Vec<String>,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_owner_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Owner".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 入力バリデーション
    if holder_id.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "Holder ID cannot be empty".to_string(),
        });
    }

    if kfrag_ids.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "KFrag IDs list cannot be empty".to_string(),
        });
    }

    // 3. kFragsの存在確認
    for kfrag_id in &kfrag_ids {
        if OWNER_KFRAGS.may_load(deps.storage, kfrag_id.clone())?.is_none() {
            return Err(ContractError::KFragNotFound {
                id: kfrag_id.clone(),
            });
        }
    }

    // 4. Holder割り当て更新
    let _assignment_result = assign_holder(
        deps.branch(),
        holder_id.clone(),
        kfrag_ids.clone(),
        env.block.time,
    )?;

    Ok(Response::new()
        .add_attribute("action", "update_holder_assignment")
        .add_attribute("holder_id", holder_id)
        .add_attribute("assigned_kfrags", kfrag_ids.len().to_string())
        .add_attribute("sender", info.sender.to_string()))
}

/// Owner-Process: 配布確認受信ハンドラ
///
/// Holder-Processから配布確認を受信し、状態を更新
pub fn handle_distribution_confirmation(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    holder_id: String,
    kfrag_id: String,
    success: bool,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_owner_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Owner".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. kFragの存在確認
    if OWNER_KFRAGS.may_load(deps.storage, kfrag_id.clone())?.is_none() {
        return Err(ContractError::KFragNotFound { id: kfrag_id });
    }

    // 3. 配布状態の更新
    if success {
        mark_kfrag_distributed(deps.branch(), kfrag_id.clone())?;
        update_holder_assignment_status(
            deps.branch(),
            holder_id.clone(),
            AssignmentStatus::Confirmed,
        )?;
    } else {
        update_holder_assignment_status(
            deps.branch(),
            holder_id.clone(),
            AssignmentStatus::Failed,
        )?;
    }

    Ok(Response::new()
        .add_attribute("action", "distribution_confirmation")
        .add_attribute("holder_id", holder_id)
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("success", success.to_string())
        .add_attribute("sender", info.sender.to_string()))
}

/// RandAO選出のモック実装
///
/// 実際の実装では、AO NetworkのRandAO機能と連携します
fn perform_randao_selection(
    total_holders_n: u32,
    block_height: u64,
) -> Result<RandAOSelection, ContractError> {
    // モック実装: ブロック高度を使用した疑似ランダム選出
    let seed = format!("randao_seed_{}", block_height);

    // 簡単な疑似ランダム選出（実装時はより堅牢なアルゴリズムを使用）
    let mut selected_holders = Vec::new();
    for i in 0..total_holders_n {
        let holder_id = format!("holder_process_{}",
            (block_height + i as u64) % 1000 // 1000個のプロセスプールから選出
        );
        selected_holders.push(holder_id);
    }

    Ok(RandAOSelection {
        selected_holders,
        selection_seed: seed,
        block_height,
    })
}

/// Owner-Process: メタデータクエリハンドラ
pub fn query_owner_metadata(deps: Deps) -> StdResult<Binary> {
    let metadata = OWNER_METADATA.load(deps.storage)?;
    to_json_binary(&OwnerMetadataResponse { metadata })
}

/// Owner-Process: kFragsクエリハンドラ
pub fn query_kfrags(deps: Deps, holder_id: Option<String>) -> StdResult<Binary> {
    let kfrags = if let Some(holder_id) = holder_id {
        get_kfrags_by_holder(deps, holder_id)?
    } else {
        get_undistributed_kfrags(deps)?
    };

    let kfrag_infos: Vec<KFragInfo> = kfrags
        .into_iter()
        .map(|kfrag| KFragInfo {
            kfrag_id: kfrag.kfrag_id.clone(),
            target_holder: kfrag.target_holder.clone(),
            created_at: kfrag.created_at,
            distributed: kfrag.distributed,
        })
        .collect();

    to_json_binary(&KFragsResponse { kfrags: kfrag_infos })
}

/// Owner-Process: Holder割り当てクエリハンドラ
pub fn query_holder_assignments(deps: Deps) -> StdResult<Binary> {
    let assignments: StdResult<Vec<_>> = HOLDER_ASSIGNMENTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, assignment)| assignment))
        .collect();

    let assignment_infos: Vec<HolderAssignmentInfo> = assignments?
        .into_iter()
        .map(|assignment| HolderAssignmentInfo {
            holder_id: assignment.holder_id,
            assigned_kfrags: assignment.assigned_kfrags,
            assignment_time: assignment.assignment_time,
            status: assignment.status.to_string(),
        })
        .collect();

    to_json_binary(&HolderAssignmentsResponse {
        assignments: assignment_infos,
    })
}

/// Owner-Process: 配布統計クエリハンドラ
pub fn query_distribution_stats(deps: Deps) -> StdResult<Binary> {
    let stats = get_holder_assignment_stats(deps)?;

    // 統計情報をJSON形式で返す
    let stats_json = serde_json::json!({
        "total_holders": stats.total_holders,
        "pending_assignments": stats.pending_assignments,
        "confirmed_assignments": stats.confirmed_assignments,
        "failed_assignments": stats.failed_assignments,
        "total_kfrags_assigned": stats.total_kfrags_assigned,
    });

    to_json_binary(&stats_json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::state::{OwnerMetadata, OwnerConfig, ProcessRole};

    fn setup_owner_process(deps: DepsMut) {
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
    fn test_distribute_kfrags_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);

        setup_owner_process(deps.as_mut());

        let kfrags = vec![
            KFragDistribution {
                id: "kfrag_1".to_string(),
                encrypted_data: vec![1, 2, 3, 4],
                holder_id: "holder_1".to_string(),
                signature: vec![5, 6, 7, 8],
            },
            KFragDistribution {
                id: "kfrag_2".to_string(),
                encrypted_data: vec![9, 10, 11, 12],
                holder_id: "holder_2".to_string(),
                signature: vec![13, 14, 15, 16],
            },
        ];

        let result = handle_distribute_kfrags(deps.as_mut(), env, info, kfrags);
        assert!(result.is_ok());

        let response = result.unwrap();
        let attributes: std::collections::HashMap<String, String> = response
            .attributes
            .into_iter()
            .map(|attr| (attr.key, attr.value))
            .collect();

        assert_eq!(attributes.get("action"), Some(&"distribute_kfrags".to_string()));
        assert_eq!(attributes.get("distributed_count"), Some(&"2".to_string()));
    }

    #[test]
    fn test_update_holder_assignment() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner", &[]);

        setup_owner_process(deps.as_mut());

        // 先にkFragsを作成
        store_kfrag(
            deps.as_mut(),
            "kfrag_1".to_string(),
            vec![1, 2, 3],
            "holder_1".to_string(),
            env.block.time,
        ).unwrap();

        let result = handle_update_holder_assignment(
            deps.as_mut(),
            env,
            info,
            "holder_1".to_string(),
            vec!["kfrag_1".to_string()],
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_randao_selection() {
        let result = perform_randao_selection(5, 12345);
        assert!(result.is_ok());

        let selection = result.unwrap();
        assert_eq!(selection.selected_holders.len(), 5);
        assert_eq!(selection.block_height, 12345);
        assert!(!selection.selection_seed.is_empty());
    }

    #[test]
    fn test_query_owner_metadata() {
        let mut deps = mock_dependencies();
        setup_owner_process(deps.as_mut());

        let result = query_owner_metadata(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_kfrags() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_owner_process(deps.as_mut());

        // kFragを追加
        store_kfrag(
            deps.as_mut(),
            "kfrag_1".to_string(),
            vec![1, 2, 3],
            "holder_1".to_string(),
            env.block.time,
        ).unwrap();

        let result = query_kfrags(deps.as_ref(), None);
        assert!(result.is_ok());

        let result_with_holder = query_kfrags(deps.as_ref(), Some("holder_1".to_string()));
        assert!(result_with_holder.is_ok());
    }
}