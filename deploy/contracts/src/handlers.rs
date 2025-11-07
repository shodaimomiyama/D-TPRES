use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
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
    get_current_timestamp, CapsuleStatus, HolderCFragData,
    IdemFlag, IndexCapsToCFragValue, IndexKFragToCapsValue, OwnerCapsuleData, OwnerKFragData,
    StorageError, CONFIG, DEFAULT_LIST_LIMIT, HOLDER_CFRAGS, IDEM_FLAGS,
    INDEX_CAPS_TO_CFRAG, INDEX_KFRAG_TO_CAPS, OWNER_CAPSULES, OWNER_KFRAGS,
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
    _info: MessageInfo,
    kfrag_id: String,
    capsule_id: String,
    capsule: Binary,
) -> ContractResult {
    let config = CONFIG.load(deps.storage)?;
    let timestamp = get_current_timestamp(&env);
    let process_id = config.process_id.clone();

    // 1. kFragの存在確認（順序固定）
    let kfrag_blob = OWNER_KFRAGS
        .may_load(deps.storage, (process_id.clone(), kfrag_id.clone()))?
        .ok_or(ContractError::KFragNotFound {
            kfrag_id: kfrag_id.clone(),
        })?
        .kfrag;

    // 2. 冪等性チェック
    let idem_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    if IDEM_FLAGS.has(deps.storage, idem_key.clone()) {
        // 冪等ONの場合は完全No-Op
        return Ok(Response::new()
            .add_attribute("action", "submit_capsule")
            .add_attribute("kfrag_id", kfrag_id)
            .add_attribute("capsule_id", capsule_id)
            .add_attribute("status", "no_op"));
    }

    // 3. Capsule本体保存
    let capsule_blob = capsule.clone();
    let capsule_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let mut capsule_data = OwnerCapsuleData::new(capsule, timestamp.clone());
    OWNER_CAPSULES.save(deps.storage, capsule_key.clone(), &capsule_data)?;

    // 4. 列挙用インデックス追加
    let index_kfrag_caps_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
    let index_value = IndexKFragToCapsValue::new(CapsuleStatus::Received, timestamp.clone());
    INDEX_KFRAG_TO_CAPS.save(deps.storage, index_kfrag_caps_key.clone(), &index_value)?;

    // 5. 状態遷移: REENC_IN_PROGRESS
    capsule_data.update_status(CapsuleStatus::ReencInProgress, timestamp.clone());
    OWNER_CAPSULES.save(deps.storage, capsule_key.clone(), &capsule_data)?;

    // 6. 再暗号化処理（umbral-pre）
    let cfrag_result = perform_reencryption(&kfrag_blob, &capsule_blob);

    match cfrag_result {
        Ok(cfrag_data) => {
            // 7. cFrag本体保存
            let cfrag_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
            let cfrag = HolderCFragData::new(cfrag_data, timestamp.clone());
            HOLDER_CFRAGS.save(deps.storage, cfrag_key, &cfrag)?;

            // 8. 存在インデックス追加
            let index_caps_cfrag_key = (process_id.clone(), kfrag_id.clone(), capsule_id.clone());
            let index_cfrag_value = IndexCapsToCFragValue::new(timestamp.clone());
            INDEX_CAPS_TO_CFRAG.save(deps.storage, index_caps_cfrag_key, &index_cfrag_value)?;

            // 9. 冪等フラグON
            let idem_flag = IdemFlag::new(timestamp.clone());
            IDEM_FLAGS.save(deps.storage, idem_key, &idem_flag)?;

            // 10. 最終状態更新
            capsule_data.update_status(CapsuleStatus::CFragReady, timestamp.clone());
            OWNER_CAPSULES.save(deps.storage, capsule_key, &capsule_data)?;

            // 11. インデックス状態も更新
            let updated_index_value =
                IndexKFragToCapsValue::new(CapsuleStatus::CFragReady, timestamp);
            INDEX_KFRAG_TO_CAPS.save(deps.storage, index_kfrag_caps_key, &updated_index_value)?;

            Ok(Response::new()
                .add_attribute("action", "submit_capsule")
                .add_attribute("kfrag_id", kfrag_id)
                .add_attribute("capsule_id", capsule_id)
                .add_attribute("status", "success"))
        }
        Err(err) => {
            // 再暗号化失敗時
            capsule_data.update_status(CapsuleStatus::Error, timestamp.clone());
            OWNER_CAPSULES.save(deps.storage, capsule_key, &capsule_data)?;


            Err(err)
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
    let idem_key = (
        config.process_id.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
    );
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
    let index_key = (
        config.process_id.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
    );
    if !INDEX_CAPS_TO_CFRAG.has(deps.storage, index_key) {
        return Err(StdError::not_found("CFrag not ready"));
    }

    // 2. cFrag本体取得
    let cfrag_key = (
        config.process_id.clone(),
        kfrag_id.clone(),
        capsule_id.clone(),
    );
    let cfrag_data = HOLDER_CFRAGS.load(deps.storage, cfrag_key)?;


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
    use crate::state::Config;
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

    #[test]
    fn test_submit_capsule_success_with_reencryption() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // Configの初期化
        let config = Config {
            process_id: "test_process".to_string(),
            default_list_limit: 50,
        };
        CONFIG.save(&mut deps.storage, &config).unwrap();

        // Umbral鍵セットアップ
        let delegating_sk = umbral_pre::SecretKey::random();
        let delegating_pk = delegating_sk.public_key();
        let receiving_sk = umbral_pre::SecretKey::random();
        let receiving_pk = receiving_sk.public_key();
        let signing_sk = umbral_pre::SecretKey::random();
        let verifying_pk = signing_sk.public_key();
        let signer = umbral_pre::Signer::new(signing_sk);

        // kFrag生成
        let verified_kfrags =
            umbral_pre::generate_kfrags(&delegating_sk, &receiving_pk, &signer, 1, 1, true, true);
        let kfrag = verified_kfrags
            .first()
            .expect("kFrag generation failed")
            .clone()
            .unverify();
        let kfrag_bytes = kfrag
            .to_bytes()
            .expect("kFrag serialization failed")
            .to_vec();

        let verification = VerificationData {
            verifying_pk: bincode::serialize(&verifying_pk).unwrap(),
            delegating_pk: bincode::serialize(&delegating_pk).unwrap(),
            receiving_pk: bincode::serialize(&receiving_pk).unwrap(),
        };
        let verification_bytes = bincode::serialize(&verification).unwrap();

        let stored_kfrag = StoredKeyFrag {
            id: 0,
            key_data: kfrag_bytes,
            verification_data: verification_bytes.clone(),
            precursor: vec![],
        };
        let serialized_kfrag = bincode::serialize(&stored_kfrag).unwrap();

        // kFrag登録
        let kfrag_info = mock_info("sender", &[]);
        handle_submit_kfrag(
            deps.as_mut(),
            env.clone(),
            kfrag_info,
            "kfrag1".to_string(),
            Binary::from(serialized_kfrag),
        )
        .unwrap();

        // Capsule生成
        let plaintext = b"secret payload".to_vec();
        let (capsule_struct, ciphertext_box) =
            umbral_pre::encrypt(&delegating_pk, &plaintext).expect("encryption failed");
        let capsule_bytes = bincode::serialize(&capsule_struct).unwrap();
        let ciphertext = ciphertext_box.to_vec();

        // Capsule送信
        let capsule_info = mock_info("sender", &[]);
        let response = handle_submit_capsule(
            deps.as_mut(),
            env.clone(),
            capsule_info,
            "kfrag1".to_string(),
            "capsule1".to_string(),
            Binary::from(capsule_bytes.clone()),
        )
        .expect("submit_capsule should succeed");

        assert_eq!(
            response
                .attributes
                .iter()
                .find(|a| a.key == "status")
                .unwrap()
                .value,
            "success"
        );

        // cFrag取得・検証
        let cfrag_key = (
            "test_process".to_string(),
            "kfrag1".to_string(),
            "capsule1".to_string(),
        );
        let stored_cfrag_data = HOLDER_CFRAGS
            .load(&deps.storage, cfrag_key)
            .expect("cFrag should be stored");
        let stored_cfrag: StoredCFrag =
            bincode::deserialize(stored_cfrag_data.cfrag.as_slice()).unwrap();

        assert_eq!(stored_cfrag.fragment_id, 0);
        assert_eq!(stored_cfrag.proof, verification_bytes);

        let capsule_frag = umbral_pre::CapsuleFrag::from_bytes(&stored_cfrag.capsule_fragment)
            .expect("capsule fragment decode failed");
        let verified_cfrag = capsule_frag
            .verify(
                &capsule_struct,
                &verifying_pk,
                &delegating_pk,
                &receiving_pk,
            )
            .expect("capsule fragment verification failed");

        let recovered = umbral_pre::decrypt_reencrypted(
            &receiving_sk,
            &delegating_pk,
            &capsule_struct,
            vec![verified_cfrag],
            ciphertext.as_slice(),
        )
        .expect("decrypt reencrypted failed");

        assert_eq!(recovered.to_vec(), plaintext);

        // Capsuleステータス確認
        let capsule_state = OWNER_CAPSULES
            .load(
                &deps.storage,
                (
                    "test_process".to_string(),
                    "kfrag1".to_string(),
                    "capsule1".to_string(),
                ),
            )
            .expect("capsule state should exist");
        assert!(matches!(capsule_state.status, CapsuleStatus::CFragReady));
    }
}
