# Owner Role Handlers 詳細設計

## 1. 概要

Owner Roleは、FORMIXシステムにおいて最も重要な権限を持つロールです。秘密データの管理、アクセス制御の設定、再暗号化キーの生成など、システムの中核となる操作を担当します。本ドキュメントでは、Owner Role専用のハンドラー設計について詳述します。

### 1.1 Owner Roleの責務

1. **秘密管理**
   - 秘密データの分割（Shamir Secret Sharing）
   - カプセルの生成と管理
   - 秘密のライフサイクル管理

2. **アクセス制御**
   - アクセス条件の設定
   - 権限の付与と取り消し
   - 監査ログの管理

3. **再暗号化キー管理**
   - ReKeyの生成
   - kFragの分割と配布
   - Holder選択戦略

### 1.2 セキュリティ原則

- **秘密鍵の保護**: Owner秘密鍵（skO）は最高レベルの保護が必要
- **最小権限**: 各操作は必要最小限の権限で実行
- **監査証跡**: すべての操作は追跡可能

## 2. ハンドラー登録

### 2.1 Owner専用ハンドラーの登録

```rust
use ao_sdk::{Handlers, Message, Response};

/// Ownerロール用ハンドラーの登録
pub fn register_owner_handlers() -> Result<(), HandlerError> {
    // Phase 0: 初期化
    Handlers::add(
        "initialize-owner",
        Handlers::utils::hasMatchingTag("Action", "Initialize-Process"),
        handle_initialize_process
    );
    
    // Phase 1: 秘密分割
    Handlers::add(
        "split-secret",
        Handlers::utils::hasMatchingTag("Action", "Split-Secret"),
        handle_split_secret
    );
    
    Handlers::add(
        "store-share",
        Handlers::utils::hasMatchingTag("Action", "Store-Share"),
        handle_store_share
    );
    
    Handlers::add(
        "store-capsule",
        Handlers::utils::hasMatchingTag("Action", "Store-Capsule"),
        handle_store_capsule
    );
    
    // Phase 3: 再暗号化キー管理
    Handlers::add(
        "generate-rekey",
        Handlers::utils::hasMatchingTag("Action", "Generate-ReKey"),
        handle_generate_rekey
    );
    
    Handlers::add(
        "distribute-kfrag",
        Handlers::utils::hasMatchingTag("Action", "Distribute-KFrag"),
        handle_distribute_kfrag
    );
    
    // 管理系
    Handlers::add(
        "update-access-control",
        Handlers::utils::hasMatchingTag("Action", "Update-Access-Control"),
        handle_update_access_control
    );
    
    Handlers::add(
        "revoke-access",
        Handlers::utils::hasMatchingTag("Action", "Revoke-Access"),
        handle_revoke_access
    );
    
    Handlers::add(
        "get-secret-status",
        Handlers::utils::hasMatchingTag("Action", "Get-Secret-Status"),
        handle_get_secret_status
    );
    
    Ok(())
}
```

### 2.2 ハンドラー実行前のロール検証

```rust
/// Ownerロール検証デコレータ
pub fn require_owner_role<F>(handler: F) -> impl Fn(Message) -> Response
where
    F: Fn(Message) -> Response,
{
    move |msg: Message| {
        // コンテキスト初期化とロール検証
        let ctx = match initialize_and_verify_owner().await {
            Ok(ctx) => ctx,
            Err(e) => return unauthorized_response(e),
        };
        
        // 実際のハンドラー実行
        handler(msg)
    }
}

async fn initialize_and_verify_owner() -> Result<HandlerContext, RoleError> {
    let ctx = HandlerContext::initialize().await?;
    
    if !ctx.process.has_role(ProcessRole::Owner) {
        return Err(RoleError::Unauthorized {
            required: ProcessRole::Owner,
            actual: ctx.process.get_primary_role(),
        });
    }
    
    Ok(ctx)
}
```

## 3. Phase 0: 初期化ハンドラー

### 3.1 Initialize-Process

```rust
/// プロセス初期化ハンドラー
pub async fn handle_initialize_process(msg: Message) -> Response {
    let ctx = match HandlerContext::initialize().await {
        Ok(ctx) => ctx,
        Err(e) => return error_response(e),
    };
    
    // 既に初期化済みの場合はエラー
    if ctx.process.is_initialized() {
        return error_response("Process already initialized");
    }
    
    // 初期化パラメータの抽出
    let init_params = match extract_init_params(&msg) {
        Ok(params) => params,
        Err(e) => return validation_error_response(e),
    };
    
    // Ownerロールの設定
    let owner_data = OwnerData {
        public_key: init_params.public_key,
        initialized_at: SystemTime::now(),
        managed_secrets: Vec::new(),
        access_policies: HashMap::new(),
    };
    
    // ProcessEntityの更新
    let updated_process = ProcessEntity {
        process_id: ao.id.clone(),
        active_roles: vec![ProcessRole::Owner],
        owner_data: Some(owner_data),
        holder_data: None,
        requester_data: None,
        created_at: SystemTime::now(),
        updated_at: SystemTime::now(),
    };
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}
```

## 4. Phase 1: 秘密分割ハンドラー

### 4.1 Split-Secret

```rust
/// 秘密分割ハンドラー
pub async fn handle_split_secret(msg: Message) -> Response {
    // コンテキスト初期化とOwnerロール検証
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 前提条件チェック
    if let Err(e) = check_split_secret_preconditions(&ctx, &msg).await {
        return precondition_failed_response(e);
    }
    
    // パラメータ抽出
    let secret_id = match msg.tags.get("Secret-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Secret-Id"),
    };
    
    // 重複チェック
    if ctx.process.owner_data.as_ref().unwrap()
        .managed_secrets.contains(&secret_id) {
        return error_response("Secret already exists");
    }
    
    // セキュリティ検証
    if let Err(e) = validate_split_parameters(&msg).await {
        return validation_error_response(e);
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の後処理
    if is_success(&response) {
        // 監査ログ
        audit_log(AuditEvent::SecretCreated {
            secret_id: secret_id.clone(),
            owner: ctx.process.process_id.clone(),
            timestamp: SystemTime::now(),
        }).await;
    }
    
    response
}

/// 秘密分割の前提条件チェック
async fn check_split_secret_preconditions(
    ctx: &HandlerContext,
    msg: &Message,
) -> Result<(), PreconditionError> {
    // 1. プロセスが初期化済みか
    if !ctx.process.is_initialized() {
        return Err(PreconditionError::NotInitialized);
    }
    
    // 2. 秘密数の上限チェック
    let current_secrets = ctx.process.owner_data.as_ref()
        .map(|d| d.managed_secrets.len())
        .unwrap_or(0);
        
    if current_secrets >= MAX_SECRETS_PER_OWNER {
        return Err(PreconditionError::LimitExceeded {
            resource: "secrets",
            limit: MAX_SECRETS_PER_OWNER,
            current: current_secrets,
        });
    }
    
    // 3. ストレージ容量チェック
    let secret_size = msg.data.len();
    if secret_size > MAX_SECRET_SIZE {
        return Err(PreconditionError::SizeLimitExceeded {
            size: secret_size,
            max_size: MAX_SECRET_SIZE,
        });
    }
    
    Ok(())
}

/// 分割パラメータの検証
async fn validate_split_parameters(msg: &Message) -> Result<(), ValidationError> {
    let k = extract_u8_tag(msg, "Threshold-K")?;
    let n = extract_u8_tag(msg, "Threshold-N")?;
    
    // 閾値の妥当性
    if k > n {
        return Err(ValidationError::InvalidThreshold { k, n });
    }
    
    if k < MIN_THRESHOLD {
        return Err(ValidationError::ThresholdTooLow { 
            threshold: k, 
            minimum: MIN_THRESHOLD 
        });
    }
    
    if n > MAX_SHARES {
        return Err(ValidationError::TooManyShares { 
            shares: n, 
            maximum: MAX_SHARES 
        });
    }
    
    Ok(())
}
```

### 4.2 Store-Share / Store-Capsule

```rust
/// シェア保存ハンドラー
pub async fn handle_store_share(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // シェアデータの検証
    let share_data = match extract_and_validate_share(&msg) {
        Ok(data) => data,
        Err(e) => return validation_error_response(e),
    };
    
    // 対応する秘密の存在確認
    if !is_managing_secret(&ctx, &share_data.secret_id) {
        return error_response("Not authorized for this secret");
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}

/// カプセル保存ハンドラー
pub async fn handle_store_capsule(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // カプセルデータの検証
    let capsule_data = match extract_and_validate_capsule(&msg) {
        Ok(data) => data,
        Err(e) => return validation_error_response(e),
    };
    
    // 暗号学的検証
    if let Err(e) = verify_capsule_integrity(&capsule_data) {
        return error_response(format!("Invalid capsule: {}", e));
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}
```

## 5. Phase 3: 再暗号化キー管理ハンドラー

### 5.1 Generate-ReKey

```rust
/// 再暗号化キー生成ハンドラー
pub async fn handle_generate_rekey(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // アクセス要求の検証
    let access_request_id = match msg.tags.get("Access-Request-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Access-Request-Id"),
    };
    
    // アクセス要求の詳細を取得
    let access_request = match load_access_request(&ctx, access_request_id).await {
        Ok(req) => req,
        Err(e) => return error_response(format!("Invalid access request: {}", e)),
    };
    
    // アクセス権限の検証
    if let Err(e) = verify_access_permission(&ctx, &access_request).await {
        return error_response(format!("Access denied: {}", e));
    }
    
    // 再暗号化キー生成の前提条件
    if let Err(e) = check_rekey_preconditions(&ctx, &access_request).await {
        return precondition_failed_response(e);
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の監査ログ
    if is_success(&response) {
        audit_log(AuditEvent::ReKeyGenerated {
            access_request_id: access_request_id.clone(),
            secret_id: access_request.secret_id.clone(),
            requester: access_request.requester_id.clone(),
            owner: ctx.process.process_id.clone(),
            timestamp: SystemTime::now(),
        }).await;
    }
    
    response
}

/// アクセス権限の検証
async fn verify_access_permission(
    ctx: &HandlerContext,
    request: &AccessRequest,
) -> Result<(), AccessError> {
    let owner_data = ctx.process.owner_data.as_ref()
        .ok_or(AccessError::NoOwnerData)?;
    
    // 秘密の所有確認
    if !owner_data.managed_secrets.contains(&request.secret_id) {
        return Err(AccessError::NotOwner);
    }
    
    // アクセスポリシーの確認
    let policy = owner_data.access_policies.get(&request.secret_id)
        .ok_or(AccessError::NoPolicyDefined)?;
    
    // ポリシー評価
    policy.evaluate(&request.requester_id, &request.conditions)
        .map_err(|e| AccessError::PolicyViolation(e))
}
```

### 5.2 Distribute-KFrag

```rust
/// kFrag配布ハンドラー
pub async fn handle_distribute_kfrag(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 配布パラメータの抽出
    let distribution_params = match extract_distribution_params(&msg) {
        Ok(params) => params,
        Err(e) => return validation_error_response(e),
    };
    
    // Holder選択戦略の実行
    let selected_holders = match select_holders(&ctx, &distribution_params).await {
        Ok(holders) => holders,
        Err(e) => return error_response(format!("Holder selection failed: {}", e)),
    };
    
    // 選択されたHolderの検証
    if let Err(e) = verify_selected_holders(&selected_holders, &distribution_params) {
        return error_response(format!("Invalid holder selection: {}", e));
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    handler.handle(msg).await
}

/// Holder選択アルゴリズム
async fn select_holders(
    ctx: &HandlerContext,
    params: &DistributionParams,
) -> Result<Vec<HolderInfo>, SelectionError> {
    // 利用可能なHolderを取得
    let available_holders = get_available_holders(ctx).await?;
    
    // 選択基準に基づくスコアリング
    let mut scored_holders: Vec<(HolderInfo, f64)> = available_holders
        .into_iter()
        .map(|holder| {
            let score = calculate_holder_score(&holder, params);
            (holder, score)
        })
        .collect();
    
    // スコア順にソート
    scored_holders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    // 必要数のHolderを選択
    let selected = scored_holders
        .into_iter()
        .take(params.required_holders)
        .map(|(holder, _)| holder)
        .collect::<Vec<_>>();
    
    if selected.len() < params.required_holders {
        return Err(SelectionError::InsufficientHolders {
            required: params.required_holders,
            available: selected.len(),
        });
    }
    
    Ok(selected)
}

/// Holderスコア計算
fn calculate_holder_score(holder: &HolderInfo, params: &DistributionParams) -> f64 {
    let mut score = 0.0;
    
    // 信頼性スコア（最重要）
    score += holder.trust_score * params.trust_weight;
    
    // 可用性スコア
    score += holder.availability * params.availability_weight;
    
    // 地理的分散（オプション）
    if let Some(geo_preference) = &params.geo_preference {
        if let Some(holder_geo) = &holder.geo_info {
            let geo_score = calculate_geo_score(holder_geo, geo_preference);
            score += geo_score * params.geo_weight;
        }
    }
    
    // 負荷分散
    let load_score = 1.0 - (holder.current_load / holder.max_capacity);
    score += load_score * params.load_balance_weight;
    
    score
}
```

## 6. 管理系ハンドラー

### 6.1 Update-Access-Control

```rust
/// アクセス制御更新ハンドラー
pub async fn handle_update_access_control(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 更新対象の秘密ID
    let secret_id = match msg.tags.get("Secret-Id") {
        Some(id) => id,
        None => return validation_error_response("Missing Secret-Id"),
    };
    
    // 所有権確認
    if !is_managing_secret(&ctx, secret_id) {
        return error_response("Not authorized for this secret");
    }
    
    // 新しいアクセスポリシー
    let new_policy = match extract_access_policy(&msg) {
        Ok(policy) => policy,
        Err(e) => return validation_error_response(e),
    };
    
    // ポリシーの妥当性検証
    if let Err(e) = validate_access_policy(&new_policy) {
        return validation_error_response(e);
    }
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の通知
    if is_success(&response) {
        // 影響を受けるRequesterに通知
        notify_policy_change(secret_id, &new_policy).await;
    }
    
    response
}
```

### 6.2 Revoke-Access

```rust
/// アクセス取り消しハンドラー
pub async fn handle_revoke_access(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 取り消し対象
    let revocation_target = match extract_revocation_target(&msg) {
        Ok(target) => target,
        Err(e) => return validation_error_response(e),
    };
    
    // 権限確認
    if !can_revoke_access(&ctx, &revocation_target) {
        return error_response("Not authorized to revoke access");
    }
    
    // 取り消しの影響分析
    let impact = analyze_revocation_impact(&ctx, &revocation_target).await;
    
    // Controller層への委譲
    let handler = MessageHandler::new(ctx.service_container);
    let response = handler.handle(msg).await;
    
    // 成功時の処理
    if is_success(&response) {
        // 影響を受けるプロセスに通知
        for affected_process in impact.affected_processes {
            send_revocation_notice(&affected_process, &revocation_target).await;
        }
        
        // 監査ログ
        audit_log(AuditEvent::AccessRevoked {
            target: revocation_target,
            owner: ctx.process.process_id.clone(),
            timestamp: SystemTime::now(),
            impact,
        }).await;
    }
    
    response
}
```

### 6.3 Get-Secret-Status

```rust
/// 秘密状態取得ハンドラー
pub async fn handle_get_secret_status(msg: Message) -> Response {
    let ctx = match initialize_and_verify_owner().await {
        Ok(ctx) => ctx,
        Err(e) => return unauthorized_response(e),
    };
    
    // 対象秘密ID
    let secret_id = match msg.tags.get("Secret-Id") {
        Some(id) => id,
        None => {
            // 秘密IDが指定されない場合は全秘密の概要を返す
            return get_all_secrets_summary(&ctx).await;
        }
    };
    
    // 所有権確認
    if !is_managing_secret(&ctx, secret_id) {
        return error_response("Not authorized for this secret");
    }
    
    // 秘密の詳細状態を収集
    let status = match collect_secret_status(&ctx, secret_id).await {
        Ok(status) => status,
        Err(e) => return error_response(format!("Failed to get status: {}", e)),
    };
    
    // レスポンス生成
    Response {
        tags: vec![
            ("Status", "Success"),
            ("Action", "Get-Secret-Status-Response"),
            ("Secret-Id", secret_id),
        ],
        data: serde_json::to_vec(&status).unwrap_or_default(),
    }
}

/// 秘密の詳細状態収集
async fn collect_secret_status(
    ctx: &HandlerContext,
    secret_id: &str,
) -> Result<SecretStatus, StatusError> {
    // 基本情報
    let secret_entity = ctx.repositories.secret_repo
        .find_by_id(secret_id)
        .await?
        .ok_or(StatusError::SecretNotFound)?;
    
    // シェア情報
    let shares = ctx.repositories.share_repo
        .find_by_secret_id(secret_id)
        .await?;
    
    // アクセス履歴
    let access_history = ctx.repositories.access_log_repo
        .find_by_secret_id(secret_id)
        .await?;
    
    // 現在のアクセス要求
    let active_requests = ctx.repositories.access_request_repo
        .find_active_by_secret_id(secret_id)
        .await?;
    
    Ok(SecretStatus {
        secret_id: secret_id.to_string(),
        created_at: secret_entity.created_at,
        share_count: shares.len(),
        threshold: secret_entity.threshold,
        access_policy: secret_entity.access_policy.clone(),
        total_access_count: access_history.len(),
        active_access_requests: active_requests.len(),
        last_accessed: access_history.last().map(|h| h.timestamp),
        health_status: calculate_secret_health(&shares, &secret_entity),
    })
}
```

## 7. セキュリティ実装

### 7.1 秘密鍵操作の保護

```rust
/// 秘密鍵操作の保護レイヤー
pub struct SecureKeyOperation {
    operation_id: String,
    started_at: SystemTime,
    completed: bool,
}

impl SecureKeyOperation {
    /// 保護された環境での秘密鍵操作実行
    pub async fn execute<F, R>(operation: F) -> Result<R, KeyOperationError>
    where
        F: FnOnce() -> Result<R, KeyOperationError>,
    {
        // メモリロック
        lock_memory()?;
        
        // 操作実行
        let result = operation();
        
        // メモリクリア
        clear_sensitive_memory();
        
        // メモリアンロック
        unlock_memory()?;
        
        result
    }
}
```

### 7.2 監査ログ

```rust
/// 監査イベント
#[derive(Debug, Serialize)]
pub enum AuditEvent {
    SecretCreated {
        secret_id: String,
        owner: ProcessId,
        timestamp: SystemTime,
    },
    ReKeyGenerated {
        access_request_id: String,
        secret_id: String,
        requester: ProcessId,
        owner: ProcessId,
        timestamp: SystemTime,
    },
    AccessRevoked {
        target: RevocationTarget,
        owner: ProcessId,
        timestamp: SystemTime,
        impact: RevocationImpact,
    },
    PolicyUpdated {
        secret_id: String,
        old_policy: AccessPolicy,
        new_policy: AccessPolicy,
        owner: ProcessId,
        timestamp: SystemTime,
    },
}

/// 監査ログ記録
async fn audit_log(event: AuditEvent) -> Result<(), AuditError> {
    let log_entry = AuditLogEntry {
        event_id: generate_event_id(),
        event,
        process_id: ao.id.clone(),
        signature: sign_audit_event(&event)?,
    };
    
    // Arweaveに永続化
    let tx_id = store_audit_log(&log_entry).await?;
    
    // ローカルインデックスの更新
    update_audit_index(&log_entry, &tx_id).await?;
    
    Ok(())
}
```

## 8. エラー処理

### 8.1 Owner固有のエラー

```rust
#[derive(Debug, thiserror::Error)]
pub enum OwnerError {
    #[error("Not authorized as owner")]
    NotOwner,
    
    #[error("Secret limit exceeded: {current}/{max}")]
    SecretLimitExceeded { current: usize, max: usize },
    
    #[error("Invalid threshold configuration: k={k}, n={n}")]
    InvalidThreshold { k: u8, n: u8 },
    
    #[error("Holder selection failed: {reason}")]
    HolderSelectionFailed { reason: String },
    
    #[error("Access policy violation: {details}")]
    PolicyViolation { details: String },
    
    #[error("Key operation failed: {operation}")]
    KeyOperationFailed { operation: String },
}
```

### 8.2 リカバリー戦略

```rust
/// エラーリカバリー
pub async fn handle_with_recovery(msg: Message) -> Response {
    match handle_internal(msg.clone()).await {
        Ok(response) => response,
        Err(OwnerError::HolderSelectionFailed { .. }) => {
            // 代替Holder選択を試行
            if let Ok(response) = retry_with_relaxed_criteria(msg).await {
                return response;
            }
            error_response("Holder selection failed after retry")
        }
        Err(e) => error_response(e),
    }
}
```

## 9. パフォーマンス最適化

### 9.1 バッチ処理

```rust
/// kFragの一括配布
pub async fn batch_distribute_kfrags(
    kfrags: Vec<KFragData>,
    holders: Vec<HolderInfo>,
) -> Vec<Result<DistributionResult, DistributionError>> {
    let tasks = kfrags.into_iter()
        .zip(holders.into_iter())
        .map(|(kfrag, holder)| {
            distribute_single_kfrag(kfrag, holder)
        });
    
    futures::future::join_all(tasks).await
}
```

### 9.2 キャッシング

```rust
/// Ownerデータのキャッシング
pub struct OwnerCache {
    managed_secrets: Arc<RwLock<HashSet<String>>>,
    access_policies: Arc<RwLock<HashMap<String, AccessPolicy>>>,
    last_update: Arc<RwLock<SystemTime>>,
}

impl OwnerCache {
    pub async fn get_or_load_policy(
        &self,
        secret_id: &str,
    ) -> Result<AccessPolicy, CacheError> {
        // キャッシュチェック
        if let Some(policy) = self.access_policies.read().await.get(secret_id) {
            return Ok(policy.clone());
        }
        
        // キャッシュミス時はロード
        let policy = load_access_policy(secret_id).await?;
        self.access_policies.write().await.insert(
            secret_id.to_string(),
            policy.clone()
        );
        
        Ok(policy)
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
    async fn test_split_secret_validation() {
        let msg = create_test_message(vec![
            ("Action", "Split-Secret"),
            ("Secret-Id", "test-123"),
            ("Threshold-K", "3"),
            ("Threshold-N", "5"),
        ]);
        
        let result = validate_split_parameters(&msg).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_holder_selection() {
        let ctx = create_test_context();
        let params = DistributionParams {
            required_holders: 5,
            trust_weight: 0.5,
            availability_weight: 0.3,
            geo_weight: 0.2,
            ..Default::default()
        };
        
        let holders = select_holders(&ctx, &params).await.unwrap();
        assert_eq!(holders.len(), 5);
        
        // スコア順になっているか確認
        for i in 0..holders.len()-1 {
            let score1 = calculate_holder_score(&holders[i], &params);
            let score2 = calculate_holder_score(&holders[i+1], &params);
            assert!(score1 >= score2);
        }
    }
}
```

### 10.2 統合テスト

```rust
#[tokio::test]
async fn test_complete_secret_lifecycle() {
    // Owner プロセスの初期化
    let owner_process = spawn_owner_process().await;
    
    // Phase 1: 秘密分割
    let split_response = send_split_secret_message(&owner_process).await;
    assert!(is_success(&split_response));
    
    let secret_id = get_secret_id(&split_response);
    
    // Phase 3: 再暗号化キー生成
    let rekey_response = send_generate_rekey_message(
        &owner_process,
        &secret_id
    ).await;
    assert!(is_success(&rekey_response));
    
    // 状態確認
    let status_response = send_get_status_message(
        &owner_process,
        &secret_id
    ).await;
    
    let status = parse_secret_status(&status_response);
    assert_eq!(status.health_status, HealthStatus::Healthy);
}
```

## 11. まとめ

Owner Roleハンドラーは、FORMIXシステムの中核として以下の特性を持ちます：

1. **最高レベルのセキュリティ**: 秘密鍵操作の厳重な保護
2. **包括的な秘密管理**: ライフサイクル全体の制御
3. **柔軟なアクセス制御**: きめ細かいポリシー設定
4. **高度な監査機能**: すべての操作の追跡可能性
5. **インテリジェントな配布**: 最適なHolder選択アルゴリズム

これらの設計により、安全で信頼性の高い秘密管理を実現します。

---

**Document Status**: Owner Role Handlers Specification  
**Version**: 1.0  
**Last Updated**: 2025-01-07