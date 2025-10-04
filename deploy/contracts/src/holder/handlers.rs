use cosmwasm_std::{
    Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    to_json_binary, Binary, Timestamp
};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::contract::ContractError;
use crate::msg::{
    KFragReceiptData, CapsuleStorageData, ValidateMessage,
    HolderMetadataResponse, CFragsResponse, CFragInfo,
    CachedCapsulesResponse, CapsuleInfo, KFragInfo
};
use crate::state::{
    HolderKFragData, HolderCFragData, CapsuleData,
    HOLDER_METADATA, HOLDER_KFRAGS, HOLDER_CFRAGS, CAPSULE_CACHE
};

use crate::holder::storage::{
    load_holder_state, save_holder_state, store_kfrag_data, get_kfrag_by_id,
    get_all_kfrags, get_processed_kfrags, get_unprocessed_kfrags,
    store_capsule_data, get_capsule_by_id, get_cached_capsules,
    store_cfrag_data, get_cfrag_by_id, get_cfrags_by_kfrag, get_all_cfrags,
    mark_kfrag_processed, update_cfrag_arweave_txid, verify_holder_role,
    get_holder_statistics, HolderStatistics
};

/// Holder-Process専用メッセージハンドラ
///
/// このモジュールは Holder-Process のみが処理するメッセージハンドラを提供します。
/// PRDのPHASE 2-3, 2-4, 2-5, 2-6 に対応した実装を行います。

/// kFrag受信処理の結果
#[derive(Debug, Zeroize, ZeroizeOnDrop)]
pub struct KFragReceiptResult {
    pub kfrag_id: String,
    #[zeroize(skip)]
    pub source_owner: String,
    #[zeroize(skip)]
    pub verified: bool,
    #[zeroize(skip)]
    pub received_at: u64,
}

/// cFrag生成処理の結果
#[derive(Debug)]
pub struct CFragGenerationResult {
    pub cfrag_id: String,
    pub source_kfrag: String,
    pub generation_time: u64,
    pub arweave_txid: Option<String>,
}

/// Capsule保存処理の結果
#[derive(Debug)]
pub struct CapsuleStorageResult {
    pub capsule_id: String,
    pub cached_at: u64,
    pub cache_size_bytes: usize,
}

/// Holder-Process: kFrag受信ハンドラ（PHASE 2-3）
///
/// PRD仕様:
/// - Owner-Processからの署名付きkFragを受信
/// - 署名を検証してkFragの真正性を確認
/// - 検証済みkFragをローカルストレージに保存
pub fn handle_receive_kfrag(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_data: KFragReceiptData,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_holder_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Holder".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 入力データのバリデーション
    kfrag_data.validate()
        .map_err(|e| ContractError::ValidationError { msg: e })?;

    // 3. 現在の状態をロード
    let current_state = load_holder_state(deps.as_ref(), env.block.time)
        .map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 4. 重複受信チェック
    if let Ok(_existing_kfrag) = get_kfrag_by_id(deps.as_ref(), kfrag_data.id.clone()) {
        return Err(ContractError::DuplicateKFrag {
            id: kfrag_data.id,
        });
    }

    // 5. Owner認証（許可されたOwnerからの送信かチェック）
    if !current_state.metadata.assigned_owners.contains(&kfrag_data.owner_id) {
        return Err(ContractError::UnauthorizedOwner {
            owner_id: kfrag_data.owner_id,
        });
    }

    // 6. kFrag署名検証（モック実装）
    let signature_valid = verify_kfrag_signature(
        &kfrag_data.encrypted_kfrag,
        &kfrag_data.signature,
        &kfrag_data.owner_id,
    )?;

    if !signature_valid {
        return Err(ContractError::InvalidSignature {
            context: "kFrag signature verification failed".to_string(),
        });
    }

    // 7. kFragをストレージに保存
    let storage_result = store_kfrag_data(
        deps.branch(),
        kfrag_data.id.clone(),
        kfrag_data.owner_id.clone(),
        kfrag_data.encrypted_kfrag,
        kfrag_data.signature,
        env.block.time,
    ).map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 8. 受信結果の構築
    let receipt_result = KFragReceiptResult {
        kfrag_id: kfrag_data.id.clone(),
        source_owner: kfrag_data.owner_id,
        verified: signature_valid,
        received_at: env.block.time.seconds(),
    };

    // 9. レスポンス構築
    let response = Response::new()
        .add_attribute("action", "receive_kfrag")
        .add_attribute("kfrag_id", &kfrag_data.id)
        .add_attribute("source_owner", &receipt_result.source_owner)
        .add_attribute("verified", signature_valid.to_string())
        .add_attribute("received_at", receipt_result.received_at.to_string())
        .add_attribute("affected_records", storage_result.affected_records.to_string());

    Ok(response)
}

/// Holder-Process: cFrag生成ハンドラ（PHASE 2-4, 2-5）
///
/// PRD仕様:
/// - 指定されたkFragを取得・復号
/// - ArweaveからCapsuleデータを取得（キャッシュ利用）
/// - cFrag = PRE_ReEnc(kFrag, Capsule) を実行
/// - 生成したcFragをローカル保存 + Arweave永続化
pub fn handle_generate_cfrag(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    kfrag_id: String,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_holder_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Holder".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 入力バリデーション
    if kfrag_id.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "KFrag ID cannot be empty".to_string(),
        });
    }

    // 3. kFragの取得と検証
    let kfrag_data = get_kfrag_by_id(deps.as_ref(), kfrag_id.clone())
        .map_err(|_| ContractError::KFragNotFound { id: kfrag_id.clone() })?;

    if kfrag_data.processed {
        return Err(ContractError::KFragAlreadyProcessed {
            id: kfrag_id,
        });
    }

    // 4. Capsuleデータの取得（まずキャッシュから）
    let _metadata = HOLDER_METADATA.load(deps.storage)?;
    let capsule_id = format!("capsule_for_{}", kfrag_data.source_owner);

    let capsule_data = match get_capsule_by_id(deps.as_ref(), capsule_id.clone()) {
        Ok(cached_capsule) => {
            // キャッシュヒット
            cached_capsule
        }
        Err(_) => {
            // キャッシュミス - Arweaveから取得（モック）
            let fetched_capsule = fetch_capsule_from_arweave(&capsule_id)?;

            // キャッシュに保存
            store_capsule_data(
                deps.branch(),
                capsule_id.clone(),
                fetched_capsule.capsule_bytes.clone(),
                fetched_capsule.arweave_txid.clone(),
                env.block.time,
            ).map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

            fetched_capsule
        }
    };

    // 5. cFrag生成（PRE_ReEnc）
    let cfrag_id = format!("cfrag_{}_{}", kfrag_id, env.block.time.seconds());
    let cfrag_data = perform_reencryption(
        &kfrag_data.encrypted_kfrag,
        &capsule_data.capsule_bytes,
    )?;

    // 6. cFragをローカルストレージに保存
    let storage_result = store_cfrag_data(
        deps.branch(),
        cfrag_id.clone(),
        kfrag_id.clone(),
        cfrag_data.clone(),
        env.block.time,
        None, // Arweave TXID は後で更新
    ).map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 7. cFragをArweaveに保存（モック）
    let arweave_txid = store_cfrag_to_arweave(&cfrag_id, &cfrag_data)?;

    // 8. Arweave TXID を更新
    update_cfrag_arweave_txid(deps.branch(), cfrag_id.clone(), arweave_txid.clone())
        .map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 9. kFragを処理済みとしてマーク
    mark_kfrag_processed(deps.branch(), kfrag_id.clone())
        .map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 10. 生成結果の構築
    let generation_result = CFragGenerationResult {
        cfrag_id: cfrag_id.clone(),
        source_kfrag: kfrag_id,
        generation_time: env.block.time.seconds(),
        arweave_txid: Some(arweave_txid.clone()),
    };

    // 11. レスポンス構築
    let response = Response::new()
        .add_attribute("action", "generate_cfrag")
        .add_attribute("cfrag_id", &cfrag_id)
        .add_attribute("source_kfrag", &generation_result.source_kfrag)
        .add_attribute("generation_time", generation_result.generation_time.to_string())
        .add_attribute("arweave_txid", &arweave_txid)
        .add_attribute("affected_records", storage_result.affected_records.to_string());

    Ok(response)
}

/// Holder-Process: Capsule保存ハンドラ（PHASE 2-6）
///
/// PRD仕様:
/// - ArweaveからCapsuleデータを取得
/// - ローカルキャッシュに効率的に保存
/// - キャッシュ管理とクリーンアップ
pub fn handle_store_capsule(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    capsule_data: CapsuleStorageData,
) -> Result<Response, ContractError> {
    // 1. プロセス役割検証
    verify_holder_role(deps.as_ref())
        .map_err(|_| ContractError::UnauthorizedRole {
            expected: "Holder".to_string(),
            actual: "Unknown".to_string(),
        })?;

    // 2. 入力データのバリデーション
    if capsule_data.capsule_id.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "Capsule ID cannot be empty".to_string(),
        });
    }

    if capsule_data.capsule_bytes.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "Capsule data cannot be empty".to_string(),
        });
    }

    if capsule_data.arweave_txid.is_empty() {
        return Err(ContractError::ValidationError {
            msg: "Arweave TXID cannot be empty".to_string(),
        });
    }

    // 3. 重複チェック
    if let Ok(_existing_capsule) = get_capsule_by_id(deps.as_ref(), capsule_data.capsule_id.clone()) {
        return Err(ContractError::DuplicateCapsule {
            id: capsule_data.capsule_id,
        });
    }

    // 4. Capsuleデータをキャッシュに保存
    let storage_result = store_capsule_data(
        deps.branch(),
        capsule_data.capsule_id.clone(),
        capsule_data.capsule_bytes.clone(),
        capsule_data.arweave_txid.clone(),
        env.block.time,
    ).map_err(|e| ContractError::StorageError { msg: e.to_string() })?;

    // 5. 保存結果の構築
    let capsule_result = CapsuleStorageResult {
        capsule_id: capsule_data.capsule_id.clone(),
        cached_at: env.block.time.seconds(),
        cache_size_bytes: capsule_data.capsule_bytes.len(),
    };

    // 6. レスポンス構築
    let response = Response::new()
        .add_attribute("action", "store_capsule")
        .add_attribute("capsule_id", &capsule_data.capsule_id)
        .add_attribute("arweave_txid", &capsule_data.arweave_txid)
        .add_attribute("cached_at", capsule_result.cached_at.to_string())
        .add_attribute("cache_size_bytes", capsule_result.cache_size_bytes.to_string())
        .add_attribute("affected_records", storage_result.affected_records.to_string());

    Ok(response)
}

/// Holder-Process: メタデータクエリ
pub fn query_holder_metadata(deps: Deps) -> StdResult<Binary> {
    let metadata = HOLDER_METADATA.load(deps.storage)?;

    let response = HolderMetadataResponse {
        metadata,
    };

    to_json_binary(&response)
}

/// Holder-Process: kFrag情報クエリ（ID指定）
pub fn query_kfrag_by_id(deps: Deps, kfrag_id: String) -> StdResult<Binary> {
    let kfrag = get_kfrag_by_id(deps, kfrag_id)?;

    // セキュリティ: 暗号化データと署名は返さない
    let kfrag_info = KFragInfo {
        kfrag_id: kfrag.kfrag_id.clone(),
        target_holder: kfrag.source_owner.clone(),
        created_at: kfrag.received_at,
        distributed: kfrag.processed,
    };

    to_json_binary(&kfrag_info)
}

/// Holder-Process: cFrag情報クエリ
pub fn query_cfrags(deps: Deps, processed_only: Option<bool>) -> StdResult<Binary> {
    let all_cfrags = get_all_cfrags(deps)?;

    let cfrags: Vec<CFragInfo> = all_cfrags
        .into_iter()
        .filter(|cfrag| {
            match processed_only {
                Some(true) => cfrag.arweave_txid.is_some(),
                Some(false) => cfrag.arweave_txid.is_none(),
                None => true,
            }
        })
        .map(|cfrag| CFragInfo {
            cfrag_id: cfrag.cfrag_id,
            source_kfrag: cfrag.source_kfrag,
            generated_at: cfrag.generated_at,
            arweave_txid: cfrag.arweave_txid,
        })
        .collect();

    let response = CFragsResponse { cfrags };
    to_json_binary(&response)
}

/// Holder-Process: キャッシュされたCapsule情報クエリ
pub fn query_cached_capsules(deps: Deps) -> StdResult<Binary> {
    let cached_capsules = get_cached_capsules(deps)?;

    let capsules: Vec<CapsuleInfo> = cached_capsules
        .into_iter()
        .map(|capsule| CapsuleInfo {
            capsule_id: capsule.capsule_id,
            arweave_txid: capsule.arweave_txid,
            cached_at: capsule.cached_at,
        })
        .collect();

    let response = CachedCapsulesResponse { capsules };
    to_json_binary(&response)
}

/// Holder-Process: 統計情報クエリ
pub fn query_holder_statistics(deps: Deps) -> StdResult<Binary> {
    let current_time = Timestamp::from_seconds(0); // クエリ時は現在時刻不要
    let stats = get_holder_statistics(deps, current_time)?;

    to_json_binary(&stats)
}

/// kFrag署名検証（モック実装）
fn verify_kfrag_signature(
    encrypted_kfrag: &[u8],
    signature: &[u8],
    owner_id: &str,
) -> Result<bool, ContractError> {
    // 実際の実装では、Owner-Processの公開鍵を使用して署名を検証
    // ここではモック実装として基本的な検証のみ

    if encrypted_kfrag.is_empty() || signature.is_empty() || owner_id.is_empty() {
        return Ok(false);
    }

    // 長さベースの簡易検証（実際は楕円曲線署名検証を使用）
    let signature_valid = signature.len() >= 32 && // 最小署名長
                         encrypted_kfrag.len() >= 16 && // 最小暗号化データ長
                         !owner_id.is_empty();

    Ok(signature_valid)
}

/// Arweaveからのcapsule取得（モック実装）
fn fetch_capsule_from_arweave(capsule_id: &str) -> Result<CapsuleData, ContractError> {
    // 実際の実装では、Arweave GraphQLクエリでCapsuleデータを取得
    // ここではモック実装

    let mock_capsule = CapsuleData {
        capsule_id: capsule_id.to_string(),
        capsule_bytes: vec![0x01, 0x02, 0x03, 0x04], // モックのCapsuleデータ
        arweave_txid: format!("arweave_tx_{}", capsule_id),
        cached_at: 0, // フェッチ時は0、キャッシュ保存時に更新
    };

    Ok(mock_capsule)
}

/// 再暗号化処理（モック実装）
fn perform_reencryption(
    encrypted_kfrag: &[u8],
    capsule_bytes: &[u8],
) -> Result<Vec<u8>, ContractError> {
    // 実際の実装では、Umbral-pre ライブラリを使用
    // cFrag = PRE_ReEnc(kFrag, Capsule)
    // ここではモック実装

    if encrypted_kfrag.is_empty() || capsule_bytes.is_empty() {
        return Err(ContractError::ReencryptionFailed {
            reason: "Empty input data".to_string(),
        });
    }

    // モックの再暗号化処理
    let mut cfrag_data = Vec::new();
    cfrag_data.extend_from_slice(encrypted_kfrag);
    cfrag_data.extend_from_slice(capsule_bytes);

    // 簡易的なハッシュ処理でcFragを生成
    cfrag_data.reverse();

    Ok(cfrag_data)
}

/// ArweaveへのcFrag保存（モック実装）
fn store_cfrag_to_arweave(cfrag_id: &str, cfrag_data: &[u8]) -> Result<String, ContractError> {
    // 実際の実装では、Arweave APIを使用してcFragデータを永続保存
    // ここではモック実装

    if cfrag_id.is_empty() || cfrag_data.is_empty() {
        return Err(ContractError::ArweaveStorageError {
            reason: "Empty data cannot be stored".to_string(),
        });
    }

    // モックのArweave TXID生成
    let arweave_txid = format!("arweave_cfrag_{}_{}",
                              cfrag_id,
                              cfrag_data.len());

    Ok(arweave_txid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use crate::state::{HolderMetadata, ProcessRole};

    fn setup_test_holder_process(deps: DepsMut) {
        let metadata = HolderMetadata {
            holder_id: "test_holder".to_string(),
            process_role: ProcessRole::Holder,
            assigned_owners: vec!["owner_1".to_string(), "owner_2".to_string()],
            initialization_time: 1000,
        };

        HOLDER_METADATA.save(deps.storage, &metadata).unwrap();
    }

    #[test]
    fn test_handle_receive_kfrag() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("owner_1", &[]);

        setup_test_holder_process(deps.as_mut());

        let kfrag_data = KFragReceiptData {
            id: "kfrag_1".to_string(),
            encrypted_kfrag: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            signature: vec![1; 64], // 64バイトの署名
            owner_id: "owner_1".to_string(),
        };

        let result = handle_receive_kfrag(deps.as_mut(), env, info, kfrag_data);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.attributes.iter().any(|attr| attr.key == "action" && attr.value == "receive_kfrag"));
    }

    #[test]
    fn test_handle_receive_kfrag_unauthorized_owner() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("unauthorized_owner", &[]);

        setup_test_holder_process(deps.as_mut());

        let kfrag_data = KFragReceiptData {
            id: "kfrag_1".to_string(),
            encrypted_kfrag: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
            signature: vec![1; 64],
            owner_id: "unauthorized_owner".to_string(),
        };

        let result = handle_receive_kfrag(deps.as_mut(), env, info, kfrag_data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ContractError::UnauthorizedOwner { .. }));
    }

    #[test]
    fn test_handle_store_capsule() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("system", &[]);

        setup_test_holder_process(deps.as_mut());

        let capsule_data = CapsuleStorageData {
            capsule_id: "capsule_1".to_string(),
            capsule_bytes: vec![9, 10, 11, 12],
            arweave_txid: "arweave_tx_1".to_string(),
        };

        let result = handle_store_capsule(deps.as_mut(), env, info, capsule_data);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.attributes.iter().any(|attr| attr.key == "action" && attr.value == "store_capsule"));
    }

    #[test]
    fn test_query_holder_metadata() {
        let mut deps = mock_dependencies();
        setup_test_holder_process(deps.as_mut());

        let result = query_holder_metadata(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_query_cached_capsules() {
        let mut deps = mock_dependencies();
        setup_test_holder_process(deps.as_mut());

        let result = query_cached_capsules(deps.as_ref());
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_kfrag_signature() {
        let encrypted_kfrag = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let signature = vec![1; 64];
        let owner_id = "test_owner";

        let result = verify_kfrag_signature(&encrypted_kfrag, &signature, owner_id);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_perform_reencryption() {
        let encrypted_kfrag = vec![1, 2, 3, 4];
        let capsule_bytes = vec![5, 6, 7, 8];

        let result = perform_reencryption(&encrypted_kfrag, &capsule_bytes);
        assert!(result.is_ok());

        let cfrag_data = result.unwrap();
        assert!(!cfrag_data.is_empty());
        assert_eq!(cfrag_data.len(), encrypted_kfrag.len() + capsule_bytes.len());
    }

    #[test]
    fn test_store_cfrag_to_arweave() {
        let cfrag_id = "cfrag_1";
        let cfrag_data = vec![1, 2, 3, 4];

        let result = store_cfrag_to_arweave(cfrag_id, &cfrag_data);
        assert!(result.is_ok());

        let arweave_txid = result.unwrap();
        assert!(arweave_txid.starts_with("arweave_cfrag_"));
    }
}