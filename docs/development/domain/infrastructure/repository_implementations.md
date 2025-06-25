---
title: "D-TPRES Repository Implementation詳細設計"
description: "Arweave永続化層の包括的実装設計"
tags: ["repository-implementation", "arweave", "infrastructure", "persistence-layer"]
status: "specification"
created: "2025-06-25"
author: "D-TPRES Development Team"
---

# D-TPRES Repository Implementation詳細設計

## 1. 概要

本ドキュメントは、D-TPRESシステムにおけるRepository実装層の詳細設計を定義します。Arweaveの不変ストレージ特性を活用し、効率的かつ信頼性の高い永続化層を実現します。

## 2. アーキテクチャ概要

### 2.1 実装層の構造

```mermaid
graph TB
    subgraph "Domain Layer"
        DI[Repository Interfaces]
    end
    
    subgraph "Infrastructure Layer"
        subgraph "Repository Implementations"
            RI1[ProcessEntityRepositoryImpl]
            RI2[ShareEntityRepositoryImpl]
            RI3[CapsuleEntityRepositoryImpl]
            RI4[AccessRequestEntityRepositoryImpl]
            RI5[RekeyFragmentEntityRepositoryImpl]
            RI6[ReencryptionEntityRepositoryImpl]
        end
        
        subgraph "Base Implementation"
            ARI[ArweaveRepositoryImpl<T, ID>]
        end
        
        subgraph "Arweave Adapter"
            AC[ArweaveClient]
            IM[IndexManager]
            QO[QueryOptimizer]
            CM[CacheManager]
        end
        
        subgraph "Utilities"
            SER[Serializer]
            TAG[TagBuilder]
            ERR[ErrorHandler]
        end
    end
    
    subgraph "External"
        AR[Arweave Network]
    end
    
    DI --> RI1
    DI --> RI2
    DI --> RI3
    DI --> RI4
    DI --> RI5
    DI --> RI6
    
    RI1 --> ARI
    RI2 --> ARI
    RI3 --> ARI
    RI4 --> ARI
    RI5 --> ARI
    RI6 --> ARI
    
    ARI --> AC
    ARI --> IM
    ARI --> QO
    ARI --> CM
    
    AC --> AR
    
    ARI --> SER
    ARI --> TAG
    ARI --> ERR
```

### 2.2 設計原則

1. **不変性への対応**: Arweaveの特性を考慮した更新戦略
2. **効率的なクエリ**: タグベースインデックスの最適化
3. **キャッシュ戦略**: 読み込み性能の向上
4. **エラー処理**: ネットワーク障害への対応
5. **型安全性**: ジェネリクスを活用した実装

## 3. ArweaveClient - ストレージアダプター

### 3.1 概要
Arweaveネットワークとの通信を抽象化するクライアント実装。

### 3.2 詳細実装

```rust
use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

/// Arweaveクライアントのエラー型
#[derive(Debug, Error)]
pub enum ArweaveError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Transaction not found: {tx_id}")]
    TransactionNotFound { tx_id: String },
    
    #[error("Invalid data format")]
    InvalidData,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Transaction rejected: {reason}")]
    TransactionRejected { reason: String },
    
    #[error("Timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
}

/// Arweaveトランザクションの状態
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed { block_height: u64 },
    Failed { reason: String },
}

/// Arweaveクライアントのトレイト定義
#[async_trait]
pub trait ArweaveClient: Send + Sync {
    /// データをArweaveに保存
    /// 
    /// # 引数
    /// - `data`: 保存するデータ
    /// - `tags`: トランザクションタグ
    /// 
    /// # 戻り値
    /// トランザクションID
    async fn store_data(
        &self,
        data: Vec<u8>,
        tags: HashMap<String, String>,
    ) -> Result<String, ArweaveError>;
    
    /// トランザクションIDからデータを取得
    async fn get_data(&self, tx_id: &str) -> Result<Vec<u8>, ArweaveError>;
    
    /// タグによるクエリ実行
    async fn query_by_tags(
        &self,
        tags: HashMap<String, String>,
    ) -> Result<Vec<String>, ArweaveError>;
    
    /// トランザクションの状態確認
    async fn get_transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<TransactionStatus, ArweaveError>;
    
    /// バッチ取得（複数トランザクション）
    async fn get_batch_data(
        &self,
        tx_ids: &[String],
    ) -> Result<Vec<(String, Vec<u8>)>, ArweaveError>;
}

/// Arweaveクライアントの実装
pub struct ArweaveClientImpl {
    gateway_url: String,
    wallet_key: Vec<u8>,
    timeout_seconds: u64,
    retry_config: RetryConfig,
}

/// リトライ設定
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_base: f64,
}

impl ArweaveClientImpl {
    pub fn new(
        gateway_url: String,
        wallet_key: Vec<u8>,
        timeout_seconds: u64,
    ) -> Self {
        Self {
            gateway_url,
            wallet_key,
            timeout_seconds,
            retry_config: RetryConfig {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                exponential_base: 2.0,
            },
        }
    }
    
    /// タグの検証とサニタイズ
    fn validate_tags(&self, tags: &HashMap<String, String>) -> Result<(), ArweaveError> {
        for (key, value) in tags {
            if key.len() > 1024 || value.len() > 3072 {
                return Err(ArweaveError::InvalidData);
            }
        }
        Ok(())
    }
    
    /// リトライ付きHTTPリクエスト実行
    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T, ArweaveError>
    where
        F: Fn() -> futures::future::BoxFuture<'static, Result<T, ArweaveError>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.retry_config.initial_delay_ms;
        
        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.retry_config.max_attempts => {
                    attempt += 1;
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms as f64 * self.retry_config.exponential_base) as u64;
                    delay_ms = delay_ms.min(self.retry_config.max_delay_ms);
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[async_trait]
impl ArweaveClient for ArweaveClientImpl {
    async fn store_data(
        &self,
        data: Vec<u8>,
        tags: HashMap<String, String>,
    ) -> Result<String, ArweaveError> {
        // タグ検証
        self.validate_tags(&tags)?;
        
        // トランザクション作成
        let tx = self.create_transaction(data, tags).await?;
        
        // 署名
        let signed_tx = self.sign_transaction(tx).await?;
        
        // 送信（リトライ付き）
        let tx_id = self.execute_with_retry(|| {
            Box::pin(self.submit_transaction(signed_tx.clone()))
        }).await?;
        
        Ok(tx_id)
    }
    
    async fn get_data(&self, tx_id: &str) -> Result<Vec<u8>, ArweaveError> {
        let url = format!("{}/tx/{}/data", self.gateway_url, tx_id);
        
        self.execute_with_retry(|| {
            Box::pin(async move {
                let response = reqwest::get(&url)
                    .await
                    .map_err(|e| ArweaveError::Network(e.to_string()))?;
                
                if response.status() == 404 {
                    return Err(ArweaveError::TransactionNotFound {
                        tx_id: tx_id.to_string(),
                    });
                }
                
                response.bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|e| ArweaveError::Network(e.to_string()))
            })
        }).await
    }
    
    async fn query_by_tags(
        &self,
        tags: HashMap<String, String>,
    ) -> Result<Vec<String>, ArweaveError> {
        // GraphQLクエリ構築
        let query = self.build_graphql_query(&tags);
        
        // クエリ実行
        let results = self.execute_graphql_query(&query).await?;
        
        Ok(results)
    }
    
    async fn get_transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<TransactionStatus, ArweaveError> {
        let url = format!("{}/tx/{}/status", self.gateway_url, tx_id);
        
        let status = self.execute_with_retry(|| {
            Box::pin(async move {
                let response = reqwest::get(&url)
                    .await
                    .map_err(|e| ArweaveError::Network(e.to_string()))?;
                
                // ステータスコードに基づく判定
                match response.status().as_u16() {
                    200 => {
                        let block_info: BlockInfo = response.json().await
                            .map_err(|_| ArweaveError::InvalidData)?;
                        Ok(TransactionStatus::Confirmed {
                            block_height: block_info.block_height,
                        })
                    }
                    202 => Ok(TransactionStatus::Pending),
                    404 => Err(ArweaveError::TransactionNotFound {
                        tx_id: tx_id.to_string(),
                    }),
                    _ => Ok(TransactionStatus::Failed {
                        reason: "Unknown status".to_string(),
                    }),
                }
            })
        }).await?;
        
        Ok(status)
    }
    
    async fn get_batch_data(
        &self,
        tx_ids: &[String],
    ) -> Result<Vec<(String, Vec<u8>)>, ArweaveError> {
        use futures::future::join_all;
        
        let futures = tx_ids.iter().map(|tx_id| {
            let tx_id = tx_id.clone();
            async move {
                match self.get_data(&tx_id).await {
                    Ok(data) => Ok((tx_id, data)),
                    Err(e) => Err(e),
                }
            }
        });
        
        let results = join_all(futures).await;
        
        // エラーがあれば最初のエラーを返す
        for result in &results {
            if let Err(e) = result {
                return Err(e.clone());
            }
        }
        
        Ok(results.into_iter().filter_map(Result::ok).collect())
    }
}

// Private実装メソッド
impl ArweaveClientImpl {
    async fn create_transaction(
        &self,
        data: Vec<u8>,
        tags: HashMap<String, String>,
    ) -> Result<Transaction, ArweaveError> {
        // 実装省略
        todo!()
    }
    
    async fn sign_transaction(
        &self,
        tx: Transaction,
    ) -> Result<SignedTransaction, ArweaveError> {
        // 実装省略
        todo!()
    }
    
    async fn submit_transaction(
        &self,
        tx: SignedTransaction,
    ) -> Result<String, ArweaveError> {
        // 実装省略
        todo!()
    }
    
    fn build_graphql_query(&self, tags: &HashMap<String, String>) -> String {
        // GraphQLクエリ構築
        let mut tag_filters = Vec::new();
        for (key, value) in tags {
            tag_filters.push(format!(
                r#"{{ name: "{}", values: ["{}"] }}"#,
                key, value
            ));
        }
        
        format!(
            r#"
            query {{
                transactions(
                    tags: [{}]
                    first: 100
                    sort: HEIGHT_DESC
                ) {{
                    edges {{
                        node {{
                            id
                            tags {{
                                name
                                value
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            tag_filters.join(", ")
        )
    }
    
    async fn execute_graphql_query(&self, query: &str) -> Result<Vec<String>, ArweaveError> {
        // 実装省略
        todo!()
    }
}
```

## 4. 基底Repository実装 - ArweaveRepositoryImpl

### 4.1 概要
全てのEntity別Repository実装の基底となる汎用実装。

### 4.2 詳細実装

```rust
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::marker::PhantomData;
use std::sync::Arc;

/// Arweave Repository基底実装
/// 
/// # 型パラメータ
/// - `T`: Entity型（Serialize + Deserialize必須）
/// - `ID`: 識別子型
pub struct ArweaveRepositoryImpl<T, ID> {
    /// Arweaveクライアント
    arweave_client: Arc<dyn ArweaveClient>,
    
    /// エンティティタイプ名
    entity_type: &'static str,
    
    /// インデックスマネージャー
    index_manager: Arc<IndexManager>,
    
    /// キャッシュマネージャー
    cache_manager: Arc<CacheManager>,
    
    /// クエリ最適化
    query_optimizer: Arc<QueryOptimizer>,
    
    /// ファントムデータ
    _phantom: PhantomData<(T, ID)>,
}

impl<T, ID> ArweaveRepositoryImpl<T, ID>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    ID: ToString + Clone + Send + Sync + 'static,
{
    /// コンストラクタ
    pub fn new(
        arweave_client: Arc<dyn ArweaveClient>,
        entity_type: &'static str,
        index_manager: Arc<IndexManager>,
        cache_manager: Arc<CacheManager>,
        query_optimizer: Arc<QueryOptimizer>,
    ) -> Self {
        Self {
            arweave_client,
            entity_type,
            index_manager,
            cache_manager,
            query_optimizer,
            _phantom: PhantomData,
        }
    }
    
    /// エンティティの永続化
    async fn persist_entity(
        &self,
        entity: &T,
        id: &ID,
        operation: PersistOperation,
    ) -> Result<String, RepositoryError> {
        // 1. シリアライゼーション
        let json_data = serde_json::to_vec(entity)
            .map_err(RepositoryError::Serialization)?;
        
        // 2. メタデータ付加
        let metadata = EntityMetadata {
            version: 1,
            created_at: current_timestamp(),
            operation: operation.clone(),
            content_type: "application/json".to_string(),
        };
        
        let wrapped_data = WrappedEntity {
            metadata,
            entity: json_data,
        };
        
        let final_data = serde_json::to_vec(&wrapped_data)
            .map_err(RepositoryError::Serialization)?;
        
        // 3. タグ作成
        let tags = self.create_storage_tags(id, &operation);
        
        // 4. Arweaveに保存
        let tx_id = self.arweave_client
            .store_data(final_data, tags)
            .await
            .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
        
        // 5. インデックス更新
        self.index_manager
            .update_index(self.entity_type, &id.to_string(), &tx_id)
            .await?;
        
        // 6. キャッシュ更新
        if operation != PersistOperation::Delete {
            self.cache_manager
                .set(&id.to_string(), entity.clone())
                .await;
        } else {
            self.cache_manager
                .invalidate(&id.to_string())
                .await;
        }
        
        Ok(tx_id)
    }
    
    /// エンティティの取得
    async fn retrieve_entity(&self, id: &ID) -> Result<Option<T>, RepositoryError> {
        let id_str = id.to_string();
        
        // 1. キャッシュチェック
        if let Some(cached) = self.cache_manager.get::<T>(&id_str).await {
            return Ok(Some(cached));
        }
        
        // 2. インデックスから最新トランザクションID取得
        let tx_id = match self.index_manager.get_latest_tx(&id_str).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };
        
        // 3. Arweaveからデータ取得
        let data = self.arweave_client
            .get_data(&tx_id)
            .await
            .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
        
        // 4. デシリアライゼーション
        let wrapped: WrappedEntity = serde_json::from_slice(&data)
            .map_err(RepositoryError::Serialization)?;
        
        // 5. 削除マーカーチェック
        if wrapped.metadata.operation == PersistOperation::Delete {
            return Ok(None);
        }
        
        let entity: T = serde_json::from_slice(&wrapped.entity)
            .map_err(RepositoryError::Serialization)?;
        
        // 6. キャッシュ更新
        self.cache_manager.set(&id_str, entity.clone()).await;
        
        Ok(Some(entity))
    }
    
    /// ストレージタグの作成
    fn create_storage_tags(
        &self,
        id: &ID,
        operation: &PersistOperation,
    ) -> HashMap<String, String> {
        let mut tags = HashMap::new();
        
        // 必須タグ
        tags.insert("App-Name".to_string(), "D-TPRES".to_string());
        tags.insert("Entity-Type".to_string(), self.entity_type.to_string());
        tags.insert("Entity-Id".to_string(), id.to_string());
        tags.insert("Operation".to_string(), operation.to_string());
        tags.insert("Timestamp".to_string(), current_timestamp().to_string());
        
        // エンティティタイプ別の追加タグ
        self.add_entity_specific_tags(&mut tags, id);
        
        tags
    }
    
    /// エンティティ固有のタグ追加（オーバーライド用）
    fn add_entity_specific_tags(&self, tags: &mut HashMap<String, String>, id: &ID) {
        // デフォルトは何もしない
    }
}

/// 永続化操作の種類
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum PersistOperation {
    Create,
    Update,
    Delete,
}

impl ToString for PersistOperation {
    fn to_string(&self) -> String {
        match self {
            PersistOperation::Create => "CREATE",
            PersistOperation::Update => "UPDATE",
            PersistOperation::Delete => "DELETE",
        }.to_string()
    }
}

/// エンティティメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntityMetadata {
    version: u32,
    created_at: u64,
    operation: PersistOperation,
    content_type: String,
}

/// ラップされたエンティティ
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WrappedEntity {
    metadata: EntityMetadata,
    entity: Vec<u8>,
}

/// 基本Repository trait実装
#[async_trait]
impl<T, ID> Repository<T, ID> for ArweaveRepositoryImpl<T, ID>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + 'static,
    ID: ToString + Clone + Send + Sync + 'static,
{
    type Error = RepositoryError;
    
    async fn create(&self, entity: &T) -> Result<(), Self::Error> {
        let id = self.extract_entity_id(entity)?;
        
        // 既存チェック
        if self.exists(&id).await? {
            return Err(RepositoryError::AlreadyExists {
                id: id.to_string(),
            });
        }
        
        self.persist_entity(entity, &id, PersistOperation::Create).await?;
        Ok(())
    }
    
    async fn find_by_id(&self, id: &ID) -> Result<Option<T>, Self::Error> {
        self.retrieve_entity(id).await
    }
    
    async fn update(&self, entity: &T) -> Result<(), Self::Error> {
        let id = self.extract_entity_id(entity)?;
        
        // 存在チェック
        if !self.exists(&id).await? {
            return Err(RepositoryError::NotFound {
                id: id.to_string(),
            });
        }
        
        // バージョンチェック（楽観ロック）
        if let Some(existing) = self.find_by_id(&id).await? {
            self.check_version_conflict(&existing, entity)?;
        }
        
        self.persist_entity(entity, &id, PersistOperation::Update).await?;
        Ok(())
    }
    
    async fn delete(&self, id: &ID) -> Result<(), Self::Error> {
        // Arweaveは不変なので、削除マーカーを保存
        let deletion_marker = self.create_deletion_marker(id);
        self.persist_entity(&deletion_marker, id, PersistOperation::Delete).await?;
        Ok(())
    }
    
    async fn find_all(&self) -> Result<Vec<T>, Self::Error> {
        // クエリ最適化
        let query_plan = self.query_optimizer
            .optimize_find_all_query(self.entity_type)
            .await?;
        
        let tx_ids = self.execute_optimized_query(query_plan).await?;
        
        // バッチ取得
        let mut entities = Vec::new();
        for chunk in tx_ids.chunks(50) {
            let batch_data = self.arweave_client
                .get_batch_data(chunk)
                .await
                .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
            
            for (_, data) in batch_data {
                let wrapped: WrappedEntity = serde_json::from_slice(&data)
                    .map_err(RepositoryError::Serialization)?;
                
                if wrapped.metadata.operation != PersistOperation::Delete {
                    let entity: T = serde_json::from_slice(&wrapped.entity)
                        .map_err(RepositoryError::Serialization)?;
                    entities.push(entity);
                }
            }
        }
        
        Ok(entities)
    }
    
    async fn exists(&self, id: &ID) -> Result<bool, Self::Error> {
        Ok(self.find_by_id(id).await?.is_some())
    }
    
    async fn count(&self) -> Result<usize, Self::Error> {
        Ok(self.find_all().await?.len())
    }
}

// ヘルパー関数
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
```

## 5. IndexManager - インデックス管理

### 5.1 概要
Arweaveのタグベースクエリを効率化するインデックス管理システム。

### 5.2 実装

```rust
/// インデックスマネージャー
pub struct IndexManager {
    /// インデックスストレージ（Arweave）
    storage: Arc<dyn ArweaveClient>,
    
    /// ローカルキャッシュ
    local_index: Arc<RwLock<HashMap<String, IndexEntry>>>,
    
    /// インデックス更新キュー
    update_queue: Arc<Mutex<Vec<IndexUpdate>>>,
}

/// インデックスエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexEntry {
    entity_id: String,
    entity_type: String,
    latest_tx_id: String,
    version: u64,
    updated_at: u64,
    previous_tx_ids: Vec<String>,
}

/// インデックス更新
#[derive(Debug, Clone)]
struct IndexUpdate {
    entity_type: String,
    entity_id: String,
    tx_id: String,
    timestamp: u64,
}

impl IndexManager {
    /// インデックスの更新
    pub async fn update_index(
        &self,
        entity_type: &str,
        entity_id: &str,
        tx_id: &str,
    ) -> Result<(), RepositoryError> {
        // 1. ローカルインデックス更新
        let mut index = self.local_index.write().await;
        let key = format!("{}:{}", entity_type, entity_id);
        
        let entry = index.entry(key.clone()).or_insert(IndexEntry {
            entity_id: entity_id.to_string(),
            entity_type: entity_type.to_string(),
            latest_tx_id: tx_id.to_string(),
            version: 0,
            updated_at: current_timestamp(),
            previous_tx_ids: vec![],
        });
        
        // 履歴保持
        entry.previous_tx_ids.push(entry.latest_tx_id.clone());
        entry.latest_tx_id = tx_id.to_string();
        entry.version += 1;
        entry.updated_at = current_timestamp();
        
        // 2. 更新キューに追加
        let update = IndexUpdate {
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            tx_id: tx_id.to_string(),
            timestamp: current_timestamp(),
        };
        
        self.update_queue.lock().await.push(update);
        
        // 3. バッチ処理トリガー（一定数溜まったら）
        if self.update_queue.lock().await.len() >= 100 {
            self.flush_updates().await?;
        }
        
        Ok(())
    }
    
    /// 最新トランザクションID取得
    pub async fn get_latest_tx(
        &self,
        entity_id: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let index = self.local_index.read().await;
        
        // ローカルインデックスから検索
        for (_, entry) in index.iter() {
            if entry.entity_id == entity_id {
                return Ok(Some(entry.latest_tx_id.clone()));
            }
        }
        
        // Arweaveから検索
        self.load_from_arweave(entity_id).await
    }
    
    /// バッチ更新のフラッシュ
    async fn flush_updates(&self) -> Result<(), RepositoryError> {
        let updates = {
            let mut queue = self.update_queue.lock().await;
            std::mem::take(&mut *queue)
        };
        
        if updates.is_empty() {
            return Ok(());
        }
        
        // インデックスマニフェスト作成
        let manifest = IndexManifest {
            updates,
            timestamp: current_timestamp(),
        };
        
        let data = serde_json::to_vec(&manifest)
            .map_err(RepositoryError::Serialization)?;
        
        let tags = HashMap::from([
            ("App-Name".to_string(), "D-TPRES".to_string()),
            ("Type".to_string(), "INDEX_MANIFEST".to_string()),
            ("Timestamp".to_string(), current_timestamp().to_string()),
        ]);
        
        self.storage
            .store_data(data, tags)
            .await
            .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
        
        Ok(())
    }
    
    /// Arweaveからインデックス読み込み
    async fn load_from_arweave(
        &self,
        entity_id: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let tags = HashMap::from([
            ("App-Name".to_string(), "D-TPRES".to_string()),
            ("Entity-Id".to_string(), entity_id.to_string()),
        ]);
        
        let tx_ids = self.storage
            .query_by_tags(tags)
            .await
            .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
        
        // 最新のトランザクションを返す
        Ok(tx_ids.first().cloned())
    }
}

/// インデックスマニフェスト
#[derive(Debug, Serialize, Deserialize)]
struct IndexManifest {
    updates: Vec<IndexUpdate>,
    timestamp: u64,
}
```

## 6. CacheManager - キャッシュ管理

### 6.1 概要
読み込み性能向上のための多層キャッシュシステム。

### 6.2 実装

```rust
use lru::LruCache;
use std::num::NonZeroUsize;

/// キャッシュマネージャー
pub struct CacheManager {
    /// LRUキャッシュ
    memory_cache: Arc<Mutex<LruCache<String, CachedItem>>>,
    
    /// キャッシュ統計
    stats: Arc<RwLock<CacheStats>>,
    
    /// TTL設定（秒）
    default_ttl_seconds: u64,
}

/// キャッシュアイテム
#[derive(Debug, Clone)]
struct CachedItem {
    data: Vec<u8>,
    cached_at: u64,
    ttl: u64,
}

/// キャッシュ統計
#[derive(Debug, Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl CacheManager {
    pub fn new(capacity: usize, default_ttl_seconds: u64) -> Self {
        Self {
            memory_cache: Arc::new(Mutex::new(
                LruCache::new(NonZeroUsize::new(capacity).unwrap())
            )),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            default_ttl_seconds,
        }
    }
    
    /// キャッシュから取得
    pub async fn get<T>(&self, key: &str) -> Option<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut cache = self.memory_cache.lock().await;
        
        if let Some(item) = cache.get_mut(key) {
            // TTLチェック
            if current_timestamp() > item.cached_at + item.ttl {
                cache.pop(key);
                self.stats.write().await.misses += 1;
                return None;
            }
            
            // デシリアライズ
            if let Ok(value) = serde_json::from_slice(&item.data) {
                self.stats.write().await.hits += 1;
                return Some(value);
            }
        }
        
        self.stats.write().await.misses += 1;
        None
    }
    
    /// キャッシュに設定
    pub async fn set<T>(&self, key: &str, value: T)
    where
        T: Serialize,
    {
        if let Ok(data) = serde_json::to_vec(&value) {
            let item = CachedItem {
                data,
                cached_at: current_timestamp(),
                ttl: self.default_ttl_seconds,
            };
            
            let mut cache = self.memory_cache.lock().await;
            if cache.push(key.to_string(), item).is_some() {
                self.stats.write().await.evictions += 1;
            }
        }
    }
    
    /// キャッシュ無効化
    pub async fn invalidate(&self, key: &str) {
        self.memory_cache.lock().await.pop(key);
    }
    
    /// キャッシュクリア
    pub async fn clear(&self) {
        self.memory_cache.lock().await.clear();
    }
    
    /// 統計取得
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }
}
```

## 7. QueryOptimizer - クエリ最適化

### 7.1 概要
Arweaveクエリの最適化とクエリプラン生成。

### 7.2 実装

```rust
/// クエリ最適化器
pub struct QueryOptimizer {
    /// クエリ統計
    query_stats: Arc<RwLock<HashMap<String, QueryStats>>>,
    
    /// 最適化ルール
    optimization_rules: Vec<Box<dyn OptimizationRule>>,
}

/// クエリ統計
#[derive(Debug, Clone)]
struct QueryStats {
    execution_count: u64,
    total_duration_ms: u64,
    average_result_size: usize,
    last_executed: u64,
}

/// クエリプラン
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub query_type: QueryType,
    pub filters: Vec<QueryFilter>,
    pub optimizations: Vec<Optimization>,
    pub estimated_cost: u64,
}

/// クエリタイプ
#[derive(Debug, Clone)]
pub enum QueryType {
    FindAll,
    FindByTags(HashMap<String, String>),
    FindByRange { start: u64, end: u64 },
}

/// クエリフィルタ
#[derive(Debug, Clone)]
pub struct QueryFilter {
    pub field: String,
    pub operator: FilterOperator,
    pub value: String,
}

/// フィルタ演算子
#[derive(Debug, Clone)]
pub enum FilterOperator {
    Equals,
    Contains,
    GreaterThan,
    LessThan,
}

/// 最適化
#[derive(Debug, Clone)]
pub enum Optimization {
    UseIndex(String),
    BatchFetch(usize),
    ParallelExecution(usize),
    CacheHint(u64),
}

/// 最適化ルール
trait OptimizationRule: Send + Sync {
    fn apply(&self, plan: &mut QueryPlan) -> Result<(), RepositoryError>;
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            query_stats: Arc::new(RwLock::new(HashMap::new())),
            optimization_rules: vec![
                Box::new(IndexUsageRule),
                Box::new(BatchSizeRule),
                Box::new(ParallelizationRule),
            ],
        }
    }
    
    /// find_allクエリの最適化
    pub async fn optimize_find_all_query(
        &self,
        entity_type: &str,
    ) -> Result<QueryPlan, RepositoryError> {
        let mut plan = QueryPlan {
            query_type: QueryType::FindAll,
            filters: vec![
                QueryFilter {
                    field: "Entity-Type".to_string(),
                    operator: FilterOperator::Equals,
                    value: entity_type.to_string(),
                },
            ],
            optimizations: vec![],
            estimated_cost: 100,
        };
        
        // 統計情報に基づく最適化
        if let Some(stats) = self.query_stats.read().await.get(entity_type) {
            if stats.average_result_size > 1000 {
                plan.optimizations.push(Optimization::BatchFetch(100));
            }
            if stats.average_result_size > 10000 {
                plan.optimizations.push(Optimization::ParallelExecution(4));
            }
        }
        
        // ルールベース最適化
        for rule in &self.optimization_rules {
            rule.apply(&mut plan)?;
        }
        
        Ok(plan)
    }
}

/// インデックス使用ルール
struct IndexUsageRule;

impl OptimizationRule for IndexUsageRule {
    fn apply(&self, plan: &mut QueryPlan) -> Result<(), RepositoryError> {
        // Entity-Typeフィルタがある場合はインデックスを使用
        for filter in &plan.filters {
            if filter.field == "Entity-Type" {
                plan.optimizations.push(Optimization::UseIndex("entity_type_index".to_string()));
                plan.estimated_cost = plan.estimated_cost.saturating_sub(20);
            }
        }
        Ok(())
    }
}

/// バッチサイズルール
struct BatchSizeRule;

impl OptimizationRule for BatchSizeRule {
    fn apply(&self, plan: &mut QueryPlan) -> Result<(), RepositoryError> {
        // デフォルトバッチサイズを設定
        if !plan.optimizations.iter().any(|o| matches!(o, Optimization::BatchFetch(_))) {
            plan.optimizations.push(Optimization::BatchFetch(50));
        }
        Ok(())
    }
}

/// 並列化ルール
struct ParallelizationRule;

impl OptimizationRule for ParallelizationRule {
    fn apply(&self, plan: &mut QueryPlan) -> Result<(), RepositoryError> {
        // 高コストクエリは並列化
        if plan.estimated_cost > 500 {
            plan.optimizations.push(Optimization::ParallelExecution(2));
        }
        Ok(())
    }
}
```

## 8. Entity別Repository実装

### 8.1 ProcessEntityRepositoryImpl

```rust
use crate::domain::entity::{ProcessEntity, OwnerData, HolderData, RequesterData};
use crate::domain::repository::ProcessEntityRepository;

/// ProcessEntityのRepository実装
pub struct ProcessEntityRepositoryImpl {
    base: Arc<ArweaveRepositoryImpl<ProcessEntity, String>>,
}

impl ProcessEntityRepositoryImpl {
    pub fn new(
        arweave_client: Arc<dyn ArweaveClient>,
        index_manager: Arc<IndexManager>,
        cache_manager: Arc<CacheManager>,
        query_optimizer: Arc<QueryOptimizer>,
    ) -> Self {
        Self {
            base: Arc::new(ArweaveRepositoryImpl::new(
                arweave_client,
                "ProcessEntity",
                index_manager,
                cache_manager,
                query_optimizer,
            )),
        }
    }
}

#[async_trait]
impl Repository<ProcessEntity, String> for ProcessEntityRepositoryImpl {
    type Error = RepositoryError;
    
    // 基本CRUD操作は基底実装に委譲
    async fn create(&self, entity: &ProcessEntity) -> Result<(), Self::Error> {
        self.base.create(entity).await
    }
    
    async fn find_by_id(&self, id: &String) -> Result<Option<ProcessEntity>, Self::Error> {
        self.base.find_by_id(id).await
    }
    
    async fn update(&self, entity: &ProcessEntity) -> Result<(), Self::Error> {
        self.base.update(entity).await
    }
    
    async fn delete(&self, id: &String) -> Result<(), Self::Error> {
        self.base.delete(id).await
    }
    
    async fn find_all(&self) -> Result<Vec<ProcessEntity>, Self::Error> {
        self.base.find_all().await
    }
    
    async fn exists(&self, id: &String) -> Result<bool, Self::Error> {
        self.base.exists(id).await
    }
    
    async fn count(&self) -> Result<usize, Self::Error> {
        self.base.count().await
    }
}

#[async_trait]
impl ProcessEntityRepository for ProcessEntityRepositoryImpl {
    async fn find_by_name(&self, name: &str) -> Result<Option<ProcessEntity>, Self::Error> {
        let tags = HashMap::from([
            ("App-Name".to_string(), "D-TPRES".to_string()),
            ("Entity-Type".to_string(), "ProcessEntity".to_string()),
            ("Process-Name".to_string(), name.to_string()),
        ]);
        
        let tx_ids = self.base.arweave_client
            .query_by_tags(tags)
            .await
            .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
        
        if let Some(tx_id) = tx_ids.first() {
            let data = self.base.arweave_client
                .get_data(tx_id)
                .await
                .map_err(|e| RepositoryError::Storage(Box::new(e)))?;
            
            let entity = serde_json::from_slice(&data)
                .map_err(RepositoryError::Serialization)?;
            
            Ok(Some(entity))
        } else {
            Ok(None)
        }
    }
    
    async fn find_by_active_role(&self, role: &str) -> Result<Vec<ProcessEntity>, Self::Error> {
        // 全プロセスを取得してフィルタリング
        let all_processes = self.find_all().await?;
        
        Ok(all_processes.into_iter()
            .filter(|p| p.active_roles.contains(&role.to_string()))
            .collect())
    }
    
    async fn find_processes_with_owner_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error> {
        let all_processes = self.find_all().await?;
        
        Ok(all_processes.into_iter()
            .filter(|p| p.owner_data.is_some())
            .collect())
    }
    
    async fn find_processes_with_holder_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error> {
        let all_processes = self.find_all().await?;
        
        Ok(all_processes.into_iter()
            .filter(|p| p.holder_data.is_some())
            .collect())
    }
    
    async fn find_processes_with_requester_capability(&self) -> Result<Vec<ProcessEntity>, Self::Error> {
        let all_processes = self.find_all().await?;
        
        Ok(all_processes.into_iter()
            .filter(|p| p.requester_data.is_some())
            .collect())
    }
    
    async fn find_holders_by_reliability_desc(&self, limit: usize) -> Result<Vec<ProcessEntity>, Self::Error> {
        let mut holders = self.find_processes_with_holder_capability().await?;
        
        // 信頼性スコアで降順ソート
        holders.sort_by(|a, b| {
            let score_a = a.holder_data.as_ref().map(|h| h.reliability_score).unwrap_or(0.0);
            let score_b = b.holder_data.as_ref().map(|h| h.reliability_score).unwrap_or(0.0);
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        holders.truncate(limit);
        Ok(holders)
    }
    
    async fn find_holders_by_load_asc(&self, max_load: u64) -> Result<Vec<ProcessEntity>, Self::Error> {
        let mut holders = self.find_processes_with_holder_capability().await?;
        
        // 負荷でフィルタリングして昇順ソート
        holders.retain(|p| {
            p.holder_data.as_ref()
                .map(|h| h.current_load <= max_load)
                .unwrap_or(false)
        });
        
        holders.sort_by_key(|p| {
            p.holder_data.as_ref()
                .map(|h| h.current_load)
                .unwrap_or(u64::MAX)
        });
        
        Ok(holders)
    }
    
    async fn find_by_crypto_operation(&self, operation: &str) -> Result<Vec<ProcessEntity>, Self::Error> {
        let all_processes = self.find_all().await?;
        
        Ok(all_processes.into_iter()
            .filter(|p| p.supported_crypto_operations.contains(&operation.to_string()))
            .collect())
    }
    
    async fn update_performance_metrics(
        &self,
        process_id: &str,
        metrics: &PerformanceMetrics,
    ) -> Result<(), Self::Error> {
        if let Some(mut process) = self.find_by_id(&process_id.to_string()).await? {
            process.performance_metrics = metrics.clone();
            process.updated_at = current_timestamp();
            process.version += 1;
            
            self.update(&process).await?;
        } else {
            return Err(RepositoryError::NotFound {
                id: process_id.to_string(),
            });
        }
        
        Ok(())
    }
    
    async fn update_owner_data(
        &self,
        process_id: &str,
        owner_data: &OwnerData,
    ) -> Result<(), Self::Error> {
        if let Some(mut process) = self.find_by_id(&process_id.to_string()).await? {
            process.owner_data = Some(owner_data.clone());
            if !process.active_roles.contains(&"owner".to_string()) {
                process.active_roles.push("owner".to_string());
            }
            process.updated_at = current_timestamp();
            process.version += 1;
            
            self.update(&process).await?;
        } else {
            return Err(RepositoryError::NotFound {
                id: process_id.to_string(),
            });
        }
        
        Ok(())
    }
    
    async fn update_holder_data(
        &self,
        process_id: &str,
        holder_data: &HolderData,
    ) -> Result<(), Self::Error> {
        if let Some(mut process) = self.find_by_id(&process_id.to_string()).await? {
            process.holder_data = Some(holder_data.clone());
            if !process.active_roles.contains(&"holder".to_string()) {
                process.active_roles.push("holder".to_string());
            }
            process.updated_at = current_timestamp();
            process.version += 1;
            
            self.update(&process).await?;
        } else {
            return Err(RepositoryError::NotFound {
                id: process_id.to_string(),
            });
        }
        
        Ok(())
    }
    
    async fn update_requester_data(
        &self,
        process_id: &str,
        requester_data: &RequesterData,
    ) -> Result<(), Self::Error> {
        if let Some(mut process) = self.find_by_id(&process_id.to_string()).await? {
            process.requester_data = Some(requester_data.clone());
            if !process.active_roles.contains(&"requester".to_string()) {
                process.active_roles.push("requester".to_string());
            }
            process.updated_at = current_timestamp();
            process.version += 1;
            
            self.update(&process).await?;
        } else {
            return Err(RepositoryError::NotFound {
                id: process_id.to_string(),
            });
        }
        
        Ok(())
    }
}

// ArweaveRepositoryImplの特殊化
impl ArweaveRepositoryImpl<ProcessEntity, String> {
    fn extract_entity_id(&self, entity: &ProcessEntity) -> Result<String, RepositoryError> {
        Ok(entity.process_id.clone())
    }
    
    fn check_version_conflict(
        &self,
        existing: &ProcessEntity,
        new: &ProcessEntity,
    ) -> Result<(), RepositoryError> {
        if existing.version >= new.version {
            return Err(RepositoryError::ConcurrentModification {
                id: existing.process_id.clone(),
            });
        }
        Ok(())
    }
    
    fn create_deletion_marker(&self, id: &String) -> ProcessEntity {
        ProcessEntity {
            process_id: id.clone(),
            process_name: format!("DELETED_{}", id),
            active_roles: vec![],
            owner_data: None,
            holder_data: None,
            requester_data: None,
            configuration: HashMap::new(),
            supported_crypto_operations: vec![],
            performance_metrics: PerformanceMetrics {
                successful_operations: 0,
                failed_operations: 0,
                average_response_time_ms: 0,
                last_updated_at: current_timestamp(),
            },
            created_at: current_timestamp(),
            updated_at: current_timestamp(),
            version: u64::MAX,
        }
    }
}
```

## 9. エラー処理とリトライ戦略

### 9.1 包括的エラー処理

```rust
use thiserror::Error;

/// Repository層のエラー型
#[derive(Debug, Error)]
pub enum RepositoryError {
    /// エンティティが見つからない
    #[error("Entity not found: {id}")]
    NotFound { id: String },
    
    /// エンティティが既に存在する
    #[error("Entity already exists: {id}")]
    AlreadyExists { id: String },
    
    /// 楽観ロックエラー
    #[error("Concurrent modification detected for entity: {id}")]
    ConcurrentModification { id: String },
    
    /// バリデーションエラー
    #[error("Validation error: {message}")]
    ValidationError { message: String },
    
    /// ストレージエラー
    #[error("Storage error: {0}")]
    Storage(#[from] Box<dyn Error + Send + Sync>),
    
    /// シリアライゼーションエラー
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// インデックスエラー
    #[error("Index error: {message}")]
    IndexError { message: String },
    
    /// キャッシュエラー
    #[error("Cache error: {message}")]
    CacheError { message: String },
    
    /// タイムアウト
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    /// ネットワークエラー
    #[error("Network error: {message}")]
    Network { message: String },
    
    /// 内部エラー
    #[error("Internal error: {0}")]
    Internal(String),
}

/// リトライ可能なエラーの判定
impl RepositoryError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            RepositoryError::Network { .. } |
            RepositoryError::Timeout { .. } |
            RepositoryError::Storage(_)
        )
    }
}
```

## 10. パフォーマンス最適化

### 10.1 バッチ処理最適化

```rust
/// バッチ処理ユーティリティ
pub struct BatchProcessor {
    max_batch_size: usize,
    max_concurrent_batches: usize,
}

impl BatchProcessor {
    pub async fn process_in_batches<T, F, R>(
        &self,
        items: Vec<T>,
        processor: F,
    ) -> Result<Vec<R>, RepositoryError>
    where
        T: Send + 'static,
        F: Fn(Vec<T>) -> futures::future::BoxFuture<'static, Result<Vec<R>, RepositoryError>> + Clone + Send + Sync,
        R: Send + 'static,
    {
        use futures::stream::{self, StreamExt};
        
        let batches: Vec<Vec<T>> = items
            .chunks(self.max_batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        let results: Vec<Result<Vec<R>, RepositoryError>> = stream::iter(batches)
            .map(|batch| {
                let processor = processor.clone();
                async move { processor(batch).await }
            })
            .buffer_unordered(self.max_concurrent_batches)
            .collect()
            .await;
        
        let mut all_results = Vec::new();
        for result in results {
            match result {
                Ok(mut batch_results) => all_results.append(&mut batch_results),
                Err(e) => return Err(e),
            }
        }
        
        Ok(all_results)
    }
}
```

## 11. 監視とメトリクス

### 11.1 パフォーマンスメトリクス収集

```rust
/// メトリクス収集器
pub struct MetricsCollector {
    operation_durations: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
    error_counts: Arc<RwLock<HashMap<String, u64>>>,
}

impl MetricsCollector {
    pub async fn record_operation(
        &self,
        operation: &str,
        duration: Duration,
    ) {
        let mut durations = self.operation_durations.write().await;
        durations.entry(operation.to_string())
            .or_insert_with(Vec::new)
            .push(duration);
    }
    
    pub async fn record_error(&self, operation: &str) {
        let mut counts = self.error_counts.write().await;
        *counts.entry(operation.to_string()).or_insert(0) += 1;
    }
    
    pub async fn get_metrics(&self) -> PerformanceReport {
        let durations = self.operation_durations.read().await;
        let errors = self.error_counts.read().await;
        
        let mut operations = Vec::new();
        
        for (op, times) in durations.iter() {
            if !times.is_empty() {
                let avg = times.iter().sum::<Duration>() / times.len() as u32;
                let min = times.iter().min().cloned().unwrap_or_default();
                let max = times.iter().max().cloned().unwrap_or_default();
                
                operations.push(OperationMetrics {
                    operation: op.clone(),
                    count: times.len() as u64,
                    avg_duration: avg,
                    min_duration: min,
                    max_duration: max,
                    error_count: errors.get(op).cloned().unwrap_or(0),
                });
            }
        }
        
        PerformanceReport {
            timestamp: current_timestamp(),
            operations,
        }
    }
}

/// パフォーマンスレポート
#[derive(Debug, Serialize)]
pub struct PerformanceReport {
    pub timestamp: u64,
    pub operations: Vec<OperationMetrics>,
}

/// 操作メトリクス
#[derive(Debug, Serialize)]
pub struct OperationMetrics {
    pub operation: String,
    pub count: u64,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub error_count: u64,
}
```

## 12. 実装ガイドライン

### 12.1 Repository実装チェックリスト

- [ ] 基底実装（ArweaveRepositoryImpl）の継承
- [ ] Entity固有のクエリメソッド実装
- [ ] 適切なタグ設計
- [ ] キャッシュ戦略の決定
- [ ] エラーハンドリング
- [ ] パフォーマンスメトリクス統合
- [ ] 単体テスト作成
- [ ] 統合テスト作成

### 12.2 Arweaveタグ設計ベストプラクティス

```rust
// 推奨タグ構造
let tags = HashMap::from([
    // 必須タグ
    ("App-Name", "D-TPRES"),
    ("Entity-Type", "ShareEntity"),
    ("Entity-Id", "share_001"),
    ("Operation", "CREATE"),
    ("Timestamp", "1703001600"),
    
    // Entity固有タグ
    ("Data-Id", "data_001"),
    ("Secret-Id", "secret_001"),
    ("Owner-Key", "0xabcd..."),
    
    // インデックス用タグ
    ("Index-Version", "1"),
    ("Index-Type", "primary"),
]);
```

## 13. まとめ

D-TPRES Repository Implementation層は以下の特徴を持ちます：

1. **Arweave最適化**: 不変ストレージの特性を活かした設計
2. **高性能**: 多層キャッシュとクエリ最適化
3. **信頼性**: 包括的エラー処理とリトライ戦略
4. **拡張性**: 新しいEntityの追加が容易
5. **監視可能性**: 詳細なメトリクス収集

この実装により、Arweaveの永続性とD-TPRESの要求する高速なデータアクセスを両立します。

---

**Document Status**: Repository Implementation Specification  
**Version**: 1.0  
**Next Steps**: 各Entity別Repository実装の開発開始