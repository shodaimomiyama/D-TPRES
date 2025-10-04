use cosmwasm_std::{Deps, DepsMut, StdError, StdResult, Timestamp};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::state::{
    HolderKFragData, HolderCFragData, CapsuleData, HolderMetadata,
    HOLDER_KFRAGS, HOLDER_CFRAGS, CAPSULE_CACHE, HOLDER_METADATA,
    ProcessRole, StorageError
};

/// Holder-Process専用ストレージ操作モジュール
///
/// このモジュールは Holder-Process のみが使用する専用ストレージ操作を提供します。
/// KV_STORAGE_SPEC.md に従い、3段階パターン（状態ロード→処理→状態保存）を実装します。

/// Holder プロセスの状態管理構造体
#[derive(Debug, Clone)]
pub struct HolderProcessState {
    pub metadata: HolderMetadata,
    pub kfrags: Vec<HolderKFragData>,
    pub cfrags: Vec<HolderCFragData>,
    pub cached_capsules: Vec<CapsuleData>,
    pub last_updated: u64,
}

impl Zeroize for HolderProcessState {
    fn zeroize(&mut self) {
        // 秘密データのみゼロ化
        self.kfrags.zeroize();
        // metadata, cfrags, cached_capsules, last_updated はスキップ
    }
}

impl ZeroizeOnDrop for HolderProcessState {}

/// Holder専用ストレージ操作の結果
#[derive(Debug)]
pub struct HolderStorageResult<T> {
    pub data: T,
    pub affected_records: usize,
}

/// Holder統計情報
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HolderStatistics {
    pub total_kfrags_received: usize,
    pub processed_kfrags: usize,
    pub unprocessed_kfrags: usize,
    pub total_cfrags_generated: usize,
    pub cached_capsules_count: usize,
    pub cache_hit_rate: f64,
    pub last_activity: u64,
}

/// Holder-Process 状態の完全ロード
pub fn load_holder_state(deps: Deps, current_time: Timestamp) -> StdResult<HolderProcessState> {
    // メタデータをロード
    let metadata = HOLDER_METADATA.load(deps.storage)
        .map_err(|_| StdError::generic_err("Holder metadata not found"))?;

    // すべてのkFragsをロード
    let kfrags: StdResult<Vec<_>> = HOLDER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    // すべてのcFragsをロード
    let cfrags: StdResult<Vec<_>> = HOLDER_CFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    // キャッシュされたCapsulesをロード
    let cached_capsules: StdResult<Vec<_>> = CAPSULE_CACHE
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    Ok(HolderProcessState {
        metadata,
        kfrags: kfrags?,
        cfrags: cfrags?,
        cached_capsules: cached_capsules?,
        last_updated: current_time.seconds(),
    })
}

/// Holder-Process 状態の完全保存
pub fn save_holder_state(
    deps: DepsMut,
    state: &HolderProcessState
) -> StdResult<HolderStorageResult<()>> {
    let mut affected_records = 0;

    // メタデータを保存
    HOLDER_METADATA.save(deps.storage, &state.metadata)?;
    affected_records += 1;

    // kFragsを保存（既存のものは上書き）
    for kfrag in &state.kfrags {
        HOLDER_KFRAGS.save(deps.storage, kfrag.kfrag_id.clone(), kfrag)?;
        affected_records += 1;
    }

    // cFragsを保存
    for cfrag in &state.cfrags {
        HOLDER_CFRAGS.save(deps.storage, cfrag.cfrag_id.clone(), cfrag)?;
        affected_records += 1;
    }

    // キャッシュされたCapsulesを保存
    for capsule in &state.cached_capsules {
        CAPSULE_CACHE.save(deps.storage, capsule.capsule_id.clone(), capsule)?;
        affected_records += 1;
    }

    Ok(HolderStorageResult {
        data: (),
        affected_records,
    })
}

/// kFragデータの保存（単体）
pub fn store_kfrag_data(
    deps: DepsMut,
    kfrag_id: String,
    source_owner: String,
    encrypted_kfrag: Vec<u8>,
    signature: Vec<u8>,
    current_time: Timestamp,
) -> StdResult<HolderStorageResult<String>> {
    let kfrag_data = HolderKFragData {
        kfrag_id: kfrag_id.clone(),
        source_owner,
        encrypted_kfrag,
        signature,
        received_at: current_time.seconds(),
        processed: false,
    };

    HOLDER_KFRAGS.save(deps.storage, kfrag_id.clone(), &kfrag_data)?;

    Ok(HolderStorageResult {
        data: kfrag_id,
        affected_records: 1,
    })
}

/// kFragの取得（ID指定）
pub fn get_kfrag_by_id(deps: Deps, kfrag_id: String) -> StdResult<HolderKFragData> {
    HOLDER_KFRAGS.load(deps.storage, kfrag_id.clone())
        .map_err(|_| StdError::generic_err(format!("KFrag not found: {}", kfrag_id)))
}

/// すべてのkFrags取得
pub fn get_all_kfrags(deps: Deps) -> StdResult<Vec<HolderKFragData>> {
    let kfrags: StdResult<Vec<_>> = HOLDER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    kfrags
}

/// 処理済みkFrags取得
pub fn get_processed_kfrags(deps: Deps) -> StdResult<Vec<HolderKFragData>> {
    let kfrags: StdResult<Vec<_>> = HOLDER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((_, kfrag)) => {
                    if kfrag.processed {
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

/// 未処理kFrags取得
pub fn get_unprocessed_kfrags(deps: Deps) -> StdResult<Vec<HolderKFragData>> {
    let kfrags: StdResult<Vec<_>> = HOLDER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((_, kfrag)) => {
                    if !kfrag.processed {
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

/// Capsuleデータの保存・キャッシュ
pub fn store_capsule_data(
    deps: DepsMut,
    capsule_id: String,
    capsule_bytes: Vec<u8>,
    arweave_txid: String,
    current_time: Timestamp,
) -> StdResult<HolderStorageResult<String>> {
    let capsule_data = CapsuleData {
        capsule_id: capsule_id.clone(),
        capsule_bytes,
        arweave_txid,
        cached_at: current_time.seconds(),
    };

    CAPSULE_CACHE.save(deps.storage, capsule_id.clone(), &capsule_data)?;

    Ok(HolderStorageResult {
        data: capsule_id,
        affected_records: 1,
    })
}

/// Capsuleの取得（ID指定）
pub fn get_capsule_by_id(deps: Deps, capsule_id: String) -> StdResult<CapsuleData> {
    CAPSULE_CACHE.load(deps.storage, capsule_id.clone())
        .map_err(|_| StdError::generic_err(format!("Capsule not found: {}", capsule_id)))
}

/// キャッシュされたCapsules一覧取得
pub fn get_cached_capsules(deps: Deps) -> StdResult<Vec<CapsuleData>> {
    let capsules: StdResult<Vec<_>> = CAPSULE_CACHE
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    capsules
}

/// Capsuleキャッシュのクリーンアップ（古いエントリの削除）
pub fn cleanup_capsule_cache(
    deps: DepsMut,
    retention_seconds: u64,
    current_time: Timestamp,
) -> StdResult<HolderStorageResult<usize>> {
    let current_seconds = current_time.seconds();
    let cutoff_time = current_seconds.saturating_sub(retention_seconds);

    let mut cleaned_count = 0;

    // 古いキャッシュエントリを取得
    let old_entries: StdResult<Vec<_>> = CAPSULE_CACHE
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((key, capsule)) => {
                    if capsule.cached_at < cutoff_time {
                        Some(Ok(key))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect();

    // 古いエントリを削除
    for key in old_entries? {
        CAPSULE_CACHE.remove(deps.storage, key);
        cleaned_count += 1;
    }

    Ok(HolderStorageResult {
        data: cleaned_count,
        affected_records: cleaned_count,
    })
}

/// cFragデータの保存
pub fn store_cfrag_data(
    deps: DepsMut,
    cfrag_id: String,
    source_kfrag: String,
    cfrag_data: Vec<u8>,
    current_time: Timestamp,
    arweave_txid: Option<String>,
) -> StdResult<HolderStorageResult<String>> {
    let cfrag = HolderCFragData {
        cfrag_id: cfrag_id.clone(),
        source_kfrag,
        cfrag_data,
        arweave_txid,
        generated_at: current_time.seconds(),
    };

    HOLDER_CFRAGS.save(deps.storage, cfrag_id.clone(), &cfrag)?;

    Ok(HolderStorageResult {
        data: cfrag_id,
        affected_records: 1,
    })
}

/// cFragの取得（ID指定）
pub fn get_cfrag_by_id(deps: Deps, cfrag_id: String) -> StdResult<HolderCFragData> {
    HOLDER_CFRAGS.load(deps.storage, cfrag_id.clone())
        .map_err(|_| StdError::generic_err(format!("CFrag not found: {}", cfrag_id)))
}

/// cFragの取得（kFrag ID指定）
pub fn get_cfrags_by_kfrag(deps: Deps, kfrag_id: String) -> StdResult<Vec<HolderCFragData>> {
    let cfrags: StdResult<Vec<_>> = HOLDER_CFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|item| {
            match item {
                Ok((_, cfrag)) => {
                    if cfrag.source_kfrag == kfrag_id {
                        Some(Ok(cfrag))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        })
        .collect();

    cfrags
}

/// すべてのcFrags取得
pub fn get_all_cfrags(deps: Deps) -> StdResult<Vec<HolderCFragData>> {
    let cfrags: StdResult<Vec<_>> = HOLDER_CFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, data)| data))
        .collect();

    cfrags
}

/// kFragを処理済みとしてマーク
pub fn mark_kfrag_processed(
    deps: DepsMut,
    kfrag_id: String,
) -> StdResult<HolderStorageResult<bool>> {
    let mut kfrag_data = HOLDER_KFRAGS.load(deps.storage, kfrag_id.clone())
        .map_err(|_| StdError::generic_err(format!("KFrag not found: {}", kfrag_id)))?;

    kfrag_data.processed = true;
    HOLDER_KFRAGS.save(deps.storage, kfrag_id, &kfrag_data)?;

    Ok(HolderStorageResult {
        data: true,
        affected_records: 1,
    })
}

/// cFragのArweave TXID更新
pub fn update_cfrag_arweave_txid(
    deps: DepsMut,
    cfrag_id: String,
    arweave_txid: String,
) -> StdResult<HolderStorageResult<String>> {
    let mut cfrag_data = HOLDER_CFRAGS.load(deps.storage, cfrag_id.clone())
        .map_err(|_| StdError::generic_err(format!("CFrag not found: {}", cfrag_id)))?;

    cfrag_data.arweave_txid = Some(arweave_txid.clone());
    HOLDER_CFRAGS.save(deps.storage, cfrag_id, &cfrag_data)?;

    Ok(HolderStorageResult {
        data: arweave_txid,
        affected_records: 1,
    })
}

/// 全kFragデータのクリーンアップ（セキュリティ対応）
pub fn cleanup_sensitive_data(deps: DepsMut) -> StdResult<HolderStorageResult<usize>> {
    let mut cleaned_count = 0;

    // すべてのkFragsを取得し、メモリクリア済みのものに置き換え
    let kfrag_keys: StdResult<Vec<_>> = HOLDER_KFRAGS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(key, _)| key))
        .collect();

    for key in kfrag_keys? {
        // 暗号化データをゼロクリア
        let mut kfrag = HOLDER_KFRAGS.load(deps.storage, key.clone())?;
        kfrag.encrypted_kfrag.zeroize(); // 明示的にゼロクリア
        kfrag.signature.zeroize(); // 署名もクリア
        kfrag.encrypted_kfrag = Vec::new();
        kfrag.signature = Vec::new();

        HOLDER_KFRAGS.save(deps.storage, key, &kfrag)?;
        cleaned_count += 1;
    }

    Ok(HolderStorageResult {
        data: cleaned_count,
        affected_records: cleaned_count,
    })
}

/// プロセス役割の検証
pub fn verify_holder_role(deps: Deps) -> Result<(), StorageError> {
    let metadata = HOLDER_METADATA.load(deps.storage)
        .map_err(|_| StorageError::DataNotFound {
            key: "holder_metadata".to_string()
        })?;

    if !matches!(metadata.process_role, ProcessRole::Holder) {
        return Err(StorageError::Unauthorized);
    }

    Ok(())
}

/// Holder統計情報の取得
pub fn get_holder_statistics(deps: Deps, current_time: Timestamp) -> StdResult<HolderStatistics> {
    // kFrag統計
    let all_kfrags = get_all_kfrags(deps)?;
    let total_kfrags_received = all_kfrags.len();
    let processed_kfrags = all_kfrags.iter().filter(|k| k.processed).count();
    let unprocessed_kfrags = total_kfrags_received - processed_kfrags;

    // cFrag統計
    let all_cfrags = get_all_cfrags(deps)?;
    let total_cfrags_generated = all_cfrags.len();

    // キャッシュ統計
    let cached_capsules = get_cached_capsules(deps)?;
    let cached_capsules_count = cached_capsules.len();

    // キャッシュヒット率の計算（簡易版）
    let cache_hit_rate = if total_cfrags_generated > 0 {
        cached_capsules_count as f64 / total_cfrags_generated as f64
    } else {
        0.0
    };

    // 最後のアクティビティ時刻
    let last_activity = all_kfrags.iter()
        .map(|k| k.received_at)
        .chain(all_cfrags.iter().map(|c| c.generated_at))
        .max()
        .unwrap_or(current_time.seconds());

    Ok(HolderStatistics {
        total_kfrags_received,
        processed_kfrags,
        unprocessed_kfrags,
        total_cfrags_generated,
        cached_capsules_count,
        cache_hit_rate,
        last_activity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};

    fn setup_test_holder_metadata(deps: DepsMut) {
        let metadata = HolderMetadata {
            holder_id: "test_holder".to_string(),
            process_role: ProcessRole::Holder,
            assigned_owners: vec!["owner_1".to_string()],
            initialization_time: 1000,
        };

        HOLDER_METADATA.save(deps.storage, &metadata).unwrap();
    }

    #[test]
    fn test_store_and_retrieve_kfrag() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // kFrag保存テスト
        let result = store_kfrag_data(
            deps.as_mut(),
            "test_kfrag_1".to_string(),
            "owner_1".to_string(),
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            env.block.time,
        ).unwrap();

        assert_eq!(result.data, "test_kfrag_1");
        assert_eq!(result.affected_records, 1);

        // 保存されたkFragを取得
        let kfrag = get_kfrag_by_id(deps.as_ref(), "test_kfrag_1".to_string()).unwrap();
        assert_eq!(kfrag.kfrag_id, "test_kfrag_1");
        assert_eq!(kfrag.source_owner, "owner_1");
        assert!(!kfrag.processed);
    }

    #[test]
    fn test_capsule_caching() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // Capsule保存テスト
        let result = store_capsule_data(
            deps.as_mut(),
            "capsule_1".to_string(),
            vec![9, 10, 11, 12],
            "arweave_tx_1".to_string(),
            env.block.time,
        ).unwrap();

        assert_eq!(result.data, "capsule_1");
        assert_eq!(result.affected_records, 1);

        // 保存されたCapsuleを取得
        let capsule = get_capsule_by_id(deps.as_ref(), "capsule_1".to_string()).unwrap();
        assert_eq!(capsule.capsule_id, "capsule_1");
        assert_eq!(capsule.arweave_txid, "arweave_tx_1");
    }

    #[test]
    fn test_cfrag_generation_and_storage() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // cFrag保存テスト
        let result = store_cfrag_data(
            deps.as_mut(),
            "cfrag_1".to_string(),
            "kfrag_1".to_string(),
            vec![13, 14, 15, 16],
            env.block.time,
            Some("arweave_cfrag_tx_1".to_string()),
        ).unwrap();

        assert_eq!(result.data, "cfrag_1");
        assert_eq!(result.affected_records, 1);

        // 保存されたcFragを取得
        let cfrag = get_cfrag_by_id(deps.as_ref(), "cfrag_1".to_string()).unwrap();
        assert_eq!(cfrag.cfrag_id, "cfrag_1");
        assert_eq!(cfrag.source_kfrag, "kfrag_1");
        assert_eq!(cfrag.arweave_txid, Some("arweave_cfrag_tx_1".to_string()));
    }

    #[test]
    fn test_kfrag_processing_status() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // kFragを追加
        store_kfrag_data(
            deps.as_mut(),
            "kfrag_1".to_string(),
            "owner_1".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
            env.block.time,
        ).unwrap();

        store_kfrag_data(
            deps.as_mut(),
            "kfrag_2".to_string(),
            "owner_1".to_string(),
            vec![7, 8, 9],
            vec![10, 11, 12],
            env.block.time,
        ).unwrap();

        // 1つを処理済みにマーク
        mark_kfrag_processed(deps.as_mut(), "kfrag_1".to_string()).unwrap();

        // 処理済みと未処理を確認
        let processed = get_processed_kfrags(deps.as_ref()).unwrap();
        let unprocessed = get_unprocessed_kfrags(deps.as_ref()).unwrap();

        assert_eq!(processed.len(), 1);
        assert_eq!(unprocessed.len(), 1);
        assert_eq!(processed[0].kfrag_id, "kfrag_1");
        assert_eq!(unprocessed[0].kfrag_id, "kfrag_2");
    }

    #[test]
    fn test_holder_statistics() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // テストデータを追加
        store_kfrag_data(
            deps.as_mut(),
            "kfrag_1".to_string(),
            "owner_1".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
            env.block.time,
        ).unwrap();

        store_cfrag_data(
            deps.as_mut(),
            "cfrag_1".to_string(),
            "kfrag_1".to_string(),
            vec![7, 8, 9],
            env.block.time,
            None,
        ).unwrap();

        store_capsule_data(
            deps.as_mut(),
            "capsule_1".to_string(),
            vec![10, 11, 12],
            "arweave_tx_1".to_string(),
            env.block.time,
        ).unwrap();

        // 統計情報を取得
        let stats = get_holder_statistics(deps.as_ref(), env.block.time).unwrap();

        assert_eq!(stats.total_kfrags_received, 1);
        assert_eq!(stats.processed_kfrags, 0);
        assert_eq!(stats.unprocessed_kfrags, 1);
        assert_eq!(stats.total_cfrags_generated, 1);
        assert_eq!(stats.cached_capsules_count, 1);
    }

    #[test]
    fn test_capsule_cache_cleanup() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        setup_test_holder_metadata(deps.as_mut());

        // 古いCapsuleを追加
        let old_time = Timestamp::from_seconds(env.block.time.seconds() - 3600); // 1時間前
        store_capsule_data(
            deps.as_mut(),
            "old_capsule".to_string(),
            vec![1, 2, 3],
            "old_tx".to_string(),
            old_time,
        ).unwrap();

        // 新しいCapsuleを追加
        store_capsule_data(
            deps.as_mut(),
            "new_capsule".to_string(),
            vec![4, 5, 6],
            "new_tx".to_string(),
            env.block.time,
        ).unwrap();

        // 30分より古いキャッシュをクリーンアップ
        let cleanup_result = cleanup_capsule_cache(
            deps.as_mut(),
            1800, // 30分
            env.block.time,
        ).unwrap();

        assert_eq!(cleanup_result.data, 1); // 1つのエントリがクリーンアップされた

        // 新しいCapsuleのみが残っていることを確認
        let remaining_capsules = get_cached_capsules(deps.as_ref()).unwrap();
        assert_eq!(remaining_capsules.len(), 1);
        assert_eq!(remaining_capsules[0].capsule_id, "new_capsule");
    }
}