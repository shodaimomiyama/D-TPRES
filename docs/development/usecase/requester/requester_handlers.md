# Requester Role Handlers 詳細設計

## 1. 概要

Requester Roleは、D-TPRESシステムにおいて秘密へのアクセスを要求し、暗号フラグメントを収集して秘密を復元する重要な役割を担います。Requesterは、EVMスマートコントラクトによるアクセス権限の検証と、k-of-n閾値暗号の復号プロセスを調整します。

### 1.1 Requester Roleの責務

1. **アクセス要求管理**
   - 秘密へのアクセス要求の作成
   - EVM検証の調整と証明の提出
   - アクセス要求のステータス管理

2. **cFrag収集とk-of-n達成**
   - Holderからの暗号フラグメント収集
   - 閾値達成の監視
   - タイムアウト管理とリトライ

3. **秘密復元**
   - 収集したcFragからの秘密復元
   - 復号結果の検証
   - エラーハンドリングと復元戦略

### 1.2 設計原則

- **並列処理**: 複数のHolderへの同時リクエスト
- **フォールトトレランス**: 一部のHolderが応答しなくても閾値達成可能
- **タイムアウト管理**: 応答待機時間の適切な制御
- **進捗可視性**: アクセス要求の進行状況の透明性

## 2. ハンドラー登録

### 2.1 Requester専用ハンドラーの登録

```rust
use ao_sdk::{Handlers, Message, Response};

/// Requesterロール用ハンドラーの登録
pub fn register_requester_handlers() -> Result<(), HandlerError> {
    // Phase 2: アクセス要求
    Handlers::add(
        "access-request",
        Handlers::utils::hasMatchingTag("Action", "Access-Request"),
        handle_access_request
    );
    
    Handlers::add(
        "submit-proof",
        Handlers::utils::hasMatchingTag("Action", "Submit-Proof"),
        handle_submit_proof
    );
    
    // Phase 4: cFrag収集
    Handlers::add(
        "request-reencryption",
        Handlers::utils::hasMatchingTag("Action", "Request-Reencryption"),
        handle_request_reencryption
    );
    
    Handlers::add(
        "collect-cfrag",
        Handlers::utils::hasMatchingTag("Action", "Collect-CFrag"),
        handle_collect_cfrag
    );
    
    // Phase 5: 秘密復元
    Handlers::add(
        "recover-secret",
        Handlers::utils::hasMatchingTag("Action", "Recover-Secret"),
        handle_recover_secret
    );
    
    // 管理系
    Handlers::add(
        "check-access-status",
        Handlers::utils::hasMatchingTag("Action", "Check-Access-Status"),
        handle_check_access_status
    );
    
    Handlers::add(
        "cancel-access-request",
        Handlers::utils::hasMatchingTag("Action", "Cancel-Access-Request"),
        handle_cancel_access_request
    );
    
    Handlers::add(
        "retry-failed-holders",
        Handlers::utils::hasMatchingTag("Action", "Retry-Failed-Holders"),
        handle_retry_failed_holders
    );
    
    Ok(())
}
```

### 2.2 Requesterロール検証

```rust
/// Requesterロール検証デコレータ
pub fn require_requester_role<F>(handler: F) -> impl Fn(Message) -> Response
where
    F: Fn(Message) -> Response,
{
    move |msg: Message| {
        // コンテキスト初期化とロール検証
        let ctx = match initialize_and_verify_requester().await {
            Ok(ctx) => ctx,
            Err(e) => return unauthorized_response(e),
        };
        
        // 実際のハンドラー実行
        handler(msg)
    }
}

async fn initialize_and_verify_requester() -> Result<HandlerContext, RoleError> {
    let ctx = HandlerContext::initialize().await?;
    
    if !ctx.process.has_role(ProcessRole::Requester) {
        return Err(RoleError::Unauthorized {
            required: ProcessRole::Requester,
            actual: ctx.process.get_primary_role(),
        });
    }
    
    Ok(ctx)
}
```

## 3. Phase 2: アクセス要求ハンドラー

### 3.1 Access-Request

```rust
/// アクセス要求ハンドラー
pub async fn handle_access_request(msg: Message) -> Response {
    // コンテキスト初期化とRequesterロール検証
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 前提条件チェック
    if let Err(e) = check_access_request_preconditions(&ctx, &msg).await {
        return precondition_failed_response(e);
    }
    
    // アクセス要求パラメータの抽出
    let request_params = match extract_access_request_params(&msg) {
        Ok(params) => params,
        Err(e) => return validation_error_response(e),
    };
    
    // 秘密の存在確認
    if let Err(e) = verify_secret_exists(&ctx, &request_params.secret_id).await {
        return error_response(format!("Secret not found: {}", e));
    }
    
    // 重複リクエストチェック
    if has_active_request(&ctx, &request_params.secret_id).await {
        return error_response("Active request already exists for this secret");
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の処理
    if is_success(&response) {
        // 監査ログ
        audit_log(AuditEvent::AccessRequested {
            secret_id: request_params.secret_id.clone(),
            requester: ctx.process.process_id.clone(),
            conditions: request_params.conditions.clone(),
            timestamp: SystemTime::now(),
        }).await;
    }
    
    response
}

/// アクセス要求の前提条件チェック
async fn check_access_request_preconditions(
    ctx: &HandlerContext,
    msg: &Message,
) -> Result<(), PreconditionError> {
    // 1. プロセスが初期化済みか
    if !ctx.process.is_initialized() {
        return Err(PreconditionError::NotInitialized);
    }
    
    // 2. アクティブなリクエスト数の上限チェック
    let active_requests = ctx.process.requester_data.as_ref()
        .map(|d| d.active_requests.len())
        .unwrap_or(0);
        
    if active_requests >= MAX_CONCURRENT_REQUESTS {
        return Err(PreconditionError::LimitExceeded {
            resource: "active_requests",
            limit: MAX_CONCURRENT_REQUESTS,
            current: active_requests,
        });
    }
    
    Ok(())
}

/// アクセス要求パラメータの抽出と検証
fn extract_access_request_params(msg: &Message) -> Result<AccessRequestParams, ValidationError> {
    let secret_id = msg.tags.get("Secret-Id")
        .ok_or(ValidationError::MissingField("Secret-Id"))?;
        
    let conditions = match msg.tags.get("Conditions") {
        Some(c) => serde_json::from_str(c)
            .map_err(|e| ValidationError::InvalidFormat("Conditions", e))?,
        None => AccessConditions::default(),
    };
    
    // EVM証明の検証
    let proof = msg.tags.get("EVM-Proof")
        .ok_or(ValidationError::MissingField("EVM-Proof"))?;
        
    if !is_valid_proof_format(proof) {
        return Err(ValidationError::InvalidProof);
    }
    
    Ok(AccessRequestParams {
        secret_id: secret_id.to_string(),
        conditions,
        proof: proof.to_string(),
    })
}
```

### 3.2 Submit-Proof

```rust
/// EVM証明提出ハンドラー
pub async fn handle_submit_proof(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // アクセス要求IDの取得
    let request_id = match msg.tags.get("Request-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Request-Id"),
    };
    
    // アクセス要求の存在確認
    let access_request = match load_access_request(&ctx, request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Request not found: {}", e)),
    };
    
    // リクエスタの確認
    if access_request.requester_id != ctx.process.process_id {
        return error_response("Not the request owner");
    }
    
    // EVM証明の検証
    let proof_data = match extract_and_verify_proof(&msg).await {
        Ok(proof) => proof,
        Err(e) => return validation_error_response(e),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}
```

## 4. Phase 4: cFrag収集ハンドラー

### 4.1 Request-Reencryption

```rust
/// 再暗号化要求ハンドラー
pub async fn handle_request_reencryption(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // アクセス要求の確認
    let request_id = match msg.tags.get("Request-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Request-Id"),
    };
    
    let access_request = match load_verified_access_request(&ctx, request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Invalid access request: {}", e)),
    };
    
    // Holder選択戦略の実行
    let target_holders = match select_target_holders(&ctx, &access_request).await {
        Ok(holders) => holders,
        Err(e) => return error_response(format!("Holder selection failed: {}", e)),
    };
    
    // 並列再暗号化要求の準備
    let reencryption_params = prepare_reencryption_params(
        &access_request,
        &target_holders,
    );
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}

/// ターゲットHolder選択アルゴリズム
async fn select_target_holders(
    ctx: &HandlerContext,
    request: &AccessRequest,
) -> Result<Vec<HolderTarget>, SelectionError> {
    // kFrag配布情報の取得
    let kfrag_distribution = get_kfrag_distribution(&request.secret_id).await?;
    
    // 利用可能なHolderのフィルタリング
    let available_holders = kfrag_distribution.holders
        .into_iter()
        .filter(|h| {
            h.is_active && 
            h.trust_score >= MIN_TRUST_SCORE &&
            !is_blacklisted(&h.holder_id)
        })
        .collect::<Vec<_>>();
    
    // 閾値達成の確認
    if available_holders.len() < request.threshold {
        return Err(SelectionError::InsufficientHolders {
            required: request.threshold,
            available: available_holders.len(),
        });
    }
    
    // 最適化：応答時間とネットワーク遅延を考慮
    let mut scored_holders = score_holders_for_selection(
        &available_holders,
        &ctx.network_metrics,
    );
    
    // 必要数より多めに選択（フォールトトレランス）
    let selection_count = min(
        request.threshold + REDUNDANCY_FACTOR,
        scored_holders.len()
    );
    
    Ok(scored_holders.drain(..selection_count).collect())
}
```

### 4.2 Collect-CFrag

```rust
/// cFrag収集ハンドラー
pub async fn handle_collect_cfrag(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // cFragデータの抽出
    let cfrag_data = match extract_cfrag_data(&msg) {
        Ok(data) => data,
        Err(e) => return validation_error_response(e),
    };
    
    // アクセス要求との関連付け
    let access_request = match verify_cfrag_association(&ctx, &cfrag_data).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Invalid cFrag association: {}", e)),
    };
    
    // cFragの暗号学的検証
    if let Err(e) = verify_cfrag_integrity(&cfrag_data, &access_request) {
        return error_response(format!("cFrag verification failed: {}", e));
    }
    
    // cFragの保存と閾値チェック
    let collection_status = match store_and_check_threshold(
        &ctx,
        &access_request,
        cfrag_data,
    ).await {
        Ok(status) => status,
        Err(e) => return error_response(e),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 閾値達成時の通知
    if collection_status.threshold_reached {
        notify_threshold_reached(&access_request).await;
    }
    
    response
}

/// cFragの保存と閾値達成チェック
async fn store_and_check_threshold(
    ctx: &HandlerContext,
    request: &AccessRequest,
    cfrag: CFragData,
) -> Result<CollectionStatus, CollectionError> {
    // cFragの重複チェック
    if is_duplicate_cfrag(&request.request_id, &cfrag.holder_id).await? {
        return Err(CollectionError::DuplicateCFrag);
    }
    
    // cFragの保存
    save_cfrag(&request.request_id, &cfrag).await?;
    
    // 現在の収集状況を確認
    let collected_cfrags = get_collected_cfrags(&request.request_id).await?;
    let valid_count = collected_cfrags.iter()
        .filter(|cf| cf.is_valid)
        .count();
    
    Ok(CollectionStatus {
        total_collected: collected_cfrags.len(),
        valid_cfrags: valid_count,
        threshold: request.threshold as usize,
        threshold_reached: valid_count >= request.threshold as usize,
        missing_count: request.threshold.saturating_sub(valid_count as u8),
    })
}
```

## 5. Phase 5: 秘密復元ハンドラー

### 5.1 Recover-Secret

```rust
/// 秘密復元ハンドラー
pub async fn handle_recover_secret(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // アクセス要求の取得
    let request_id = match msg.tags.get("Request-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Request-Id"),
    };
    
    let access_request = match load_completed_access_request(&ctx, request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Request not ready: {}", e)),
    };
    
    // 閾値達成の確認
    let cfrags = match verify_threshold_reached(&access_request).await {
        Ok(cfrags) => cfrags,
        Err(e) => return error_response(format!("Threshold not met: {}", e)),
    };
    
    // 復元プロセスの実行
    let recovery_result = match execute_recovery(&access_request, &cfrags).await {
        Ok(result) => result,
        Err(e) => return error_response(format!("Recovery failed: {}", e)),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の処理
    if is_success(&response) {
        // 復元成功の記録
        record_successful_recovery(&access_request, &recovery_result).await;
        
        // 監査ログ
        audit_log(AuditEvent::SecretRecovered {
            request_id: request_id.clone(),
            secret_id: access_request.secret_id.clone(),
            requester: ctx.process.process_id.clone(),
            cfrags_used: cfrags.len(),
            timestamp: SystemTime::now(),
        }).await;
    }
    
    response
}

/// 秘密復元の実行
async fn execute_recovery(
    request: &AccessRequest,
    cfrags: &[ValidatedCFrag],
) -> Result<RecoveryResult, RecoveryError> {
    // カプセルの取得
    let capsules = load_capsules(&request.secret_id).await?;
    
    // cFragの準備
    let prepared_cfrags = prepare_cfrags_for_recovery(cfrags, &capsules)?;
    
    // 暗号学的復元処理
    let decrypted_shares = decrypt_with_cfrags(
        &prepared_cfrags,
        &capsules,
        &request.requester_public_key,
    )?;
    
    // Shamir Secret Sharingによる秘密復元
    let recovered_secret = recover_from_shares(&decrypted_shares)?;
    
    // 復元結果の検証
    verify_recovery_integrity(&recovered_secret, &request)?;
    
    Ok(RecoveryResult {
        request_id: request.request_id.clone(),
        secret_id: request.secret_id.clone(),
        recovered_at: SystemTime::now(),
        cfrags_used: cfrags.len(),
        verification_hash: compute_verification_hash(&recovered_secret),
    })
}
```

## 6. 管理系ハンドラー

### 6.1 Check-Access-Status

```rust
/// アクセス状態確認ハンドラー
pub async fn handle_check_access_status(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 対象の特定
    let target = match determine_status_target(&msg) {
        Ok(target) => target,
        Err(e) => return validation_error_response(e),
    };
    
    // ステータス情報の収集
    let status = match target {
        StatusTarget::SingleRequest(id) => {
            collect_request_status(&ctx, &id).await
        },
        StatusTarget::AllRequests => {
            collect_all_requests_status(&ctx).await
        },
        StatusTarget::SecretSpecific(secret_id) => {
            collect_secret_requests_status(&ctx, &secret_id).await
        },
    };
    
    // レスポンス生成
    match status {
        Ok(status_data) => Response {
            tags: vec![
                ("Status", "Success"),
                ("Action", "Check-Access-Status-Response"),
            ],
            data: serde_json::to_vec(&status_data).unwrap_or_default(),
        },
        Err(e) => error_response(e),
    }
}

/// 単一リクエストのステータス収集
async fn collect_request_status(
    ctx: &HandlerContext,
    request_id: &str,
) -> Result<RequestStatus, StatusError> {
    let request = ctx.repositories.access_request_repo
        .find_by_id(request_id)
        .await?
        .ok_or(StatusError::RequestNotFound)?;
    
    // cFrag収集状況
    let cfrags = ctx.repositories.reencryption_repo
        .find_by_request_id(request_id)
        .await?;
    
    let cfrag_summary = CFragCollectionSummary {
        total_expected: request.threshold as usize + REDUNDANCY_FACTOR,
        collected: cfrags.len(),
        valid: cfrags.iter().filter(|cf| cf.is_valid).count(),
        failed: cfrags.iter().filter(|cf| cf.is_failed).count(),
        pending: request.pending_holders.len(),
    };
    
    Ok(RequestStatus {
        request_id: request_id.to_string(),
        secret_id: request.secret_id.clone(),
        status: request.status.clone(),
        created_at: request.created_at,
        last_updated: request.updated_at,
        cfrag_collection: cfrag_summary,
        estimated_completion: estimate_completion_time(&request, &cfrags),
    })
}
```

### 6.2 Cancel-Access-Request

```rust
/// アクセス要求キャンセルハンドラー
pub async fn handle_cancel_access_request(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // キャンセル対象の取得
    let request_id = match msg.tags.get("Request-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Request-Id"),
    };
    
    // 権限確認
    let access_request = match verify_cancel_permission(&ctx, request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Cannot cancel: {}", e)),
    };
    
    // キャンセル可能性チェック
    if let Err(e) = check_cancellation_eligibility(&access_request) {
        return error_response(e);
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の処理
    if is_success(&response) {
        // 関連プロセスへの通知
        notify_cancellation(&access_request).await;
        
        // 監査ログ
        audit_log(AuditEvent::RequestCancelled {
            request_id: request_id.clone(),
            reason: extract_cancellation_reason(&msg),
            timestamp: SystemTime::now(),
        }).await;
    }
    
    response
}
```

### 6.3 Retry-Failed-Holders

```rust
/// 失敗Holderリトライハンドラー
pub async fn handle_retry_failed_holders(msg: Message) -> Response {
    let ctx = match initialize_and_verify_requester().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // リトライ対象の特定
    let retry_params = match extract_retry_params(&msg) {
        Ok(params) => params,
        Err(e) => return validation_error_response(e),
    };
    
    // アクセス要求の確認
    let access_request = match load_active_request(&ctx, &retry_params.request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Invalid request: {}", e)),
    };
    
    // 失敗Holderの特定
    let failed_holders = identify_failed_holders(&access_request, &retry_params).await;
    
    if failed_holders.is_empty() {
        return error_response("No failed holders to retry");
    }
    
    // 代替Holder選択
    let alternative_holders = match select_alternative_holders(
        &ctx,
        &access_request,
        &failed_holders,
    ).await {
        Ok(holders) => holders,
        Err(e) => return error_response(format!("No alternatives available: {}", e)),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}

/// 代替Holder選択アルゴリズム
async fn select_alternative_holders(
    ctx: &HandlerContext,
    request: &AccessRequest,
    failed_holders: &[HolderId],
) -> Result<Vec<HolderTarget>, SelectionError> {
    // 既に試行済みのHolderを除外
    let attempted_holders = get_attempted_holders(&request.request_id).await?;
    
    // 利用可能なHolderのリスト
    let available_holders = get_kfrag_holders(&request.secret_id).await?
        .into_iter()
        .filter(|h| {
            !attempted_holders.contains(&h.holder_id) &&
            !failed_holders.contains(&h.holder_id) &&
            h.is_active &&
            h.trust_score >= MIN_RETRY_TRUST_SCORE
        })
        .collect::<Vec<_>>();
    
    // 必要数の確認
    let needed = failed_holders.len();
    if available_holders.len() < needed {
        return Err(SelectionError::InsufficientAlternatives {
            needed,
            available: available_holders.len(),
        });
    }
    
    // パフォーマンスメトリクスに基づく選択
    let mut scored = score_holders_by_performance(available_holders);
    scored.truncate(needed);
    
    Ok(scored)
}
```

## 7. 並列処理とパフォーマンス最適化

### 7.1 並列cFrag収集

```rust
/// 並列再暗号化要求
pub async fn parallel_reencryption_requests(
    request: &AccessRequest,
    holders: Vec<HolderTarget>,
) -> Vec<Result<CFragResponse, RequestError>> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_HOLDER_REQUESTS));
    
    let futures = holders.into_iter().map(|holder| {
        let sem = semaphore.clone();
        let req = request.clone();
        
        async move {
            let _permit = sem.acquire().await?;
            
            // タイムアウト付きリクエスト
            tokio::time::timeout(
                HOLDER_REQUEST_TIMEOUT,
                send_reencryption_request(&req, &holder)
            ).await
            .map_err(|_| RequestError::Timeout)?
        }
    });
    
    futures::future::join_all(futures).await
}
```

### 7.2 進捗トラッキング

```rust
/// リアルタイム進捗管理
pub struct ProgressTracker {
    request_id: String,
    total_holders: usize,
    responses: Arc<Mutex<Vec<HolderResponse>>>,
    threshold: u8,
}

impl ProgressTracker {
    pub async fn track_response(&self, response: HolderResponse) -> ProgressUpdate {
        let mut responses = self.responses.lock().await;
        responses.push(response);
        
        let valid_count = responses.iter()
            .filter(|r| matches!(r, HolderResponse::Success(_)))
            .count();
        
        ProgressUpdate {
            total_sent: self.total_holders,
            received: responses.len(),
            successful: valid_count,
            failed: responses.iter()
                .filter(|r| matches!(r, HolderResponse::Failed(_)))
                .count(),
            pending: self.total_holders - responses.len(),
            threshold_reached: valid_count >= self.threshold as usize,
            completion_percentage: (valid_count * 100 / self.threshold as usize).min(100),
        }
    }
    
    pub async fn estimate_completion(&self) -> Option<Duration> {
        let responses = self.responses.lock().await;
        
        if responses.is_empty() {
            return None;
        }
        
        // 応答時間の統計から推定
        let avg_response_time = calculate_average_response_time(&responses);
        let remaining = self.threshold as usize - responses.len();
        
        Some(avg_response_time * remaining as u32)
    }
}
```

### 7.3 適応的タイムアウト

```rust
/// ネットワーク状況に基づく動的タイムアウト
pub struct AdaptiveTimeout {
    base_timeout: Duration,
    network_metrics: Arc<RwLock<NetworkMetrics>>,
}

impl AdaptiveTimeout {
    pub async fn calculate_timeout(&self, holder: &HolderId) -> Duration {
        let metrics = self.network_metrics.read().await;
        
        // Holderごとの履歴から計算
        let holder_metrics = metrics.get_holder_metrics(holder);
        
        let adjusted_timeout = match holder_metrics {
            Some(m) => {
                // 過去の応答時間の95パーセンタイル + バッファ
                let p95 = m.response_time_p95();
                let buffer = p95 * TIMEOUT_BUFFER_FACTOR;
                p95 + buffer
            }
            None => {
                // 新規Holderはデフォルト値
                self.base_timeout
            }
        };
        
        // 上限と下限の適用
        adjusted_timeout.clamp(MIN_TIMEOUT, MAX_TIMEOUT)
    }
}
```

## 8. エラー処理とリカバリー

### 8.1 Requester固有のエラー

```rust
#[derive(Debug, thiserror::Error)]
pub enum RequesterError {
    #[error("Not authorized as requester")]
    NotRequester,
    
    #[error("Access request limit exceeded: {current}/{max}")]
    RequestLimitExceeded { current: usize, max: usize },
    
    #[error("Insufficient cFrags: collected={collected}, required={required}")]
    InsufficientCFrags { collected: usize, required: usize },
    
    #[error("Recovery failed: {reason}")]
    RecoveryFailed { reason: String },
    
    #[error("Holder communication failed: {holder_id}")]
    HolderCommunicationFailed { holder_id: String },
    
    #[error("Timeout waiting for cFrags")]
    CollectionTimeout,
    
    #[error("Invalid proof submitted")]
    InvalidProof,
}
```

### 8.2 エラーリカバリー戦略

```rust
/// エラー回復メカニズム
pub struct ErrorRecovery {
    retry_policy: RetryPolicy,
    fallback_strategy: FallbackStrategy,
}

impl ErrorRecovery {
    pub async fn handle_holder_failure(
        &self,
        error: HolderError,
        context: &RequestContext,
    ) -> Result<RecoveryAction, RecoveryError> {
        match error {
            HolderError::Timeout => {
                // タイムアウトの場合は代替Holderを選択
                Ok(RecoveryAction::SelectAlternative)
            }
            HolderError::InvalidResponse => {
                // 不正な応答は記録してブラックリスト
                blacklist_holder(&error.holder_id, Duration::from_secs(3600)).await;
                Ok(RecoveryAction::SelectAlternative)
            }
            HolderError::Unavailable => {
                // 一時的に利用不可の場合はリトライ
                if self.retry_policy.should_retry(&error) {
                    Ok(RecoveryAction::Retry {
                        delay: self.retry_policy.next_delay(),
                    })
                } else {
                    Ok(RecoveryAction::SelectAlternative)
                }
            }
            _ => Err(RecoveryError::Unrecoverable(error)),
        }
    }
}
```

## 9. セキュリティ実装

### 9.1 アクセス権限の厳密な検証

```rust
/// 多層防御によるアクセス制御
pub async fn verify_access_authorization(
    ctx: &HandlerContext,
    request: &AccessRequest,
) -> Result<(), AuthorizationError> {
    // 1. プロセスレベルの検証
    if !ctx.process.has_role(ProcessRole::Requester) {
        return Err(AuthorizationError::InvalidRole);
    }
    
    // 2. リクエスト所有権の検証
    if request.requester_id != ctx.process.process_id {
        return Err(AuthorizationError::NotOwner);
    }
    
    // 3. EVM証明の検証
    let proof_valid = verify_evm_proof(
        &request.proof,
        &request.conditions,
        &request.secret_id,
    ).await?;
    
    if !proof_valid {
        return Err(AuthorizationError::InvalidProof);
    }
    
    // 4. タイムスタンプ検証
    if request.expires_at < SystemTime::now() {
        return Err(AuthorizationError::Expired);
    }
    
    Ok(())
}
```

### 9.2 cFragの完全性検証

```rust
/// cFragの暗号学的検証
pub fn verify_cfrag_integrity(
    cfrag: &CFragData,
    request: &AccessRequest,
) -> Result<(), IntegrityError> {
    // 1. 署名検証
    let signature_valid = verify_holder_signature(
        &cfrag.data,
        &cfrag.signature,
        &cfrag.holder_public_key,
    )?;
    
    if !signature_valid {
        return Err(IntegrityError::InvalidSignature);
    }
    
    // 2. cFragの暗号学的検証
    let cfrag_valid = umbral_verify_cfrag(
        &cfrag.cfrag,
        &request.capsule,
        &request.verifying_key,
    )?;
    
    if !cfrag_valid {
        return Err(IntegrityError::InvalidCFrag);
    }
    
    // 3. タイムスタンプ検証
    let timestamp_valid = verify_timestamp(
        &cfrag.timestamp,
        MAX_CFRAG_AGE,
    );
    
    if !timestamp_valid {
        return Err(IntegrityError::StaleData);
    }
    
    Ok(())
}
```

## 10. 監査とコンプライアンス

### 10.1 アクセスログ

```rust
/// 包括的な監査ログ
#[derive(Debug, Serialize)]
pub struct AccessAuditLog {
    pub request_id: String,
    pub secret_id: String,
    pub requester_id: ProcessId,
    pub action: AccessAction,
    pub timestamp: SystemTime,
    pub metadata: AccessMetadata,
}

#[derive(Debug, Serialize)]
pub enum AccessAction {
    RequestInitiated {
        conditions: AccessConditions,
    },
    ProofSubmitted {
        proof_hash: String,
        verification_result: bool,
    },
    CFragCollected {
        holder_id: HolderId,
        cfrag_hash: String,
    },
    ThresholdReached {
        collected: usize,
        threshold: usize,
    },
    RecoveryCompleted {
        duration: Duration,
        cfrags_used: usize,
    },
    RequestCancelled {
        reason: String,
    },
}

/// 監査ログの永続化
async fn persist_audit_log(log: AccessAuditLog) -> Result<(), AuditError> {
    // Arweaveへの永続化
    let tx_id = store_to_arweave(&log).await?;
    
    // インデックスの更新
    update_audit_index(&log, &tx_id).await?;
    
    // コンプライアンス要件に応じた追加処理
    if requires_regulatory_reporting(&log) {
        submit_regulatory_report(&log).await?;
    }
    
    Ok(())
}
```

## 11. テスト

### 11.1 単体テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_threshold_calculation() {
        let cfrags = vec![
            create_test_cfrag("holder1", true),
            create_test_cfrag("holder2", true),
            create_test_cfrag("holder3", false), // 無効
            create_test_cfrag("holder4", true),
        ];
        
        let threshold = 3;
        let status = calculate_collection_status(&cfrags, threshold);
        
        assert_eq!(status.valid_cfrags, 3);
        assert!(status.threshold_reached);
    }
    
    #[tokio::test]
    async fn test_holder_selection_strategy() {
        let holders = create_test_holders(10);
        let ctx = create_test_context();
        let request = create_test_access_request();
        
        let selected = select_target_holders(&ctx, &request).await.unwrap();
        
        // 閾値 + 冗長性分が選択されること
        assert_eq!(selected.len(), request.threshold as usize + REDUNDANCY_FACTOR);
        
        // スコアの高い順に選択されていること
        for i in 0..selected.len()-1 {
            assert!(selected[i].score >= selected[i+1].score);
        }
    }
    
    #[tokio::test]
    async fn test_parallel_collection_timeout() {
        let holders = create_slow_test_holders(5);
        let request = create_test_access_request();
        
        let start = Instant::now();
        let results = parallel_reencryption_requests(&request, holders).await;
        let duration = start.elapsed();
        
        // タイムアウトが機能していること
        assert!(duration < HOLDER_REQUEST_TIMEOUT + Duration::from_millis(100));
        
        // タイムアウトエラーが含まれること
        let timeout_count = results.iter()
            .filter(|r| matches!(r, Err(RequestError::Timeout)))
            .count();
        assert!(timeout_count > 0);
    }
}
```

### 11.2 統合テスト

```rust
#[tokio::test]
async fn test_complete_access_flow() {
    // Requester プロセスの初期化
    let requester_process = spawn_requester_process().await;
    
    // Phase 2: アクセス要求
    let access_response = send_access_request_message(
        &requester_process,
        "secret-123"
    ).await;
    assert!(is_success(&access_response));
    
    let request_id = get_request_id(&access_response);
    
    // EVM証明の提出
    let proof_response = send_submit_proof_message(
        &requester_process,
        &request_id,
        &create_test_proof()
    ).await;
    assert!(is_success(&proof_response));
    
    // Phase 4: 再暗号化要求
    let reencrypt_response = send_request_reencryption_message(
        &requester_process,
        &request_id
    ).await;
    assert!(is_success(&reencrypt_response));
    
    // cFragの収集（シミュレート）
    for i in 0..3 {
        let cfrag_response = simulate_cfrag_collection(
            &requester_process,
            &request_id,
            &format!("holder-{}", i)
        ).await;
        assert!(is_success(&cfrag_response));
    }
    
    // Phase 5: 秘密復元
    let recover_response = send_recover_secret_message(
        &requester_process,
        &request_id
    ).await;
    assert!(is_success(&recover_response));
    
    // 復元結果の検証
    let recovery_data = parse_recovery_response(&recover_response);
    assert_eq!(recovery_data.secret_id, "secret-123");
    assert!(recovery_data.verification_hash.is_some());
}
```

## 12. まとめ

Requester Roleハンドラーは、D-TPRESシステムにおける秘密へのアクセスと復元プロセスの中核として、以下の特性を持ちます：

1. **堅牢なアクセス制御**: 多層防御による厳密な権限検証
2. **効率的なcFrag収集**: 並列処理とフォールトトレランス
3. **適応的な処理**: ネットワーク状況に応じた動的調整
4. **包括的なエラー処理**: 自動リトライと代替戦略
5. **透明性の確保**: 詳細な進捗トラッキングと監査ログ

これらの設計により、安全で信頼性の高い秘密復元プロセスを実現します。

---

**Document Status**: Requester Role Handlers Specification  
**Version**: 1.0  
**Last Updated**: 2025-01-07