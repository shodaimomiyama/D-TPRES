use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Storage,
};
use cw_storage_plus::Bound;

use crate::msg::{
    CapsuleInfo, ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, QueryMsg, ValidateMessage,
};
use crate::state::{
    generate_audit_key, get_current_timestamp, AuditLogEntry, CapsuleStatus, Config, HolderCFragData,
    IdemFlag, IndexCapsToCFragValue, IndexKFragToCapsValue, OwnerCapsuleData, OwnerKFragData,
    StorageError, AUDIT_LOGS, CONFIG, HOLDER_CFRAGS, IDEM_FLAGS, INDEX_CAPS_TO_CFRAG,
    INDEX_KFRAG_TO_CAPS, OWNER_CAPSULES, OWNER_KFRAGS, DEFAULT_LIST_LIMIT,
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

    #[error("KFrag not found: {kfrag_id}")]
    KFragNotFound { kfrag_id: String },

    #[error("CFrag not ready: {kfrag_id}/{capsule_id}")]
    CFragNotReady {
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

// --------------------- Execute ハンドラー ---------------------

pub fn handle_submit_kfrag(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    kfrag: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let timestamp = get_current_timestamp(&env);

    // kFragIDの重複チェック
    let kfrag_key = (config.process_id.clone(), kfrag_id.clone());
    if OWNER_KFRAGS.has(deps.storage, kfrag_key.clone()) {
        // 既存の場合はNo-Op
        return Ok(Response::new()
            .add_attribute("action", "submit_kfrag")
            .add_attribute("kfrag_id", kfrag_id)
            .add_attribute("status", "no_op"));
    }

    // kFragデータ作成・保存
    let kfrag_data = OwnerKFragData::new(kfrag, timestamp);
    OWNER_KFRAGS.save(deps.storage, kfrag_key, &kfrag_data)?;

    Ok(Response::new()
        .add_attribute("action", "submit_kfrag")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("process_id", config.process_id)
        .add_attribute("status", "success"))
}

pub fn handle_submit_capsule(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let timestamp = get_current_timestamp(&env);

    // 1. kFragの存在確認（順序固定）
    let kfrag_key = (config.process_id.clone(), kfrag_id.clone());
    if !OWNER_KFRAGS.has(deps.storage, kfrag_key) {
        return Err(ContractError::KFragNotFound {
            kfrag_id: kfrag_id.clone(),
        });
    }

    // 2. 冪等性チェック
    let idem_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if IDEM_FLAGS.has(deps.storage, idem_key.clone()) {
        // 冪等ONの場合は完全No-Op
        return Ok(Response::new()
            .add_attribute("action", "submit_capsule")
            .add_attribute("kfrag_id", kfrag_id)
            .add_attribute("capsule_id", capsule_id)
            .add_attribute("status", "no_op"));
    }

    // 3. Capsule本体保存
    let capsule_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let mut capsule_data = OwnerCapsuleData::new(capsule, timestamp.clone());
    OWNER_CAPSULES.save(deps.storage, capsule_key.clone(), &capsule_data)?;

    // 4. 列挙用インデックス追加
    let index_kfrag_caps_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let index_value = IndexKFragToCapsValue::new(CapsuleStatus::Received, timestamp.clone());
    INDEX_KFRAG_TO_CAPS.save(deps.storage, index_kfrag_caps_key.clone(), &index_value)?;

    // 5. 状態遷移: REENC_IN_PROGRESS
    capsule_data.update_status(CapsuleStatus::ReencInProgress, timestamp.clone());
    OWNER_CAPSULES.save(deps.storage, capsule_key.clone(), &capsule_data)?;

    // 6. 再暗号化処理（モック実装）
    let cfrag_result = perform_reencryption(&kfrag_id, &capsule_id);

    match cfrag_result {
        Ok(cfrag_data) => {
            // 7. cFrag本体保存
            let cfrag_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
            let cfrag = HolderCFragData::new(cfrag_data, timestamp.clone());
            HOLDER_CFRAGS.save(deps.storage, cfrag_key, &cfrag)?;

            // 8. 存在インデックス追加
            let index_caps_cfrag_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
            let index_cfrag_value = IndexCapsToCFragValue::new(timestamp.clone());
            INDEX_CAPS_TO_CFRAG.save(deps.storage, index_caps_cfrag_key, &index_cfrag_value)?;

            // 9. 冪等フラグON
            let idem_flag = IdemFlag::new(timestamp.clone());
            IDEM_FLAGS.save(deps.storage, idem_key, &idem_flag)?;

            // 10. 最終状態更新
            capsule_data.update_status(CapsuleStatus::CFragReady, timestamp.clone());
            OWNER_CAPSULES.save(deps.storage, capsule_key, &capsule_data)?;

            // 11. インデックス状態も更新
            let updated_index_value = IndexKFragToCapsValue::new(CapsuleStatus::CFragReady, timestamp);
            INDEX_KFRAG_TO_CAPS.save(deps.storage, index_kfrag_caps_key, &updated_index_value)?;

            Ok(Response::new()
                .add_attribute("action", "submit_capsule")
                .add_attribute("kfrag_id", kfrag_id)
                .add_attribute("capsule_id", capsule_id)
                .add_attribute("status", "success"))
        }
        Err(reason) => {
            // 再暗号化失敗時
            capsule_data.update_status(CapsuleStatus::Error, timestamp.clone());
            OWNER_CAPSULES.save(deps.storage, capsule_key, &capsule_data)?;

            // 監査ログ追加
            let actor = info.sender.to_string();
            let ext_ts = timestamp; // 実際の実装では外部時刻を使用
            log_audit_event(
                deps.storage,
                "ERROR".to_string(),
                actor,
                kfrag_id.clone(),
                capsule_id.clone(),
                reason.clone(),
                ext_ts,
                &config.process_id,
            )?;

            Err(ContractError::ReencryptionFailed { reason })
        }
    }
}

pub fn handle_reencrypt(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;

    // 冪等性チェック（既に成功している場合はNo-Op）
    let idem_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if IDEM_FLAGS.has(deps.storage, idem_key) {
        return Ok(Response::new()
            .add_attribute("action", "reencrypt")
            .add_attribute("kfrag_id", kfrag_id)
            .add_attribute("capsule_id", capsule_id)
            .add_attribute("status", "no_op"));
    }

    // handle_submit_capsuleの4〜7を再実行
    // 実際の実装では必要な処理のみを抽出

    Ok(Response::new()
        .add_attribute("action", "reencrypt")
        .add_attribute("kfrag_id", kfrag_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("status", "retry"))
}

// --------------------- Query ハンドラー ---------------------

pub fn handle_get_cfrag(
    deps: Deps,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let _timestamp = get_current_timestamp(&env);

    // 1. 存在判定（O(1)）
    let index_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if !INDEX_CAPS_TO_CFRAG.has(deps.storage, index_key) {
        return Err(StdError::not_found("CFrag not ready"));
    }

    // 2. cFrag本体取得
    let cfrag_key = (config.process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let cfrag_data = HOLDER_CFRAGS.load(deps.storage, cfrag_key)?;

    // 3. 監査ログ追加（非可変処理なので実際の実装では別途処理）
    // log_audit_event_readonly(...)

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
    let limit = limit.unwrap_or(DEFAULT_LIST_LIMIT).min(100) as usize;

    // プレフィックス決定
    let prefix = (config.process_id.clone(), kfrag_id.clone());

    // 開始位置の決定
    let start_bound = start_after.map(|s| Bound::exclusive(s));

    // レンジ走査
    let capsules: Result<Vec<CapsuleInfo>, StdError> = INDEX_KFRAG_TO_CAPS
        .prefix(prefix)
        .range(deps.storage, start_bound, None, cosmwasm_std::Order::Ascending)
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

fn perform_reencryption(kfrag_id: &str, _capsule_id: &str) -> Result<Binary, String> {
    // モック実装：実際のumbral-preライブラリを使用した再暗号化
    // 現在はダミーデータを返す
    if kfrag_id == "fail_test" {
        return Err("Mock reencryption failure".to_string());
    }

    // 32バイトのモックcFragデータ
    let mock_cfrag = vec![0u8; 32];
    Ok(Binary::from(mock_cfrag))
}

fn log_audit_event(
    storage: &mut dyn Storage,
    event: String,
    actor_wallet: String,
    kfrag_id: String,
    capsule_id: String,
    reason_or_meta: String,
    ext_ts: String,
    process_id: &str,
) -> StdResult<()> {
    let audit_key = generate_audit_key(&ext_ts, &event, process_id, &kfrag_id, &capsule_id);
    let audit_entry = AuditLogEntry::new(
        event,
        actor_wallet,
        kfrag_id,
        capsule_id,
        reason_or_meta,
        ext_ts,
    );

    AUDIT_LOGS.save(storage, audit_key, &audit_entry)?;
    Ok(())
}

// --------------------- 公開インターフェース ---------------------

pub fn execute_handler(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult {
    // メッセージバリデーション
    msg.validate().map_err(|e| ContractError::ValidationError { msg: e })?;

    match msg {
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

pub fn query_handler(deps: Deps, env: Env, info: MessageInfo, msg: QueryMsg) -> StdResult<Binary> {
    // メッセージバリデーション
    msg.validate().map_err(|e| StdError::generic_err(e))?;

    match msg {
        QueryMsg::GetCFrag {
            kfrag_id,
            capsule_id,
        } => handle_get_cfrag(deps, env, info, kfrag_id, capsule_id),
        QueryMsg::ListCapsulesByKFrag {
            kfrag_id,
            start_after,
            limit,
        } => handle_list_capsules_by_kfrag(deps, env, kfrag_id, start_after, limit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Binary;

    #[test]
    fn test_submit_kfrag_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Configの初期化
        let config = Config {
            process_id: "test_process".to_string(),
            chunk_threshold: 4096,
            max_chunks: 32,
            default_list_limit: 50,
        };
        CONFIG.save(&mut deps.storage, &config).unwrap();

        let result = handle_submit_kfrag(
            deps.as_mut(),
            env,
            info,
            "test_kfrag".to_string(),
            Binary::from(b"test_kfrag_data"),
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_submit_capsule_kfrag_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // Configの初期化
        let config = Config {
            process_id: "test_process".to_string(),
            chunk_threshold: 4096,
            max_chunks: 32,
            default_list_limit: 50,
        };
        CONFIG.save(&mut deps.storage, &config).unwrap();

        let result = handle_submit_capsule(
            deps.as_mut(),
            env,
            info,
            "nonexistent_kfrag".to_string(),
            "test_capsule".to_string(),
            Binary::from(b"test_capsule_data"),
        );

        assert!(matches!(result, Err(ContractError::KFragNotFound { .. })));
    }
}