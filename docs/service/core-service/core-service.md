# FORMIX Core Service層設計

## 1. 概要

Core Service層は、FORMIXの基本機能を提供するサービス群です。これらのサービスは、ビジネスロジックの実装と計算処理に特化し、データ永続化はRepository層に委譲します。Workflow Service層から利用され、複雑な処理フローの基盤となります。

## 2. Core Serviceの設計原則

### 2.1 責務の明確化

- **単一責任の原則**: 各Core Serviceは明確に定義された単一の責務を持つ
- **ビジネスロジック特化**: 計算処理やビジネスルールの実装に集中
- **Repository層との分離**: データ永続化はRepository層に委譲し、ビジネスロジックのみを実装
- **再利用性**: 複数のWorkflow Serviceから利用可能な汎用的な設計

### 2.2 実装指針

- **ステートレス**: AO環境に適応したステートレス設計
- **非同期処理**: I/O操作は非同期で実装
- **エラー処理**: 明確なエラー型定義と適切な例外処理

## 3. CryptoService

### 3.1 概要

CryptoServiceは、FORMIXの暗号化操作すべてを担当する中核サービスです。Threshold Proxy Re-Encryption (TPRE) とShamir Secret Sharingの実装を提供します。

### 3.2 インターフェース定義

```rust
use async_trait::async_trait;
use crate::domain::entity::{ShamirShare, Capsule, PublicKey, SecretKey, ReencryptionKey, KeyFragment, CipherFragment};
use crate::domain::error::CryptoError;

#[async_trait]
pub trait CryptoService: Send + Sync {
    // Shamir Secret Sharing操作
    async fn split_secret_shamir(
        &self,
        secret: &[u8],
        threshold: u8,
        total_shares: u8,
    ) -> Result<Vec<ShamirShare>, CryptoError>;
    
    async fn reconstruct_secret_shamir(
        &self,
        shares: &[ShamirShare],
        threshold: u8,
    ) -> Result<Vec<u8>, CryptoError>;
    
    // PRE暗号化操作
    async fn create_pre_capsule(
        &self,
        public_key: &PublicKey,
        plaintext: &[u8],
    ) -> Result<(Capsule, Vec<u8>), CryptoError>;
    
    async fn generate_reencryption_key(
        &self,
        owner_secret_key: &SecretKey,
        accessor_public_key: &PublicKey,
    ) -> Result<ReencryptionKey, CryptoError>;
    
    async fn create_kfrags(
        &self,
        reencryption_key: &ReencryptionKey,
        threshold: u8,
        total_fragments: u8,
    ) -> Result<Vec<KeyFragment>, CryptoError>;
    
    async fn proxy_reencrypt(
        &self,
        kfrag: &KeyFragment,
        capsule: &Capsule,
    ) -> Result<CipherFragment, CryptoError>;
    
    async fn combine_and_decrypt(
        &self,
        cfrags: &[CipherFragment],
        accessor_secret_key: &SecretKey,
        original_capsule: &Capsule,
        ciphertext: &[u8],
    ) -> Result<Vec<u8>, CryptoError>;
}
```

### 3.3 実装ガイドライン

#### 3.3.1 暗号ライブラリの使用

```rust
pub struct CryptoServiceImpl {
    umbral_engine: UmbralEngine,
    shamir_engine: ShamirEngine,
}

impl CryptoServiceImpl {
    pub fn new() -> Self {
        Self {
            umbral_engine: UmbralEngine::new(),
            shamir_engine: ShamirEngine::new(),
        }
    }
}
```

#### 3.3.2 エラーハンドリング

```rust
impl From<umbral_pre::Error> for CryptoError {
    fn from(err: umbral_pre::Error) -> Self {
        CryptoError::UmbralError(err.to_string())
    }
}

impl From<shamir::Error> for CryptoError {
    fn from(err: shamir::Error) -> Self {
        CryptoError::ShamirError(err.to_string())
    }
}
```

### 3.4 使用例

```rust
// Workflow Serviceからの使用例
async fn execute_secret_sharing(&self, secret_data: &[u8]) -> Result<(), WorkflowError> {
    // Shamir分割
    let shares = self.crypto_service
        .split_secret_shamir(secret_data, 3, 5)
        .await?;
    
    // 各シェアを暗号化
    for share in shares {
        let (capsule, ciphertext) = self.crypto_service
            .create_pre_capsule(&holder_public_key, &share.data)
            .await?;
        
        // ストレージに保存...
    }
    
    Ok(())
}
```

## 4. MessageRoutingService

### 4.1 概要

MessageRoutingServiceは、AOプロセス間の非同期メッセージングとルーティングを管理します。プロセス発見、メッセージ配信、応答収集機能を提供します。

### 4.2 インターフェース定義

```rust
use async_trait::async_trait;
use crate::domain::entity::{ProcessMessage, MessageId, MessageResponse, ProcessInfo};
use crate::domain::error::RoutingError;
use std::time::Duration;

#[async_trait]
pub trait MessageRoutingService: Send + Sync {
    // メッセージ送信
    async fn send_message(
        &self,
        target_process_id: &str,
        message: ProcessMessage,
    ) -> Result<MessageId, RoutingError>;
    
    async fn broadcast_message(
        &self,
        target_processes: &[String],
        message: ProcessMessage,
    ) -> Result<Vec<MessageId>, RoutingError>;
    
    // 応答管理
    async fn wait_for_responses(
        &self,
        message_ids: &[MessageId],
        timeout: Duration,
    ) -> Result<Vec<MessageResponse>, RoutingError>;
    
    async fn wait_for_threshold_responses(
        &self,
        message_ids: &[MessageId],
        threshold: usize,
        timeout: Duration,
    ) -> Result<Vec<MessageResponse>, RoutingError>;
    
    // プロセス発見
    async fn discover_online_processes(
        &self,
        role_filter: Option<ProcessRole>,
        capacity_requirement: Option<u64>,
    ) -> Result<Vec<ProcessInfo>, RoutingError>;
    
    // 特化メソッド
    async fn collect_cfrags(
        &self,
        holder_processes: &[String],
        reencryption_request: ReencryptionRequest,
        threshold: u8,
    ) -> Result<Vec<CipherFragment>, RoutingError>;
}
```

### 4.3 実装ガイドライン

#### 4.3.1 非同期メッセージング

```rust
pub struct MessageRoutingServiceImpl {
    ao_client: Arc<AOClient>,
    pending_responses: Arc<RwLock<HashMap<MessageId, oneshot::Sender<MessageResponse>>>>,
}

impl MessageRoutingServiceImpl {
    async fn send_message_impl(
        &self,
        target: &str,
        message: ProcessMessage,
    ) -> Result<MessageId, RoutingError> {
        let message_id = MessageId::new();
        
        // AOメッセージ送信
        self.ao_client
            .send(AOMessage {
                target: target.to_string(),
                tags: message.tags.clone(),
                data: message.payload.clone(),
            })
            .await?;
        
        Ok(message_id)
    }
}
```

#### 4.3.2 応答収集パターン

```rust
impl MessageRoutingServiceImpl {
    async fn collect_responses_with_threshold(
        &self,
        message_ids: &[MessageId],
        threshold: usize,
        timeout: Duration,
    ) -> Result<Vec<MessageResponse>, RoutingError> {
        let mut responses = Vec::new();
        let deadline = Instant::now() + timeout;
        
        let mut response_stream = futures::stream::FuturesUnordered::new();
        
        for id in message_ids {
            let rx = self.create_response_channel(id).await?;
            response_stream.push(rx);
        }
        
        while let Some(result) = response_stream.next().await {
            match result {
                Ok(response) => {
                    responses.push(response);
                    if responses.len() >= threshold {
                        return Ok(responses);
                    }
                }
                Err(_) if Instant::now() > deadline => {
                    return Err(RoutingError::Timeout);
                }
                Err(e) => continue, // 個別の失敗は無視
            }
        }
        
        if responses.len() >= threshold {
            Ok(responses)
        } else {
            Err(RoutingError::InsufficientResponses)
        }
    }
}
```

### 4.4 使用例

```rust
// Holder群へのkFrag配布
let holder_processes = routing_service
    .discover_online_processes(
        Some(ProcessRole::Holder { capacity: 0 }),
        Some(10), // 最小容量要件
    )
    .await?;

let message_ids = routing_service
    .broadcast_message(
        &holder_processes.iter().map(|p| p.process_id.clone()).collect::<Vec<_>>(),
        ProcessMessage {
            message_type: MessageType::DistributeKFrag,
            payload: kfrag_data,
            tags: tags,
            created_at: now,
        },
    )
    .await?;

// 応答待機
let responses = routing_service
    .wait_for_threshold_responses(&message_ids, threshold as usize, Duration::from_secs(30))
    .await?;
```

## 5. Core Service間の連携

### 5.1 サービス間依存関係

```mermaid
graph LR
    CS1[CryptoService]
    CS3[MessageRoutingService]
    R[Repository Layer]
    D[Domain Layer]

    CS3 --> R
    CS1 --> D
    CS3 --> D
```

Core Service層は、各サービスが独立した責務を持ちながら、必要に応じて連携します：
- **CryptoService**: 暗号化計算に特化、他サービスから独立
- **MessageRoutingService**: プロセス間通信とRepository経由での状態確認

プロセス管理機能は、Domain層のProcessEntityとRepository層で実装されます。

### 5.2 連携パターン

#### 5.2.1 MessageRoutingとRepository層

```rust
impl MessageRoutingServiceImpl {
    process_repository: Arc<dyn ProcessEntityRepository>,

    async fn discover_holders_with_capacity(
        &self,
        min_capacity: u64,
    ) -> Result<Vec<ProcessInfo>, RoutingError> {
        // Repositoryから直接Holder情報を取得
        let holder_processes = self.process_repository
            .find_processes_with_holder_capability()
            .await?;

        let eligible_holders = holder_processes
            .into_iter()
            .filter(|p| {
                if let Some(holder_data) = &p.holder_data {
                    holder_data.current_load < min_capacity
                } else {
                    false
                }
            })
            .map(|p| ProcessInfo::from(p))
            .collect();

        Ok(eligible_holders)
    }
}
```

#### 5.2.2 Domain層でのプロセス管理

プロセス管理のビジネスロジックは、ProcessEntityのメソッドとして実装されます：

```rust
// domain/entities/process.rs
impl ProcessEntity {
    pub fn validate_role_compatibility(&self, new_role: &ProcessRole) -> Result<(), DomainError> {
        // ロール互換性チェックロジック
        match new_role {
            ProcessRole::Owner if self.has_owner_role() => {
                return Err(DomainError::business_rule_violation(
                    "owner_unique",
                    "Owner role already exists"
                ));
            }
            _ => Ok(())
        }
    }

    pub fn calculate_reliability_score(&self) -> f64 {
        let metrics = &self.performance_metrics;
        let total_operations = metrics.successful_operations + metrics.failed_operations;
        let success_factor = if total_operations > 0 {
            metrics.successful_operations as f64 / total_operations as f64
        } else {
            1.0
        };
        let penalty = metrics.failed_operations as f64 * 0.01;
        (success_factor - penalty).max(0.0).min(1.0)
    }
}
```

## 6. テスト戦略

### 6.1 単体テスト

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    
    mock! {
        CryptoService {}
        
        #[async_trait]
        impl CryptoService for CryptoService {
            async fn split_secret_shamir(
                &self,
                secret: &[u8],
                threshold: u8,
                total_shares: u8,
            ) -> Result<Vec<ShamirShare>, CryptoError>;
            
            // 他のメソッド...
        }
    }
    
    #[tokio::test]
    async fn test_shamir_split_and_reconstruct() {
        let service = CryptoServiceImpl::new();
        let secret = b"test secret data";
        
        let shares = service.split_secret_shamir(secret, 3, 5).await.unwrap();
        assert_eq!(shares.len(), 5);
        
        let reconstructed = service
            .reconstruct_secret_shamir(&shares[0..3], 3)
            .await
            .unwrap();
        
        assert_eq!(reconstructed, secret);
    }
}
```

### 6.2 統合テスト

```rust
#[tokio::test]
async fn test_process_management_with_repository() {
    let mock_repo = MockProcessEntityRepository::new();
    let process_service = ProcessManagementServiceImpl::new(mock_repo);
    
    // ロール追加のビジネスロジックテスト
    let result = process_service
        .validate_and_add_role("test-process", ProcessRole::Holder { capacity: 50 })
        .await;
    
    assert!(result.is_ok());
}
```

## 7. パフォーマンス考慮事項

### 7.1 キャッシング戦略

- ProcessEntity: AOステートレス環境では適用不可
- 暗号化操作結果: 結果のキャッシングは避ける（セキュリティ）
- メッセージ配信: バッチ処理による効率化

### 7.2 並列処理

- メッセージブロードキャスト: 並列送信
- バッチストレージ操作: 並列トランザクション作成
- cFrag収集: 並列応答待機

## 8. まとめ

Core Service層は、FORMIXの基盤機能を提供する重要な層です。各サービスは明確な責務を持ち：

1. **CryptoService**: 暗号化計算処理に特化
2. **MessageRoutingService**: AOプロセス間の非同期通信

### 主要な設計原則

1. **AOステートレス環境への適応**
   - 複雑な状態管理サービスを排除
   - プロセス管理はDomain層とRepository層で実装

2. **シンプルな責務分担**
   - 暗号化とメッセージングの基本機能に集約
   - ビジネスロジックはWorkflow層とDomain層で処理

3. **Repository層との直接連携**
   - データ永続化の責任を明確化
   - 関心の分離と保守性の向上

この設計により、AOプラットフォームの制約に適合し、高性能で信頼性の高いFORMIX暗号システムを実現します。