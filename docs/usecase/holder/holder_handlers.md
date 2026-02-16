# Holder Role Handlers 詳細設計

## 1. 概要

Holder Roleは、FORMIXシステムにおける分散ストレージと再暗号化の実行を担当する重要なロールです。k-of-n閾値暗号化スキームにおいて、各Holderは再暗号化キーフラグメント（kFrag）を保持し、認証されたアクセス要求に対してプロキシ再暗号化を実行します。

### 1.1 Holder Roleの責務

1. **kFrag管理**
   - Ownerから配布されたkFragの安全な保存
   - kFragの有効期限管理
   - 不正なkFragの拒否

2. **プロキシ再暗号化**
   - 認証されたアクセス要求に対する再暗号化実行
   - cFrag（暗号フラグメント）の生成
   - 再暗号化の正確性保証

3. **信頼性維持**
   - 可用性の維持
   - 応答時間の最適化
   - 信頼スコアの管理

### 1.2 設計原則

- **独立性**: 各Holderは他のHolderと独立して動作
- **検証可能性**: すべての再暗号化操作は暗号学的に検証可能
- **可用性**: 高い稼働率とレスポンス性能の維持
- **プライバシー**: 元の秘密データへのアクセス不可

## 2. ハンドラー登録

### 2.1 Holder専用ハンドラーの登録

```rust
use ao_sdk::{Handlers, Message, Response};

/// Holderロール用ハンドラーの登録
pub fn register_holder_handlers() -> Result<(), HandlerError> {
    // Phase 3: kFrag受信・保存
    Handlers::add(
        "store-kfrag",
        Handlers::utils::hasMatchingTag("Action", "Store-KFrag"),
        handle_store_kfrag
    );
    
    Handlers::add(
        "acknowledge-kfrag",
        Handlers::utils::hasMatchingTag("Action", "Acknowledge-KFrag"),
        handle_acknowledge_kfrag
    );
    
    // Phase 4: プロキシ再暗号化
    Handlers::add(
        "perform-reencryption",
        Handlers::utils::hasMatchingTag("Action", "Perform-Reencryption"),
        handle_perform_reencryption
    );
    
    Handlers::add(
        "send-cfrag",
        Handlers::utils::hasMatchingTag("Action", "Send-CFrag"),
        handle_send_cfrag
    );
    
    // 管理系
    Handlers::add(
        "update-trust-score",
        Handlers::utils::hasMatchingTag("Action", "Update-Trust-Score"),
        handle_update_trust_score
    );
    
    Handlers::add(
        "report-status",
        Handlers::utils::hasMatchingTag("Action", "Report-Status"),
        handle_report_status
    );
    
    Handlers::add(
        "cleanup-expired",
        Handlers::utils::hasMatchingTag("Action", "Cleanup-Expired"),
        handle_cleanup_expired
    );
    
    Handlers::add(
        "verify-kfrag",
        Handlers::utils::hasMatchingTag("Action", "Verify-KFrag"),
        handle_verify_kfrag
    );
    
    Ok(())
}
```

### 2.2 Holderロール検証

```rust
/// Holderロール検証デコレータ
pub fn require_holder_role<F>(handler: F) -> impl Fn(Message) -> Response
where
    F: Fn(Message) -> Response,
{
    move |msg: Message| {
        // コンテキスト初期化とロール検証
        let ctx = match initialize_and_verify_holder().await {
            Ok(ctx) => ctx,
            Err(e) => return unauthorized_response(e),
        };
        
        // 実際のハンドラー実行
        handler(msg)
    }
}

async fn initialize_and_verify_holder() -> Result<HandlerContext, RoleError> {
    let ctx = HandlerContext::initialize().await?;
    
    if !ctx.process.has_role(ProcessRole::Holder) {
        return Err(RoleError::Unauthorized {
            required: ProcessRole::Holder,
            actual: ctx.process.get_primary_role(),
        });
    }
    
    // Holder固有データの存在確認
    if ctx.process.holder_data.is_none() {
        return Err(RoleError::UninitializedRole);
    }
    
    Ok(ctx)
}
```

## 3. Phase 3: kFrag管理ハンドラー

### 3.1 Store-KFrag

```rust
/// kFrag保存ハンドラー
pub async fn handle_store_kfrag(msg: Message) -> Response {
    // コンテキスト初期化とHolderロール検証
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // kFragデータの抽出と検証
    let kfrag_data = match extract_and_validate_kfrag(&msg) {
        Ok(data) => data,
        Err(e) => return validation_error_response(e),
    };
    
    // 送信元Owner の検証
    if let Err(e) = verify_kfrag_source(&ctx, &kfrag_data, &msg).await {
        return error_response(format!("Invalid kFrag source: {}", e));
    }
    
    // 暗号学的検証
    if let Err(e) = verify_kfrag_cryptography(&kfrag_data).await {
        return error_response(format!("Invalid kFrag cryptography: {}", e));
    }
    
    // ストレージ容量チェック
    if let Err(e) = check_storage_capacity(&ctx).await {
        return error_response(format!("Storage capacity exceeded: {}", e));
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の後処理
    if is_success(&response) {
        // Ownerへの確認応答を送信
        send_kfrag_acknowledgment(&kfrag_data).await;
        
        // メトリクス更新
        update_holder_metrics(HolderMetric::KFragStored).await;
    }
    
    response
}

/// kFragソースの検証
async fn verify_kfrag_source(
    ctx: &HandlerContext,
    kfrag_data: &KFragData,
    msg: &Message,
) -> Result<(), SourceVerificationError> {
    // 署名検証
    let signature = msg.tags.get("Owner-Signature")
        .ok_or(SourceVerificationError::MissingSignature)?;
    
    // Ownerの公開鍵を取得
    let owner_pubkey = get_owner_public_key(&kfrag_data.data_id).await?;
    
    // 署名検証
    verify_signature(
        &kfrag_data.to_bytes(),
        signature,
        &owner_pubkey,
    )?;
    
    // 配布リストに含まれているか確認
    let distribution_list = get_distribution_list(&kfrag_data.access_request_id).await?;
    if !distribution_list.contains(&ctx.process.process_id) {
        return Err(SourceVerificationError::NotInDistributionList);
    }
    
    Ok(())
}

/// kFragの暗号学的検証
async fn verify_kfrag_cryptography(kfrag_data: &KFragData) -> Result<(), CryptoError> {
    // kFragの形式検証
    let kfrag = KeyFragment::from_bytes(&kfrag_data.encrypted_kfrag)?;
    
    // 検証可能なランダムネスのチェック
    if let Some(proof) = &kfrag_data.validity_proof {
        kfrag.verify_proof(proof)?;
    }
    
    // kFragのサイズと構造の検証
    if !kfrag.is_well_formed() {
        return Err(CryptoError::MalformedKFrag);
    }
    
    Ok(())
}
```

### 3.2 Acknowledge-KFrag

```rust
/// kFrag受信確認ハンドラー
pub async fn handle_acknowledge_kfrag(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 確認対象のkFrag ID
    let kfrag_id = match msg.tags.get("KFrag-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing KFrag-Id"),
    };
    
    // 保存済みkFragの確認
    let stored_kfrag = match get_stored_kfrag(&ctx, kfrag_id).await {
        Ok(kfrag) => kfrag,
        Err(e) => return error_response(format!("KFrag not found: {}", e)),
    };
    
    // 確認応答の生成
    let acknowledgment = KFragAcknowledgment {
        kfrag_id: kfrag_id.clone(),
        holder_id: ctx.process.process_id.clone(),
        stored_at: SystemTime::now(),
        storage_proof: generate_storage_proof(&stored_kfrag),
        availability_commitment: AvailabilityCommitment {
            guaranteed_until: SystemTime::now() + Duration::from_secs(86400 * 30), // 30日
            sla_level: SLALevel::Standard,
        },
    };
    
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Acknowledge-KFrag-Response"),
            ("KFrag-Id", kfrag_id),
            ("Storage-Proof", &acknowledgment.storage_proof),
        ],
        data: serde_json::to_vec(&acknowledgment).unwrap_or_default(),
    }
}
```

## 4. Phase 4: プロキシ再暗号化ハンドラー

### 4.1 Perform-Reencryption

```rust
/// プロキシ再暗号化実行ハンドラー
pub async fn handle_perform_reencryption(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 再暗号化要求の抽出
    let reenc_request = match extract_reencryption_request(&msg) {
        Ok(req) => req,
        Err(e) => return validation_error_response(e),
    };
    
    // アクセス権限の検証
    if let Err(e) = verify_access_authorization(&ctx, &reenc_request).await {
        return error_response(format!("Access not authorized: {}", e));
    }
    
    // 対応するkFragの取得
    let kfrag = match get_kfrag_for_request(&ctx, &reenc_request).await {
        Ok(kfrag) => kfrag,
        Err(e) => return error_response(format!("KFrag not available: {}", e));
    };
    
    // カプセルの取得と検証
    let capsule = match get_and_verify_capsule(&reenc_request.capsule_id).await {
        Ok(cap) => cap,
        Err(e) => return error_response(format!("Invalid capsule: {}", e));
    };
    
    // プロキシ再暗号化の実行
    let cfrag = match perform_proxy_reencryption(&kfrag, &capsule) {
        Ok(cf) => cf,
        Err(e) => {
            error!("Reencryption failed: {:?}", e);
            return error_response("Reencryption failed");
        }
    };
    
    // cFragの保存と送信準備
    let cfrag_data = CFragData {
        cfrag_id: generate_cfrag_id(),
        request_id: reenc_request.request_id.clone(),
        holder_id: ctx.process.process_id.clone(),
        cfrag: cfrag.to_bytes(),
        created_at: SystemTime::now(),
        validity_proof: generate_cfrag_proof(&cfrag, &kfrag),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時のメトリクス更新
    if is_success(&response) {
        update_reencryption_metrics(&ctx).await;
    }
    
    response
}

/// アクセス権限の検証
async fn verify_access_authorization(
    ctx: &HandlerContext,
    request: &ReencryptionRequest,
) -> Result<(), AuthorizationError> {
    // アクセス要求の有効性確認
    let access_request = get_access_request(&request.access_request_id).await?;
    
    if !access_request.is_approved() {
        return Err(AuthorizationError::NotApproved);
    }
    
    if access_request.has_expired() {
        return Err(AuthorizationError::Expired);
    }
    
    // リクエスターの確認
    if request.requester_id != access_request.requester_id {
        return Err(AuthorizationError::RequesterMismatch);
    }
    
    // 再実行防止
    if has_already_reencrypted(&ctx, &request.request_id).await? {
        return Err(AuthorizationError::AlreadyProcessed);
    }
    
    Ok(())
}

/// プロキシ再暗号化の実行
fn perform_proxy_reencryption(
    kfrag: &KeyFragment,
    capsule: &Capsule,
) -> Result<CipherFragment, ReencryptionError> {
    // Umbralライブラリを使用した再暗号化
    use umbral_pre::{reencrypt, CipherFragment};
    
    // メモリ保護
    let _guard = SecureMemoryGuard::new();
    
    // 再暗号化実行
    let cfrag = reencrypt(kfrag, capsule)
        .map_err(|e| ReencryptionError::CryptoError(e.to_string()))?;
    
    // cFragの妥当性検証
    if !cfrag.verify() {
        return Err(ReencryptionError::InvalidCFrag);
    }
    
    Ok(cfrag)
}
```

### 4.2 Send-CFrag

```rust
/// cFrag送信ハンドラー
pub async fn handle_send_cfrag(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 送信対象のcFrag ID
    let cfrag_id = match msg.tags.get("CFrag-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing CFrag-Id"),
    };
    
    // 保存済みcFragの取得
    let cfrag_data = match get_stored_cfrag(&ctx, cfrag_id).await {
        Ok(data) => data,
        Err(e) => return error_response(format!("CFrag not found: {}", e)),
    };
    
    // 送信先の確認
    let target_requester = match msg.tags.get("Target-Requester") {
        Some(id) => id,
        None => &cfrag_data.original_requester,
    };
    
    // cFragの最終検証
    if let Err(e) = final_cfrag_verification(&cfrag_data) {
        return error_response(format!("CFrag verification failed: {}", e));
    }
    
    // 送信レコードの作成
    let send_record = CFragSendRecord {
        cfrag_id: cfrag_id.clone(),
        sent_to: target_requester.clone(),
        sent_at: SystemTime::now(),
        delivery_proof: generate_delivery_proof(&cfrag_data),
    };
    
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Send-CFrag-Response"),
            ("CFrag-Id", cfrag_id),
            ("Target-Requester", target_requester),
            ("Delivery-Proof", &send_record.delivery_proof),
        ],
        data: serde_json::to_vec(&cfrag_data).unwrap_or_default(),
    }
}
```

## 5. 管理系ハンドラー

### 5.1 Update-Trust-Score

```rust
/// 信頼スコア更新ハンドラー
pub async fn handle_update_trust_score(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // スコア更新情報の抽出
    let score_update = match extract_score_update(&msg) {
        Ok(update) => update,
        Err(e) => return validation_error_response(e),
    };
    
    // 更新権限の確認（システムまたは監査人のみ）
    if !is_authorized_to_update_score(&msg) {
        return error_response("Not authorized to update trust score");
    }
    
    // 現在のスコアを取得
    let current_score = ctx.process.holder_data
        .as_ref()
        .map(|d| d.trust_score)
        .unwrap_or(DEFAULT_TRUST_SCORE);
    
    // 新しいスコアの計算
    let new_score = calculate_new_trust_score(
        current_score,
        &score_update,
    );
    
    // スコアの妥当性検証
    if new_score < MIN_TRUST_SCORE || new_score > MAX_TRUST_SCORE {
        return error_response(format!(
            "Invalid trust score: {} (must be between {} and {})",
            new_score, MIN_TRUST_SCORE, MAX_TRUST_SCORE
        ));
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}

/// 信頼スコアの計算
fn calculate_new_trust_score(
    current: f64,
    update: &TrustScoreUpdate,
) -> f64 {
    match update.update_type {
        ScoreUpdateType::Increment => {
            // 成功した操作による増加
            (current + update.delta).min(MAX_TRUST_SCORE)
        }
        ScoreUpdateType::Decrement => {
            // 失敗やタイムアウトによる減少
            (current - update.delta).max(MIN_TRUST_SCORE)
        }
        ScoreUpdateType::Set => {
            // 監査による直接設定
            update.new_value.unwrap_or(current)
        }
        ScoreUpdateType::Decay => {
            // 時間経過による自然減衰
            current * (1.0 - update.decay_rate)
        }
    }
}
```

### 5.2 Report-Status

```rust
/// ステータス報告ハンドラー
pub async fn handle_report_status(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // ステータス収集
    let status = match collect_holder_status(&ctx).await {
        Ok(status) => status,
        Err(e) => return error_response(format!("Failed to collect status: {}", e)),
    };
    
    // ヘルスチェック
    let health = perform_health_check(&ctx).await;
    
    // パフォーマンスメトリクス
    let metrics = collect_performance_metrics(&ctx).await;
    
    let report = HolderStatusReport {
        holder_id: ctx.process.process_id.clone(),
        timestamp: SystemTime::now(),
        status,
        health,
        metrics,
        capabilities: HolderCapabilities {
            max_kfrags: MAX_KFRAGS_PER_HOLDER,
            current_kfrags: status.active_kfrags,
            available_capacity: calculate_available_capacity(&status),
            // 注: 現在の実装はsecp256k1のみをサポート。ed25519は将来的な拡張予定
            supported_algorithms: vec!["umbral-secp256k1"],
            average_response_time: metrics.avg_response_time,
        },
    };
    
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Report-Status-Response"),
            ("Health", &health.overall_status.to_string()),
            ("Available-Capacity", &report.capabilities.available_capacity.to_string()),
        ],
        data: serde_json::to_vec(&report).unwrap_or_default(),
    }
}

/// Holderステータスの収集
async fn collect_holder_status(ctx: &HandlerContext) -> Result<HolderStatus, StatusError> {
    let holder_data = ctx.process.holder_data.as_ref()
        .ok_or(StatusError::NoHolderData)?;
    
    // 保持しているkFragの統計
    let kfrag_stats = ctx.repositories.kfrag_repo
        .get_statistics(&ctx.process.process_id)
        .await?;
    
    // 再暗号化の履歴
    let reenc_history = ctx.repositories.reencryption_repo
        .get_recent_history(&ctx.process.process_id, 100)
        .await?;
    
    Ok(HolderStatus {
        active_kfrags: kfrag_stats.active_count,
        expired_kfrags: kfrag_stats.expired_count,
        total_reencryptions: reenc_history.len(),
        success_rate: calculate_success_rate(&reenc_history),
        last_activity: reenc_history.first().map(|h| h.timestamp),
        trust_score: holder_data.trust_score,
        uptime: calculate_uptime(&holder_data.activated_at),
    })
}
```

### 5.3 Cleanup-Expired

```rust
/// 期限切れデータのクリーンアップハンドラー
pub async fn handle_cleanup_expired(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // クリーンアップ対象の特定
    let expired_items = match identify_expired_items(&ctx).await {
        Ok(items) => items,
        Err(e) => return error_response(format!("Failed to identify expired items: {}", e)),
    };
    
    if expired_items.is_empty() {
        return Response {
            tags: vec![
                ("Status", "Success"),
                ("Action", "Cleanup-Expired-Response"),
                ("Cleaned-Count", "0"),
            ],
            data: vec![],
        };
    }
    
    // クリーンアップの実行
    let cleanup_results = perform_cleanup(&ctx, expired_items).await;
    
    // 結果の集計
    let success_count = cleanup_results.iter()
        .filter(|r| r.is_ok())
        .count();
    let failure_count = cleanup_results.len() - success_count;
    
    // クリーンアップレポートの生成
    let report = CleanupReport {
        total_items: cleanup_results.len(),
        success_count,
        failure_count,
        freed_space: calculate_freed_space(&cleanup_results),
        errors: cleanup_results.iter()
            .filter_map(|r| r.as_ref().err().map(|e| e.to_string()))
            .collect(),
    };
    
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Cleanup-Expired-Response"),
            ("Cleaned-Count", &success_count.to_string()),
            ("Failed-Count", &failure_count.to_string()),
        ],
        data: serde_json::to_vec(&report).unwrap_or_default(),
    }
}

/// 期限切れアイテムの特定
async fn identify_expired_items(ctx: &HandlerContext) -> Result<Vec<ExpiredItem>, CleanupError> {
    let mut expired_items = Vec::new();
    let current_time = SystemTime::now();
    
    // 期限切れkFragの検索
    let expired_kfrags = ctx.repositories.kfrag_repo
        .find_expired(&ctx.process.process_id, current_time)
        .await?;
    
    for kfrag in expired_kfrags {
        expired_items.push(ExpiredItem::KFrag(kfrag));
    }
    
    // 古いcFragの検索（送信済みで一定期間経過）
    let old_cfrags = ctx.repositories.cfrag_repo
        .find_old_sent(&ctx.process.process_id, current_time - CFRAG_RETENTION_PERIOD)
        .await?;
    
    for cfrag in old_cfrags {
        expired_items.push(ExpiredItem::CFrag(cfrag));
    }
    
    Ok(expired_items)
}
```

### 5.4 Verify-KFrag

```rust
/// kFrag検証ハンドラー
pub async fn handle_verify_kfrag(msg: Message) -> Response {
    let ctx = match initialize_and_verify_holder().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 検証対象のkFrag ID
    let kfrag_id = match msg.tags.get("KFrag-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing KFrag-Id"),
    };
    
    // kFragの取得
    let kfrag = match get_stored_kfrag(&ctx, kfrag_id).await {
        Ok(kfrag) => kfrag,
        Err(e) => return error_response(format!("KFrag not found: {}", e)),
    };
    
    // 包括的検証の実行
    let verification_result = perform_comprehensive_verification(&kfrag).await;
    
    let report = VerificationReport {
        kfrag_id: kfrag_id.clone(),
        timestamp: SystemTime::now(),
        cryptographic_validity: verification_result.crypto_valid,
        storage_integrity: verification_result.storage_valid,
        access_permissions: verification_result.permissions_valid,
        expiration_status: verification_result.expiration_status,
        overall_status: verification_result.is_valid(),
        details: verification_result.details,
    };
    
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Verify-KFrag-Response"),
            ("KFrag-Id", kfrag_id),
            ("Valid", &report.overall_status.to_string()),
        ],
        data: serde_json::to_vec(&report).unwrap_or_default(),
    }
}

/// 包括的なkFrag検証
async fn perform_comprehensive_verification(
    kfrag: &StoredKFrag,
) -> VerificationResult {
    let mut result = VerificationResult::default();
    
    // 1. 暗号学的検証
    match KeyFragment::from_bytes(&kfrag.encrypted_data) {
        Ok(key_frag) => {
            result.crypto_valid = key_frag.verify();
            if !result.crypto_valid {
                result.details.push("Cryptographic verification failed".to_string());
            }
        }
        Err(e) => {
            result.crypto_valid = false;
            result.details.push(format!("Failed to parse kFrag: {}", e));
        }
    }
    
    // 2. ストレージ整合性
    let computed_hash = compute_hash(&kfrag.encrypted_data);
    result.storage_valid = computed_hash == kfrag.data_hash;
    if !result.storage_valid {
        result.details.push("Storage integrity check failed".to_string());
    }
    
    // 3. アクセス権限
    result.permissions_valid = kfrag.access_list.is_some() && 
                              !kfrag.access_list.as_ref().unwrap().is_empty();
    
    // 4. 有効期限
    result.expiration_status = if let Some(expires_at) = kfrag.expires_at {
        if SystemTime::now() > expires_at {
            ExpirationStatus::Expired
        } else {
            ExpirationStatus::Valid
        }
    } else {
        ExpirationStatus::NeverExpires
    };
    
    result
}
```

## 6. パフォーマンス最適化

### 6.1 並列再暗号化処理

```rust
/// 複数の再暗号化要求の並列処理
pub async fn parallel_reencryption_handler(
    requests: Vec<ReencryptionRequest>,
) -> Vec<Result<CFragData, ReencryptionError>> {
    let semaphore = Arc::new(Semaphore::new(MAX_PARALLEL_REENCRYPTIONS));
    
    let tasks = requests.into_iter().map(|request| {
        let sem = semaphore.clone();
        async move {
            let _permit = sem.acquire().await?;
            perform_single_reencryption(request).await
        }
    });
    
    futures::future::join_all(tasks).await
}
```

### 6.2 kFragキャッシング

```rust
/// 頻繁にアクセスされるkFragのキャッシング
pub struct KFragCache {
    cache: Arc<RwLock<LruCache<String, CachedKFrag>>>,
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl KFragCache {
    pub async fn get_or_load(
        &self,
        kfrag_id: &str,
        loader: impl Future<Output = Result<StoredKFrag, LoadError>>,
    ) -> Result<CachedKFrag, CacheError> {
        // キャッシュチェック
        if let Some(cached) = self.cache.read().await.get(kfrag_id) {
            self.metrics.write().await.hits += 1;
            return Ok(cached.clone());
        }
        
        // キャッシュミス - ロード
        self.metrics.write().await.misses += 1;
        let kfrag = loader.await?;
        
        // 暗号化データの事前パース
        let parsed_kfrag = KeyFragment::from_bytes(&kfrag.encrypted_data)?;
        
        let cached = CachedKFrag {
            raw_data: kfrag,
            parsed_fragment: parsed_kfrag,
            loaded_at: SystemTime::now(),
        };
        
        self.cache.write().await.put(kfrag_id.to_string(), cached.clone());
        Ok(cached)
    }
}
```

### 6.3 バッチ処理最適化

```rust
/// cFragのバッチ送信
pub async fn batch_send_cfrags(
    cfrag_batch: Vec<CFragData>,
    target_requester: &ProcessId,
) -> BatchSendResult {
    // cFragをチャンクに分割
    let chunks: Vec<Vec<CFragData>> = cfrag_batch
        .chunks(MAX_CFRAGS_PER_MESSAGE)
        .map(|chunk| chunk.to_vec())
        .collect();
    
    // 各チャンクを並列送信
    let send_tasks = chunks.into_iter().map(|chunk| {
        send_cfrag_chunk(chunk, target_requester)
    });
    
    let results = futures::future::join_all(send_tasks).await;
    
    BatchSendResult {
        total_sent: cfrag_batch.len(),
        successful: results.iter().filter(|r| r.is_ok()).count(),
        failed: results.iter().filter(|r| r.is_err()).count(),
        errors: results.into_iter()
            .filter_map(|r| r.err())
            .collect(),
    }
}
```

## 7. セキュリティ実装

### 7.1 kFrag隔離

```rust
/// kFragの安全な隔離ストレージ
pub struct IsolatedKFragStorage {
    storage_key: SecureKey,
    isolation_level: IsolationLevel,
}

impl IsolatedKFragStorage {
    pub async fn store_kfrag(
        &self,
        kfrag_id: &str,
        kfrag_data: &[u8],
    ) -> Result<(), StorageError> {
        // データの暗号化
        let encrypted = self.encrypt_kfrag(kfrag_data)?;
        
        // 隔離されたストレージ領域への保存
        let isolated_path = self.get_isolated_path(kfrag_id);
        
        // アクセス権限の設定
        set_strict_permissions(&isolated_path)?;
        
        // データの書き込み
        secure_write(&isolated_path, &encrypted).await?;
        
        Ok(())
    }
    
    fn encrypt_kfrag(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // AES-GCM暗号化
        let cipher = Aes256Gcm::new(&self.storage_key);
        let nonce = generate_nonce();
        
        cipher.encrypt(&nonce, data)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))
    }
}
```

### 7.2 再暗号化の監査証跡

```rust
/// 再暗号化操作の監査ログ
#[derive(Debug, Serialize)]
pub struct ReencryptionAuditLog {
    pub operation_id: String,
    pub timestamp: SystemTime,
    pub holder_id: ProcessId,
    pub requester_id: ProcessId,
    pub kfrag_id: String,
    pub capsule_id: String,
    pub result: ReencryptionResult,
    pub performance_metrics: PerformanceMetrics,
    pub verification_proof: VerificationProof,
}

impl ReencryptionAuditLog {
    pub async fn record(&self) -> Result<(), AuditError> {
        // デジタル署名の生成
        let signature = self.sign()?;
        
        // タイムスタンプ証明の取得
        let timestamp_proof = get_timestamp_proof(&self.operation_id).await?;
        
        // Arweaveへの永続化
        let audit_entry = AuditEntry {
            log: self.clone(),
            signature,
            timestamp_proof,
        };
        
        store_audit_log(&audit_entry).await
    }
}
```

## 8. エラー処理

### 8.1 Holder固有のエラー

```rust
#[derive(Debug, thiserror::Error)]
pub enum HolderError {
    #[error("KFrag not found: {kfrag_id}")]
    KFragNotFound { kfrag_id: String },
    
    #[error("Reencryption failed: {reason}")]
    ReencryptionFailed { reason: String },
    
    #[error("Storage capacity exceeded: current {current}, max {max}")]
    StorageCapacityExceeded { current: usize, max: usize },
    
    #[error("Trust score too low: {score} < {minimum}")]
    InsufficientTrustScore { score: f64, minimum: f64 },
    
    #[error("Invalid capsule: {details}")]
    InvalidCapsule { details: String },
    
    #[error("Verification failed: {component}")]
    VerificationFailed { component: String },
}
```

### 8.2 フォールバック戦略

```rust
/// エラー時のフォールバック処理
pub async fn handle_with_fallback(msg: Message) -> Response {
    let primary_result = handle_primary(msg.clone()).await;
    
    match primary_result {
        Ok(response) => response,
        Err(HolderError::ReencryptionFailed { .. }) => {
            // 再試行with別のアルゴリズム
            if let Ok(response) = retry_with_alternative_method(msg).await {
                return response;
            }
            error_response("Reencryption failed with all methods")
        }
        Err(e) => error_response(e),
    }
}
```

## 9. 監視とメトリクス

### 9.1 Holder固有メトリクス

```rust
/// Holderパフォーマンスメトリクス
#[derive(Debug, Clone, Serialize)]
pub struct HolderMetrics {
    // ストレージメトリクス
    pub total_kfrags: u64,
    pub active_kfrags: u64,
    pub expired_kfrags: u64,
    pub storage_utilization: f64,
    
    // パフォーマンスメトリクス
    pub total_reencryptions: u64,
    pub successful_reencryptions: u64,
    pub failed_reencryptions: u64,
    pub average_reencryption_time: Duration,
    pub p99_reencryption_time: Duration,
    
    // 可用性メトリクス
    pub uptime_percentage: f64,
    pub last_downtime: Option<SystemTime>,
    pub response_rate: f64,
    
    // 信頼性メトリクス
    pub trust_score: f64,
    pub verification_success_rate: f64,
}

impl HolderMetrics {
    pub async fn collect(ctx: &HandlerContext) -> Result<Self, MetricsError> {
        // 各種メトリクスの収集
        let storage_metrics = collect_storage_metrics(ctx).await?;
        let performance_metrics = collect_performance_metrics(ctx).await?;
        let availability_metrics = collect_availability_metrics(ctx).await?;
        
        Ok(Self {
            total_kfrags: storage_metrics.total,
            active_kfrags: storage_metrics.active,
            expired_kfrags: storage_metrics.expired,
            storage_utilization: storage_metrics.utilization,
            // ... 他のフィールド
        })
    }
}
```

## 10. テスト

### 10.1 単体テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_kfrag_storage() {
        let ctx = create_test_holder_context();
        let kfrag_data = create_test_kfrag();
        
        let msg = create_store_kfrag_message(kfrag_data);
        let response = handle_store_kfrag(msg).await;
        
        assert!(is_success(&response));
        assert!(get_stored_kfrag(&ctx, &kfrag_data.id).await.is_ok());
    }
    
    #[tokio::test]
    async fn test_reencryption_authorization() {
        let ctx = create_test_holder_context();
        let request = create_unauthorized_request();
        
        let result = verify_access_authorization(&ctx, &request).await;
        
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            AuthorizationError::NotApproved
        );
    }
    
    #[tokio::test]
    async fn test_parallel_reencryption() {
        let requests = (0..10)
            .map(|i| create_test_reencryption_request(i))
            .collect();
        
        let results = parallel_reencryption_handler(requests).await;
        
        assert_eq!(results.len(), 10);
        assert!(results.iter().all(|r| r.is_ok()));
    }
}
```

### 10.2 統合テスト

```rust
#[tokio::test]
async fn test_complete_holder_workflow() {
    // Holder プロセスの初期化
    let holder_process = spawn_holder_process().await;
    
    // Phase 3: kFrag受信
    let kfrag_msg = create_kfrag_distribution_message();
    let store_response = send_message(&holder_process, kfrag_msg).await;
    assert!(is_success(&store_response));
    
    // Phase 4: 再暗号化実行
    let reenc_msg = create_reencryption_request_message();
    let reenc_response = send_message(&holder_process, reenc_msg).await;
    assert!(is_success(&reenc_response));
    
    // cFragの検証
    let cfrag = extract_cfrag(&reenc_response);
    assert!(verify_cfrag(&cfrag).is_ok());
    
    // ステータス確認
    let status_msg = create_status_request_message();
    let status_response = send_message(&holder_process, status_msg).await;
    
    let status = parse_holder_status(&status_response);
    assert_eq!(status.total_reencryptions, 1);
    assert!(status.trust_score > DEFAULT_TRUST_SCORE);
}
```

## 11. まとめ

Holder Roleハンドラーは、FORMIXシステムの分散性と信頼性を支える重要な要素として、以下の特性を実現します：

1. **安全なkFrag管理**: 暗号学的に保護された鍵フラグメントの保存
2. **高性能再暗号化**: 並列処理とキャッシングによる最適化
3. **厳格な検証**: 多層的な暗号学的検証
4. **高可用性**: 信頼スコアとSLAコミットメント
5. **包括的な監査**: すべての操作の追跡可能性

これらの設計により、k-of-n閾値暗号化スキームの信頼できる実行を保証します。

---

**Document Status**: Holder Role Handlers Specification  
**Version**: 1.0  
**Last Updated**: 2025-01-07