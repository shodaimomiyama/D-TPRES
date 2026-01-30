use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response,
    StdError, StdResult, Storage, SubMsg, SubMsgResult, WasmMsg,
};
use cw_storage_plus::Bound;
use serde::{Deserialize, Serialize};

use bincode;
use umbral_pre::{self, DefaultDeserialize, DefaultSerialize};

use crate::msg::{
    CapsuleInfo, ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, QueryMsg,
    ValidateMessage,
};
use crate::state::{
    get_current_timestamp, CapsuleStatus, Config, HolderCFragData, IdemFlag, IndexCapsToCFragValue,
    IndexKFragToCapsValue, OwnerCapsuleData, OwnerKFragData, StorageError, CONFIG,
    DEFAULT_HOLDER_PROCESS_ID, HOLDER_CFRAGS, IDEM_FLAGS, INDEX_CAPS_TO_CFRAG, INDEX_KFRAG_TO_CAPS,
    KFRAG_HOLDERS, OWNER_CAPSULES, OWNER_KFRAGS,
};

// --------------------- カスタムエラー型 ---------------------
#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Storage(#[from] StorageError),

    #[error("Validation error: {msg}")]
    ValidationError { msg: String },

    #[error("ERR_KFRAG_NOT_FOUND: {kfrag_id}")]
    KFragNotFound { kfrag_id: String },

    #[error("ERR_CFRAG_NOT_READY: {kfrag_id}/{capsule_id}")]
    CFragNotReady {
        kfrag_id: String,
        capsule_id: String,
    },

    #[error("ERR_CAPSULE_NOT_FOUND: {kfrag_id}/{capsule_id}")]
    CapsuleNotFound {
        kfrag_id: String,
        capsule_id: String,
    },

    #[error("Reencryption failed: {reason}")]
    ReencryptionFailed { reason: String },

    #[error("Object too large: {size} bytes")]
    ObjectTooLarge { size: u64 },

    #[error("Bad request: {msg}")]
    BadRequest { msg: String },

    #[error("Process not initialized")]
    ProcessNotInitialized,
}

type ContractResult<T = Response> = Result<T, ContractError>;

pub const REPLY_DELEGATE_KFRAG: u64 = 1;
pub const REPLY_DELEGATE_CAPSULE: u64 = 2;

// --------------------- Execute ハンドラー ---------------------

pub fn handle_delegate_kfrag(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    kfrag: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let holder_process_id = DEFAULT_HOLDER_PROCESS_ID.to_string();
    let process_id = config.process_id.clone();

    let holder_key = (process_id.clone(), kfrag_id.clone());
    if let Some(existing_holder) = KFRAG_HOLDERS.may_load(deps.storage, holder_key.clone())? {
        if existing_holder != holder_process_id {
            return Err(ContractError::BadRequest {
                msg: format!("kFrag already delegated to holder {}", existing_holder),
            });
        }
    } else {
        KFRAG_HOLDERS.save(deps.storage, holder_key, &holder_process_id)?;
    }

    let wasm_msg = WasmMsg::Execute {
        contract_addr: holder_process_id.clone(),
        msg: to_json_binary(&ExecuteMsg::SubmitKFrag {
            kfrag_id: kfrag_id.clone(),
            kfrag,
        })?,
        funds: vec![],
    };
    let sub_msg = SubMsg {
        id: REPLY_DELEGATE_KFRAG,
        msg: CosmosMsg::Wasm(wasm_msg),
        gas_limit: None,
        reply_on: ReplyOn::Error,
    };

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "delegate_kfrag")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("holder_process_id", holder_process_id)
        .add_attribute("process_id", process_id))
}

pub fn handle_delegate_capsule(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let process_id = config.process_id.clone();
    let holder_process_id = KFRAG_HOLDERS
        .may_load(deps.storage, (process_id.clone(), kfrag_id.clone()))?
        .ok_or(ContractError::KFragNotFound {
            kfrag_id: kfrag_id.clone(),
        })?;

    let wasm_msg = WasmMsg::Execute {
        contract_addr: holder_process_id.clone(),
        msg: to_json_binary(&ExecuteMsg::SubmitCapsule {
            kfrag_id: kfrag_id.clone(),
            capsule_id: capsule_id.clone(),
            capsule,
        })?,
        funds: vec![],
    };

    let sub_msg = SubMsg {
        id: REPLY_DELEGATE_CAPSULE,
        msg: CosmosMsg::Wasm(wasm_msg),
        gas_limit: None,
        reply_on: ReplyOn::Error,
    };

    Ok(Response::new()
        .add_submessage(sub_msg)
        .add_attribute("action", "delegate_capsule")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("holder_process_id", holder_process_id)
        .add_attribute("process_id", config.process_id))
}

pub fn handle_submit_kfrag(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    kfrag: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let timestamp = get_current_timestamp(&env);
    let created = persist_kfrag(
        deps.storage,
        &config.process_id,
        &kfrag_id,
        kfrag,
        &timestamp,
    )?;

    Ok(Response::new()
        .add_attribute("action", "submit_kfrag")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("process_id", config.process_id)
        .add_attribute("status", if created { "success" } else { "no_op" }))
}

pub fn handle_submit_capsule(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let status = process_capsule_submission(
        deps,
        env,
        config.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
        capsule,
    )?;

    Ok(Response::new()
        .add_attribute("action", "submit_capsule")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("process_id", config.process_id)
        .add_attribute("status", status.as_str()))
}

pub fn handle_reencrypt(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let status = retry_reencryption(
        deps,
        env,
        config.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
    )?;

    Ok(Response::new()
        .add_attribute("action", "reencrypt")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("process_id", config.process_id)
        .add_attribute("status", status.as_str()))
}

// --------------------- Query ハンドラー ---------------------

pub fn handle_get_cfrag(
    deps: Deps,
    env: Env,
    kfrag_id: String,
    capsule_id: String,
) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let _timestamp = get_current_timestamp(&env);

    // 1. 存在判定（O(1)）
    let key = (
        config.process_id.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
    );
    if !INDEX_CAPS_TO_CFRAG.has(deps.storage, key.clone()) {
        return Err(StdError::generic_err(format!(
            "ERR_CFRAG_NOT_READY: {}/{}",
            kfrag_id, capsule_id
        )));
    }

    // 2. cFrag本体取得
    let cfrag_data = HOLDER_CFRAGS.load(deps.storage, key)?;

    // 4. レスポンス構築
    let response = GetCFragResponse {
        cfrag: cfrag_data.cfrag,
        meta: cfrag_data.meta,
    };

    to_json_binary(&response)
}

pub fn handle_list_capsules_by_kfrag(
    deps: Deps,
    _env: Env,
    kfrag_id: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let limit = limit.unwrap_or(config.default_list_limit).min(100) as usize;

    // プレフィックス決定
    let prefix = (config.process_id.clone(), kfrag_id.clone());

    // 開始位置の決定
    let start_bound = start_after.map(|s| Bound::exclusive(s));

    // レンジ走査
    let capsules: Result<Vec<CapsuleInfo>, StdError> = INDEX_KFRAG_TO_CAPS
        .prefix(prefix)
        .range(
            deps.storage,
            start_bound,
            None,
            cosmwasm_std::Order::Ascending,
        )
        .take(limit)
        .map(|item| {
            let (capsule_id, value) = item?;
            Ok(CapsuleInfo {
                capsule_id,
                status: value.status,
                updated_ts: value.updated_ts,
            })
        })
        .collect();

    let capsules = capsules?;

    // 次のページングトークン
    let next_start_after = if capsules.len() == limit {
        capsules.last().map(|c| c.capsule_id.clone())
    } else {
        None
    };

    let response = ListCapsulesByKFragResponse {
        capsules,
        next_start_after,
    };

    to_json_binary(&response)
}

// --------------------- ヘルパー関数 ---------------------

#[derive(Debug, PartialEq, Eq)]
enum PipelineStatus {
    Success,
    NoOp,
}

impl PipelineStatus {
    fn as_str(&self) -> &'static str {
        match self {
            PipelineStatus::Success => "success",
            PipelineStatus::NoOp => "no_op",
        }
    }
}

fn persist_kfrag(
    storage: &mut dyn Storage,
    process_id: &str,
    kfrag_id: &str,
    kfrag: Binary,
    timestamp: &str,
) -> Result<bool, ContractError> {
    let key = (process_id.to_string(), kfrag_id.to_string());
    if OWNER_KFRAGS.has(storage, key.clone()) {
        return Ok(false);
    }

    let kfrag_data = OwnerKFragData::new(kfrag, timestamp.to_string());
    OWNER_KFRAGS.save(storage, key, &kfrag_data)?;
    Ok(true)
}

fn save_index_status(
    storage: &mut dyn Storage,
    process_id: &str,
    kfrag_id: &str,
    capsule_id: &str,
    status: CapsuleStatus,
    timestamp: &str,
) -> Result<(), ContractError> {
    let key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    let value = IndexKFragToCapsValue::new(status, timestamp.to_string());
    INDEX_KFRAG_TO_CAPS.save(storage, key, &value)?;
    Ok(())
}

fn finalize_capsule_success(
    storage: &mut dyn Storage,
    process_id: &str,
    kfrag_id: &str,
    capsule_id: &str,
    capsule_data: &mut OwnerCapsuleData,
    timestamp: &str,
    cfrag_binary: Binary,
) -> Result<(), ContractError> {
    let cfrag_key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    let cfrag_entry = HolderCFragData::new(cfrag_binary, timestamp.to_string());
    HOLDER_CFRAGS.save(storage, cfrag_key, &cfrag_entry)?;

    let caps_index_key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    let caps_index_value = IndexCapsToCFragValue::new(timestamp.to_string());
    INDEX_CAPS_TO_CFRAG.save(storage, caps_index_key, &caps_index_value)?;

    let idem_key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    let idem_flag = IdemFlag::new(timestamp.to_string());
    IDEM_FLAGS.save(storage, idem_key, &idem_flag)?;

    capsule_data.update_status(CapsuleStatus::CFragReady, timestamp.to_string());
    let capsule_key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    OWNER_CAPSULES.save(storage, capsule_key, capsule_data)?;

    save_index_status(
        storage,
        process_id,
        kfrag_id,
        capsule_id,
        CapsuleStatus::CFragReady,
        timestamp,
    )?;

    Ok(())
}

fn mark_capsule_error(
    storage: &mut dyn Storage,
    process_id: &str,
    kfrag_id: &str,
    capsule_id: &str,
    capsule_data: &mut OwnerCapsuleData,
    timestamp: &str,
) -> Result<(), ContractError> {
    capsule_data.update_status(CapsuleStatus::Error, timestamp.to_string());
    let capsule_key = (
        process_id.to_string(),
        kfrag_id.to_string(),
        capsule_id.to_string(),
    );
    OWNER_CAPSULES.save(storage, capsule_key, capsule_data)?;
    save_index_status(
        storage,
        process_id,
        kfrag_id,
        capsule_id,
        CapsuleStatus::Error,
        timestamp,
    )?;

    Ok(())
}

fn process_capsule_submission(
    deps: DepsMut,
    env: Env,
    config: Config,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> Result<PipelineStatus, ContractError> {
    let storage = deps.storage;
    let timestamp = get_current_timestamp(&env);
    let process_id = config.process_id.clone();

    let kfrag_key = (process_id.clone(), kfrag_id.clone());
    let kfrag_blob = OWNER_KFRAGS
        .may_load(storage, kfrag_key)?
        .ok_or(ContractError::KFragNotFound {
            kfrag_id: kfrag_id.clone(),
        })?
        .kfrag;

    let idem_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if IDEM_FLAGS.has(storage, idem_key) {
        return Ok(PipelineStatus::NoOp);
    }

    let capsule_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let mut capsule_data = OwnerCapsuleData::new(capsule.clone(), timestamp.clone());
    OWNER_CAPSULES.save(storage, capsule_key.clone(), &capsule_data)?;
    save_index_status(
        storage,
        &process_id,
        &kfrag_id,
        &capsule_id,
        CapsuleStatus::Received,
        &timestamp,
    )?;

    capsule_data.update_status(CapsuleStatus::ReencInProgress, timestamp.clone());
    OWNER_CAPSULES.save(storage, capsule_key, &capsule_data)?;
    save_index_status(
        storage,
        &process_id,
        &kfrag_id,
        &capsule_id,
        CapsuleStatus::ReencInProgress,
        &timestamp,
    )?;

    match perform_reencryption(&kfrag_blob, &capsule) {
        Ok(cfrag_data) => {
            finalize_capsule_success(
                storage,
                &process_id,
                &kfrag_id,
                &capsule_id,
                &mut capsule_data,
                &timestamp,
                cfrag_data,
            )?;
            Ok(PipelineStatus::Success)
        }
        Err(err) => {
            mark_capsule_error(
                storage,
                &process_id,
                &kfrag_id,
                &capsule_id,
                &mut capsule_data,
                &timestamp,
            )?;
            Err(err)
        }
    }
}

fn retry_reencryption(
    deps: DepsMut,
    env: Env,
    config: Config,
    kfrag_id: String,
    capsule_id: String,
) -> Result<PipelineStatus, ContractError> {
    let storage = deps.storage;
    let timestamp = get_current_timestamp(&env);
    let process_id = config.process_id.clone();

    let idem_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if IDEM_FLAGS.has(storage, idem_key) {
        return Ok(PipelineStatus::NoOp);
    }

    let kfrag_key = (process_id.clone(), kfrag_id.clone());
    let kfrag_blob = OWNER_KFRAGS
        .may_load(storage, kfrag_key)?
        .ok_or(ContractError::KFragNotFound {
            kfrag_id: kfrag_id.clone(),
        })?
        .kfrag;

    let capsule_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let mut capsule_data = OWNER_CAPSULES
        .may_load(storage, capsule_key.clone())?
        .ok_or(ContractError::CapsuleNotFound {
            kfrag_id: kfrag_id.clone(),
            capsule_id: capsule_id.clone(),
        })?;

    let capsule_blob = capsule_data.capsule.clone();
    capsule_data.update_status(CapsuleStatus::ReencInProgress, timestamp.clone());
    OWNER_CAPSULES.save(storage, capsule_key, &capsule_data)?;
    save_index_status(
        storage,
        &process_id,
        &kfrag_id,
        &capsule_id,
        CapsuleStatus::ReencInProgress,
        &timestamp,
    )?;

    match perform_reencryption(&kfrag_blob, &capsule_blob) {
        Ok(cfrag_data) => {
            finalize_capsule_success(
                storage,
                &process_id,
                &kfrag_id,
                &capsule_id,
                &mut capsule_data,
                &timestamp,
                cfrag_data,
            )?;
            Ok(PipelineStatus::Success)
        }
        Err(err) => {
            mark_capsule_error(
                storage,
                &process_id,
                &kfrag_id,
                &capsule_id,
                &mut capsule_data,
                &timestamp,
            )?;
            Err(err)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StoredKeyFrag {
    id: u8,
    key_data: Vec<u8>,
    verification_data: Vec<u8>,
    #[serde(default)]
    precursor: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct StoredCFrag {
    fragment_id: u8,
    capsule_fragment: Vec<u8>,
    proof: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct VerificationData {
    verifying_pk: Vec<u8>,
    delegating_pk: Vec<u8>,
    receiving_pk: Vec<u8>,
}

fn perform_reencryption(
    kfrag_blob: &Binary,
    capsule_blob: &Binary,
) -> Result<Binary, ContractError> {
    if kfrag_blob.is_empty() {
        return Err(ContractError::ReencryptionFailed {
            reason: "Empty kFrag payload".to_string(),
        });
    }

    if capsule_blob.is_empty() {
        return Err(ContractError::ReencryptionFailed {
            reason: "Empty capsule payload".to_string(),
        });
    }

    let stored_kfrag: StoredKeyFrag =
        bincode::deserialize(kfrag_blob.as_slice()).map_err(|_| {
            ContractError::ReencryptionFailed {
                reason: "Failed to decode kFrag package".to_string(),
            }
        })?;

    if stored_kfrag.key_data.is_empty() {
        return Err(ContractError::ReencryptionFailed {
            reason: "Invalid kFrag data".to_string(),
        });
    }

    if stored_kfrag.verification_data.is_empty() {
        return Err(ContractError::ReencryptionFailed {
            reason: "Missing verification data".to_string(),
        });
    }

    let verification_bytes = stored_kfrag.verification_data.clone();
    let verification: VerificationData =
        bincode::deserialize(&verification_bytes).map_err(|_| {
            ContractError::ReencryptionFailed {
                reason: "Invalid verification payload".to_string(),
            }
        })?;

    let verifying_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.verifying_pk)
        .map_err(|_| ContractError::ReencryptionFailed {
            reason: "Invalid verifying key".to_string(),
        })?;

    let delegating_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.delegating_pk)
        .map_err(|_| ContractError::ReencryptionFailed {
            reason: "Invalid delegating key".to_string(),
        })?;

    let receiving_pk: umbral_pre::PublicKey = bincode::deserialize(&verification.receiving_pk)
        .map_err(|_| ContractError::ReencryptionFailed {
            reason: "Invalid receiving key".to_string(),
        })?;

    let key_frag = umbral_pre::KeyFrag::from_bytes(&stored_kfrag.key_data).map_err(|_| {
        ContractError::ReencryptionFailed {
            reason: "Failed to deserialize kFrag".to_string(),
        }
    })?;

    let verified_kfrag = key_frag
        .verify(&verifying_pk, Some(&delegating_pk), Some(&receiving_pk))
        .map_err(|_| ContractError::ReencryptionFailed {
            reason: "kFrag verification failed".to_string(),
        })?;

    let capsule: umbral_pre::Capsule =
        bincode::deserialize(capsule_blob.as_slice()).map_err(|_| {
            ContractError::ReencryptionFailed {
                reason: "Failed to deserialize capsule".to_string(),
            }
        })?;

    let verified_cfrag = umbral_pre::reencrypt(&capsule, verified_kfrag);
    let cfrag = verified_cfrag.unverify();
    let cfrag_bytes = cfrag
        .to_bytes()
        .map_err(|_| ContractError::ReencryptionFailed {
            reason: "Failed to serialize cFrag".to_string(),
        })?;

    let stored_cfrag = StoredCFrag {
        fragment_id: stored_kfrag.id,
        capsule_fragment: cfrag_bytes.to_vec(),
        proof: verification_bytes,
    };

    let serialized_cfrag =
        bincode::serialize(&stored_cfrag).map_err(|_| ContractError::ReencryptionFailed {
            reason: "Failed to encode cFrag".to_string(),
        })?;

    Ok(Binary::from(serialized_cfrag))
}

// --------------------- Reply ハンドラー ---------------------

pub fn handle_reply(deps: DepsMut, _env: Env, msg: Reply) -> ContractResult {
    match msg.id {
        REPLY_DELEGATE_KFRAG => handle_delegate_kfrag_reply(deps, msg),
        REPLY_DELEGATE_CAPSULE => handle_delegate_capsule_reply(deps, msg),
        _ => Err(ContractError::BadRequest {
            msg: format!("Unknown reply id: {}", msg.id),
        }),
    }
}

fn handle_delegate_kfrag_reply(_deps: DepsMut, msg: Reply) -> ContractResult {
    match msg.result {
        SubMsgResult::Err(err) => Ok(Response::new()
            .add_attribute("action", "delegate_kfrag_error")
            .add_attribute("error", err)),
        _ => Ok(Response::new().add_attribute("action", "delegate_kfrag_reply_ok")),
    }
}

fn handle_delegate_capsule_reply(_deps: DepsMut, msg: Reply) -> ContractResult {
    match msg.result {
        SubMsgResult::Err(err) => Ok(Response::new()
            .add_attribute("action", "delegate_capsule_error")
            .add_attribute("error", err)),
        _ => Ok(Response::new().add_attribute("action", "delegate_capsule_reply_ok")),
    }
}

// --------------------- 公開インターフェース ---------------------

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult {
    // メッセージバリデーション
    msg.validate()
        .map_err(|e| ContractError::ValidationError { msg: e })?;

    match msg {
        ExecuteMsg::DelegateKFrag { kfrag_id, kfrag } => {
            handle_delegate_kfrag(deps, env, info, kfrag_id, kfrag)
        }
        ExecuteMsg::DelegateCapsule {
            kfrag_id,
            capsule_id,
            capsule,
        } => handle_delegate_capsule(deps, env, info, kfrag_id, capsule_id, capsule),
        ExecuteMsg::SubmitKFrag { kfrag_id, kfrag } => {
            handle_submit_kfrag(deps, env, info, kfrag_id, kfrag)
        }
        ExecuteMsg::SubmitCapsule {
            kfrag_id,
            capsule_id,
            capsule,
        } => handle_submit_capsule(deps, env, info, kfrag_id, capsule_id, capsule),
        ExecuteMsg::Reencrypt {
            kfrag_id,
            capsule_id,
        } => handle_reencrypt(deps, env, info, kfrag_id, capsule_id),
    }
}

pub fn query_handler(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    msg.validate().map_err(|e| StdError::generic_err(e))?;

    match msg {
        QueryMsg::GetCFrag {
            kfrag_id,
            capsule_id,
        } => handle_get_cfrag(deps, env, kfrag_id, capsule_id),
        QueryMsg::ListCapsulesByKFrag {
            kfrag_id,
            start_after,
            limit,
        } => handle_list_capsules_by_kfrag(deps, env, kfrag_id, start_after, limit),
    }
}

// tests moved to `tests/`
