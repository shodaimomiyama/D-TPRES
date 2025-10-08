use crate::ao_integration::{AOIntegration, OwnerAOIntegration, HolderAOIntegration, RequesterAOIntegration};
use crate::holder::handlers::{
    handle_generate_cfrag, handle_receive_kfrag, handle_store_capsule, query_cached_capsules,
    query_cfrags, query_holder_metadata, query_holder_statistics, query_kfrag_by_id,
};
use crate::msg::{
    CFragCollectionResponse, CFragsResponse, CachedCapsulesResponse, ExecuteMsg,
    HolderAssignmentsResponse, HolderMetadataResponse, InstantiateMsg, KFragInfo, KFragsResponse,
    OwnerMetadataResponse, ProcessMetadata, ProcessRoleResponse, ProcessStatusResponse, QueryMsg,
    RecoverySessionResponse, RequesterMetadataResponse, ThresholdInfoResponse, ValidateMessage,
};
use crate::owner::handlers::{
    handle_receive_kfrags, query_holder_assignments,
    query_kfrags, query_owner_metadata,
};
use crate::requester::handlers::{
    handle_collect_cfrag, handle_initiate_recovery, handle_start_cfrag_collection,
    query_cfrag_collection, query_recovery_session, query_requester_metadata, query_threshold_info,
};
use crate::state::{
    AssignmentStatus, CapsuleData, HolderCFragData, HolderKFragData, HolderMetadata, OwnerConfig,
    OwnerKFragData, OwnerMetadata, ProcessRole, RequesterMetadata, CAPSULE_CACHE, CFRAG_COLLECTION,
    HOLDER_ASSIGNMENTS, HOLDER_CFRAGS, HOLDER_KFRAGS, HOLDER_METADATA, OWNER_CONFIG, OWNER_KFRAGS,
    OWNER_METADATA, RECOVERY_SESSIONS, REQUESTER_METADATA, THRESHOLD_TRACKER,
};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

// カスタムエラー型
#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized: expected role {expected}, got {actual}")]
    UnauthorizedRole { expected: String, actual: String },

    #[error("Process not initialized")]
    ProcessNotInitialized,

    #[error("Invalid message for process role: {role}")]
    InvalidMessageForRole { role: String },

    #[error("KFrag not found: {id}")]
    KFragNotFound { id: String },

    #[error("CFrag not found: {id}")]
    CFragNotFound { id: String },

    #[error("Session not found: {id}")]
    SessionNotFound { id: String },

    #[error("Insufficient cFrags: need {required}, have {available}")]
    InsufficientCFrags { required: u32, available: u32 },

    #[error("Threshold not met: {current}/{required}")]
    ThresholdNotMet { current: u32, required: u32 },

    #[error("Validation error: {msg}")]
    ValidationError { msg: String },

    #[error("Storage error: {msg}")]
    StorageError { msg: String },

    #[error("Duplicate KFrag: {id}")]
    DuplicateKFrag { id: String },

    #[error("Duplicate Capsule: {id}")]
    DuplicateCapsule { id: String },

    #[error("KFrag already processed: {id}")]
    KFragAlreadyProcessed { id: String },

    #[error("Unauthorized owner: {owner_id}")]
    UnauthorizedOwner { owner_id: String },

    #[error("Invalid signature: {context}")]
    InvalidSignature { context: String },

    #[error("Reencryption failed: {reason}")]
    ReencryptionFailed { reason: String },

    #[error("Arweave storage error: {reason}")]
    ArweaveStorageError { reason: String },

    #[error("Invalid input: {msg}")]
    InvalidInput { msg: String },

    #[error("Invalid state: {msg}")]
    InvalidState { msg: String },

    #[error("Not found: {msg}")]
    NotFound { msg: String },

    #[error("Unauthorized: {msg}")]
    Unauthorized { msg: String },
}

type ContractResult<T = Response> = Result<T, ContractError>;

// インスタンス化関数（ユニバーサル初期化）
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    // メッセージバリデーション
    msg.validate()
        .map_err(|e| ContractError::ValidationError { msg: e })?;

    // 全てのロールのメタデータをデフォルト値で初期化
    let mut owner_metadata = OwnerMetadata::default();
    let mut holder_metadata = HolderMetadata::default();
    let mut requester_metadata = RequesterMetadata::default();

    // 指定されたロールのメタデータを実際の値で設定
    let mut response = Response::new();
    match msg.process_role {
        ProcessRole::Owner => {
            if let ProcessMetadata::Owner {
                owner_id,
                total_holders_n,
                signer_pubkey,
                holder_process_ids,
            } = msg.metadata
            {
                owner_metadata = OwnerMetadata {
                    owner_id: owner_id.clone(),
                    total_holders_n,
                    creation_time: env.block.time.seconds(),
                    signer_pubkey: signer_pubkey.clone(),
                    holder_process_ids: holder_process_ids.clone(),
                };

                let owner_config = OwnerConfig {
                    process_role: ProcessRole::Owner,
                };
                OWNER_CONFIG.save(deps.storage, &owner_config)?;

                response = response
                    .add_attribute("action", "instantiate_owner")
                    .add_attribute("owner_id", owner_id)
                    .add_attribute("total_holders_n", total_holders_n.to_string())
                    .add_attribute("signer_pubkey", signer_pubkey);
            } else {
                return Err(ContractError::ValidationError {
                    msg: "Invalid metadata for Owner process".to_string(),
                });
            }
        }
        ProcessRole::Holder => {
            if let ProcessMetadata::Holder { holder_id } = msg.metadata {
                holder_metadata = HolderMetadata {
                    holder_id: holder_id.clone(),
                    process_role: ProcessRole::Holder,
                    assigned_owners: Vec::new(),
                    initialization_time: env.block.time.seconds(),
                };

                response = response
                    .add_attribute("action", "instantiate_holder")
                    .add_attribute("holder_id", holder_id);
            } else {
                return Err(ContractError::ValidationError {
                    msg: "Invalid metadata for Holder process".to_string(),
                });
            }
        }
        ProcessRole::Requester => {
            if let ProcessMetadata::Requester { requester_id } = msg.metadata {
                requester_metadata = RequesterMetadata {
                    requester_id: requester_id.clone(),
                    process_role: ProcessRole::Requester,
                    active_sessions: Vec::new(),
                    initialization_time: env.block.time.seconds(),
                };

                response = response
                    .add_attribute("action", "instantiate_requester")
                    .add_attribute("requester_id", requester_id);
            } else {
                return Err(ContractError::ValidationError {
                    msg: "Invalid metadata for Requester process".to_string(),
                });
            }
        }
    }

    // 全てのメタデータを保存
    OWNER_METADATA.save(deps.storage, &owner_metadata)?;
    HOLDER_METADATA.save(deps.storage, &holder_metadata)?;
    REQUESTER_METADATA.save(deps.storage, &requester_metadata)?;

    Ok(response.add_attribute("universal_init", "true"))
}


// 実行関数
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    match msg {
        // Owner-Process メッセージ (PHASE 2のみ)
        ExecuteMsg::ReceiveKFrags { kfrags } => {
            handle_receive_kfrags(deps, env, info, kfrags)
        }

        // Holder-Process メッセージ
        ExecuteMsg::ReceiveKFrag { kfrag_data } => {
            handle_receive_kfrag(deps, env, info, kfrag_data)
        }
        ExecuteMsg::GenerateCFrag { kfrag_id } => handle_generate_cfrag(deps, env, info, kfrag_id),
        ExecuteMsg::StoreCapsule { capsule_data } => {
            handle_store_capsule(deps, env, info, capsule_data)
        }

        // Requester-Process メッセージ
        ExecuteMsg::StartCFragCollection {
            session_id,
            threshold,
        } => handle_start_cfrag_collection(deps, env, info, session_id, threshold),
        ExecuteMsg::CollectCFrag {
            session_id,
            cfrag_data,
        } => handle_collect_cfrag(deps, env, info, session_id, cfrag_data),
        ExecuteMsg::InitiateRecovery {
            session_id,
            capsule_data,
        } => handle_initiate_recovery(deps, env, info, session_id, capsule_data),

        // AO Network プロセス間メッセージ
        ExecuteMsg::SendKFragToHolder {
            target_process,
            kfrag,
            owner_process,
        } => execute_send_kfrag_to_holder(deps, env, info, target_process, kfrag, owner_process),

        ExecuteMsg::SendCFragToRequester {
            target_process,
            cfrag,
            holder_process,
        } => execute_send_cfrag_to_requester(deps, env, info, target_process, cfrag, holder_process),

        ExecuteMsg::RequestCFragFromHolder {
            target_process,
            session_id,
            requester_process,
        } => execute_request_cfrag_from_holder(deps, env, info, target_process, session_id, requester_process),

        // 共通メッセージ
        ExecuteMsg::UpdateProcessStatus { status } => {
            execute_update_process_status(deps, env, status)
        }
    }
}

// AO Network プロセス間メッセージハンドラ

/// Owner-Process → Holder-Process へのkFrag送信
fn execute_send_kfrag_to_holder(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _target_process: String,
    kfrag: crate::msg::KFragDistribution,
    owner_process: String,
) -> ContractResult {
    // Holder-Processでのみ処理
    verify_process_role(deps.as_ref(), ProcessRole::Holder)?;

    // Owner-Processからの送信であることを確認
    if info.sender.to_string() != owner_process {
        return Err(ContractError::Unauthorized {
            msg: format!("Expected message from owner {}, got {}", owner_process, info.sender),
        });
    }

    // kFragを受信処理
    let kfrag_receipt = crate::msg::KFragReceiptData {
        id: kfrag.id,
        encrypted_kfrag: kfrag.encrypted_data,
        signature: kfrag.signature,
        owner_id: owner_process,
    };

    handle_receive_kfrag(deps, env, info, kfrag_receipt)
}

/// Holder-Process → Requester-Process へのcFrag送信
fn execute_send_cfrag_to_requester(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _target_process: String,
    cfrag: crate::msg::CFragSubmission,
    holder_process: String,
) -> ContractResult {
    // Requester-Processでのみ処理
    verify_process_role(deps.as_ref(), ProcessRole::Requester)?;

    // Holder-Processからの送信であることを確認
    if info.sender.to_string() != holder_process {
        return Err(ContractError::Unauthorized {
            msg: format!("Expected message from holder {}, got {}", holder_process, info.sender),
        });
    }

    // セッションIDを特定し、cFragを収集処理に渡す
    // 実際の実装では、cfragからsession_idを導出または事前に関連付ける必要がある
    let session_id = format!("session_for_{}", cfrag.cfrag_id);

    handle_collect_cfrag(deps, env, info, session_id, cfrag)
}

/// Requester-Process → Holder-Process へのcFrag要求
fn execute_request_cfrag_from_holder(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _target_process: String,
    session_id: String,
    requester_process: String,
) -> ContractResult {
    // Holder-Processでのみ処理
    verify_process_role(deps.as_ref(), ProcessRole::Holder)?;

    // Requester-Processからの要求であることを確認
    if info.sender.to_string() != requester_process {
        return Err(ContractError::Unauthorized {
            msg: format!("Expected message from requester {}, got {}", requester_process, info.sender),
        });
    }

    // プロセスレジストリからrequester_processを検証（プレースホルダー実装）
    // 実際の実装では適切な方法でレジストリを取得
    let registry = crate::state::ProcessRegistry::new("mock_process".to_string(), crate::state::ProcessRole::Holder);
    if !registry.requester_processes.contains(&requester_process) {
        return Err(ContractError::Unauthorized {
            msg: "Requester process not registered".to_string(),
        });
    }

    // 利用可能なcFragがあるかチェック
    let available_cfrags = query_cfrags(deps.as_ref(), Some(true))?;

    if let Ok(cfrags_response) = serde_json::from_slice::<crate::msg::CFragsResponse>(&available_cfrags) {
        if !cfrags_response.cfrags.is_empty() {
            // 最初の利用可能なcFragを送信
            let cfrag_info = &cfrags_response.cfrags[0];

            // cFragデータを構築（実際の実装では適切なデータを取得）
            let cfrag_submission = crate::msg::CFragSubmission {
                cfrag_id: cfrag_info.cfrag_id.clone(),
                holder_id: env.contract.address.to_string(), // 現在のHolder Process ID
                holder_process_id: env.contract.address.to_string(),
                cfrag_data: vec![0; 32], // モックデータ、実際の実装では適切なcFragデータ
                signature: vec![0; 64], // モック署名
            };

            // Requester-ProcessにcFragを送信
            let send_msg = HolderAOIntegration::send_cfrag_to_requester(
                requester_process.clone(),
                cfrag_submission,
                env.contract.address.to_string(),
            )?;

            Ok(Response::new()
                .add_message(send_msg)
                .add_attribute("action", "send_cfrag_to_requester")
                .add_attribute("session_id", session_id)
                .add_attribute("requester", requester_process)
                .add_attribute("cfrag_id", cfrag_info.cfrag_id.clone()))
        } else {
            Err(ContractError::NotFound {
                msg: "No available cFrags".to_string(),
            })
        }
    } else {
        Err(ContractError::InvalidState {
            msg: "Failed to parse cFrags response".to_string(),
        })
    }
}

// 共通実行関数
fn execute_update_process_status(_deps: DepsMut, _env: Env, status: String) -> ContractResult {
    Ok(Response::new()
        .add_attribute("action", "update_process_status")
        .add_attribute("status", status))
}

// クエリ関数
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Owner-Process クエリ
        QueryMsg::GetOwnerMetadata {} => query_owner_metadata(deps),
        QueryMsg::GetKFrags { holder_id } => query_kfrags(deps, holder_id),
        QueryMsg::GetHolderAssignments {} => query_holder_assignments(deps),

        // Holder-Process クエリ
        QueryMsg::GetHolderMetadata {} => query_holder_metadata(deps),
        QueryMsg::GetKFragById { kfrag_id } => query_kfrag_by_id(deps, kfrag_id),
        QueryMsg::GetCFrags { processed_only } => query_cfrags(deps, processed_only),
        QueryMsg::GetCachedCapsules {} => query_cached_capsules(deps),

        // Requester-Process クエリ
        QueryMsg::GetRequesterMetadata {} => query_requester_metadata(deps),
        QueryMsg::GetCFragCollection { session_id } => query_cfrag_collection(deps, session_id),
        QueryMsg::GetRecoverySession { session_id } => query_recovery_session(deps, session_id),
        QueryMsg::GetThresholdInfo {} => query_threshold_info(deps),

        // AO Network プロセス管理クエリ
        QueryMsg::GetConnectedProcesses {} => query_connected_processes(deps),
        QueryMsg::GetProcessInfo { process_id } => query_process_info(deps, process_id),

        // 共通クエリ
        QueryMsg::GetProcessRole {} => to_json_binary(&query_process_role(deps)?),
        QueryMsg::GetProcessStatus {} => to_json_binary(&query_process_status(deps)?),
    }
}

// ヘルパー関数
fn verify_process_role(deps: Deps, expected_role: ProcessRole) -> Result<(), ContractError> {
    let actual_role = get_process_role(deps)?;
    if actual_role != expected_role {
        return Err(ContractError::UnauthorizedRole {
            expected: expected_role.to_string(),
            actual: actual_role.to_string(),
        });
    }
    Ok(())
}

fn get_process_role(deps: Deps) -> Result<ProcessRole, ContractError> {
    // Owner役割の確認（空でない値がある場合）
    if let Ok(owner_metadata) = OWNER_METADATA.load(deps.storage) {
        if !owner_metadata.owner_id.is_empty() {
            return Ok(ProcessRole::Owner);
        }
    }

    // Holder役割の確認（空でない値がある場合）
    if let Ok(holder_metadata) = HOLDER_METADATA.load(deps.storage) {
        if !holder_metadata.holder_id.is_empty() {
            return Ok(ProcessRole::Holder);
        }
    }

    // Requester役割の確認（空でない値がある場合）
    if let Ok(requester_metadata) = REQUESTER_METADATA.load(deps.storage) {
        if !requester_metadata.requester_id.is_empty() {
            return Ok(ProcessRole::Requester);
        }
    }

    Err(ContractError::ProcessNotInitialized)
}

fn query_process_role(deps: Deps) -> StdResult<ProcessRoleResponse> {
    let role = get_process_role(deps).map_err(|e| StdError::generic_err(format!("{}", e)))?;
    Ok(ProcessRoleResponse { role })
}

fn query_process_status(_deps: Deps) -> StdResult<ProcessStatusResponse> {
    // プレースホルダー実装
    Ok(ProcessStatusResponse {
        status: "active".to_string(),
        last_updated: 0,
    })
}

// AO Network クエリ関数

fn query_connected_processes(_deps: Deps) -> StdResult<Binary> {
    // プレースホルダー実装 - 実際の実装では適切な方法でプロセス情報を取得
    let processes = Vec::new();
    let response = crate::msg::ConnectedProcessesResponse { processes };
    to_json_binary(&response)
}

fn query_process_info(_deps: Deps, process_id: String) -> StdResult<Binary> {
    // プレースホルダー実装 - クエリはDepsを使用し、変更を行わない
    let mock_process_info = crate::msg::ProcessInfo {
        process_id: process_id.clone(),
        wasm_tx_id: "mock_wasm_tx_id".to_string(),
        process_role: crate::state::ProcessRole::Owner,
        spawned_at: 0,
        status: crate::msg::ProcessStatus::Active,
    };
    let response = crate::msg::ProcessInfoResponse {
        info: mock_process_info
    };
    to_json_binary(&response)
}
