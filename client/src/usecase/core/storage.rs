//! ArweaveStorageService - Arweaveストレージサービス
//!
//! Arweaveへのデータ永続化、トランザクション管理、データ取得を担当します。
//! AO contract communication is handled by ContractStorage (contract_storage.rs).

// Allow unwrap for RwLock operations - internal locks won't be poisoned
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::service::error::{ServiceError, ServiceResult};

/// Arweaveトランザクション
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ArweaveTransaction {
    pub id: String,
    pub data: Vec<u8>,
    pub tags: Vec<Tag>,
    pub timestamp: u64,
}

/// Arweaveタグ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tag {
    pub name: String,
    pub value: String,
}

/// トランザクションステータス
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed(String),
}

/// ストレージ設定
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StorageConfig {
    /// 最大トランザクションサイズ（バイト）
    pub max_transaction_size: usize,
    /// バッチ処理の最大アイテム数
    pub max_batch_size: usize,
    /// キャッシュの最大エントリ数
    pub cache_max_entries: usize,
    /// タグの最大数
    pub max_tags_per_transaction: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_transaction_size: 2 * 1024 * 1024, // 2MB
            max_batch_size: 100,
            cache_max_entries: 1000,
            max_tags_per_transaction: 20,
        }
    }
}

/// クエリパラメータ
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct QueryParams {
    pub tags: Vec<Tag>,
    pub limit: Option<usize>,
    pub sort_by: Option<SortBy>,
}

/// ソート順
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SortBy {
    Timestamp(SortOrder),
    Id(SortOrder),
}

/// ソート順序
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// バッチ操作結果
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BatchResult {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>, // (id, error_message)
}

/// ArweaveStorageService trait
pub trait ArweaveStorageService: Send + Sync {
    /// データをArweaveに保存
    fn store_data(&self, data: &[u8], tags: Vec<Tag>) -> ServiceResult<String>;

    /// トランザクションIDでデータを取得
    fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction>;

    /// タグベースでデータをクエリ
    fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>>;

    /// トランザクションステータスを確認
    fn check_transaction_status(&self, transaction_id: &str) -> ServiceResult<TransactionStatus>;

    /// バッチでデータを保存
    fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult>;

    /// データの存在確認
    fn exists(&self, transaction_id: &str) -> ServiceResult<bool>;

    /// タグを更新（新しいトランザクションとして作成）
    fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String>;
}

/// ArweaveStorageService実装
pub struct ArweaveStorageServiceImpl {
    config: StorageConfig,
    cache: Arc<RwLock<HashMap<String, ArweaveTransaction>>>,
    transaction_counter: Arc<RwLock<u64>>,
}

impl ArweaveStorageServiceImpl {
    /// 新しいArweaveStorageServiceインスタンスを作成
    pub fn new(config: StorageConfig) -> Self {
        Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            transaction_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// トランザクションIDを生成
    fn generate_transaction_id(&self) -> String {
        let counter_value = {
            let mut counter = self.transaction_counter.write().unwrap();
            *counter += 1;
            *counter
        };
        format!("tx_{counter_value:016x}")
    }

    /// データサイズを検証
    fn validate_data_size(&self, payload: &[u8]) -> ServiceResult<()> {
        if payload.is_empty() {
            return Err(ServiceError::validation_error("Data cannot be empty"));
        }

        if payload.len() > self.config.max_transaction_size {
            return Err(ServiceError::validation_error(format!(
                "Data size {} exceeds maximum allowed size {}",
                payload.len(),
                self.config.max_transaction_size
            )));
        }

        Ok(())
    }

    /// タグを検証
    fn validate_tags(&self, tags: &[Tag]) -> ServiceResult<()> {
        if tags.len() > self.config.max_tags_per_transaction {
            return Err(ServiceError::validation_error(format!(
                "Number of tags {} exceeds maximum allowed {}",
                tags.len(),
                self.config.max_tags_per_transaction
            )));
        }

        for tag in tags {
            if tag.name.is_empty() || tag.value.is_empty() {
                return Err(ServiceError::validation_error(
                    "Tag name and value cannot be empty",
                ));
            }
        }

        Ok(())
    }
}

impl ArweaveStorageService for ArweaveStorageServiceImpl {
    fn store_data(&self, payload: &[u8], tags: Vec<Tag>) -> ServiceResult<String> {
        // 入力検証
        self.validate_data_size(payload)?;
        self.validate_tags(&tags)?;

        // トランザクション作成
        let tx_id = self.generate_transaction_id();
        let transaction = ArweaveTransaction {
            id: tx_id.clone(),
            data: payload.to_vec(),
            tags,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // キャッシュに保存（実際の実装ではArweaveに送信）
        self.cache
            .write()
            .unwrap()
            .insert(tx_id.clone(), transaction);

        Ok(tx_id)
    }

    fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction> {
        // 入力検証
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        // キャッシュから取得（実際の実装ではArweaveから取得）
        self.cache
            .read()
            .unwrap()
            .get(transaction_id)
            .cloned()
            .ok_or_else(|| {
                ServiceError::not_found(format!("Transaction {} not found", transaction_id))
            })
    }

    fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>> {
        // 入力検証
        if params.tags.is_empty() {
            return Err(ServiceError::validation_error(
                "Query must include at least one tag",
            ));
        }

        let cache = self.cache.read().unwrap();
        let mut results: Vec<ArweaveTransaction> = cache
            .values()
            .filter(|tx| {
                // すべてのクエリタグがトランザクションに含まれているかチェック
                params.tags.iter().all(|query_tag| {
                    tx.tags.iter().any(|tx_tag| {
                        tx_tag.name == query_tag.name && tx_tag.value == query_tag.value
                    })
                })
            })
            .cloned()
            .collect();

        // ソート処理
        if let Some(sort_by) = params.sort_by {
            match sort_by {
                SortBy::Timestamp(order) => {
                    results.sort_by(|a, b| match order {
                        SortOrder::Ascending => a.timestamp.cmp(&b.timestamp),
                        SortOrder::Descending => b.timestamp.cmp(&a.timestamp),
                    });
                }
                SortBy::Id(order) => {
                    results.sort_by(|a, b| match order {
                        SortOrder::Ascending => a.id.cmp(&b.id),
                        SortOrder::Descending => b.id.cmp(&a.id),
                    });
                }
            }
        }

        // リミット適用
        if let Some(limit) = params.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    fn check_transaction_status(&self, transaction_id: &str) -> ServiceResult<TransactionStatus> {
        // 入力検証
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        // シミュレーション：キャッシュに存在すれば確認済み
        if self.cache.read().unwrap().contains_key(transaction_id) {
            Ok(TransactionStatus::Confirmed)
        } else {
            Ok(TransactionStatus::Pending)
        }
    }

    fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult> {
        // 入力検証
        if items.is_empty() {
            return Ok(BatchResult {
                successful: Vec::new(),
                failed: Vec::new(),
            });
        }

        if items.len() > self.config.max_batch_size {
            return Err(ServiceError::validation_error(format!(
                "Batch size {} exceeds maximum allowed {}",
                items.len(),
                self.config.max_batch_size
            )));
        }

        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for (index, (item_payload, tags)) in items.into_iter().enumerate() {
            match self.store_data(&item_payload, tags) {
                Ok(tx_id) => successful.push(tx_id),
                Err(e) => failed.push((index.to_string(), e.to_string())),
            }
        }

        Ok(BatchResult { successful, failed })
    }

    fn exists(&self, transaction_id: &str) -> ServiceResult<bool> {
        // 入力検証
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        Ok(self.cache.read().unwrap().contains_key(transaction_id))
    }

    fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String> {
        // 入力検証
        self.validate_tags(&new_tags)?;

        // 既存のトランザクションを取得
        let existing_tx = self.retrieve_data(transaction_id)?;

        // 新しいトランザクションとして保存（Arweaveは不変）
        let new_tx_id = self.store_data(&existing_tx.data, new_tags)?;

        Ok(new_tx_id)
    }
}

impl Default for ArweaveStorageServiceImpl {
    fn default() -> Self {
        Self::new(StorageConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        println!("\n=== ArweaveStorageService: Store and Retrieve Test ===");
        println!("【テスト内容】: Arweaveへのデータ保存と取得機能を検証");
        println!("【テスト対象】: store_data()とretrieve_data()メソッド");
        println!("【期待結果】: データとタグが正しく保存・取得される");

        let service = ArweaveStorageServiceImpl::default();

        // テストデータの準備
        let data = b"test data";
        let data_hex: Vec<String> = data.iter().map(|b| format!("{b:02x}")).collect();
        let data_text = std::str::from_utf8(data).unwrap();
        let data_len = data.len();
        println!("\n1. 保存するデータを準備:");
        println!("   - テキスト: \"{data_text}\"");
        println!("   - バイナリ: [{}]", data_hex.join(" "));
        println!("   - サイズ: {data_len} bytes");

        let tags = vec![
            Tag {
                name: "Type".to_string(),
                value: "Test".to_string(),
            },
            Tag {
                name: "Version".to_string(),
                value: "1.0".to_string(),
            },
        ];
        println!("\n2. タグを設定:");
        for tag in &tags {
            println!("   - {}: {}", tag.name, tag.value);
        }

        // データの保存
        println!("\n3. Arweaveにデータを保存...");
        let tx_id = service.store_data(data, tags).unwrap();
        println!("   ✓ 保存成功!");
        println!("   トランザクションID: {tx_id}");
        assert!(!tx_id.is_empty());

        // データの取得
        println!("\n4. 保存したデータを取得...");
        let retrieved = service.retrieve_data(&tx_id).unwrap();
        println!("   ✓ 取得成功!");

        // 取得したデータの検証
        println!("\n5. 取得したデータを検証:");
        println!("   - データサイズ: {} bytes", retrieved.data.len());
        println!(
            "   - データ内容: \"{}\"",
            std::str::from_utf8(&retrieved.data).unwrap()
        );
        println!("   - タグ数: {}", retrieved.tags.len());

        assert_eq!(retrieved.data, data);
        println!("   ✓ データが一致");

        assert_eq!(retrieved.tags.len(), 2);
        println!("   ✓ タグ数が一致");

        // タグの詳細を表示
        println!("\n6. 取得したタグの詳細:");
        for tag in &retrieved.tags {
            println!("   - {}: {}", tag.name, tag.value);
        }

        println!("\n✅ テスト成功: データの保存・取得が正常に動作しました！");
    }

    #[test]
    fn test_query_by_tags() {
        let service = ArweaveStorageServiceImpl::default();

        // 複数のトランザクションを作成
        for i in 0..3 {
            let data = format!("data {i}").into_bytes();
            let tags = vec![
                Tag {
                    name: "Type".to_string(),
                    value: "Test".to_string(),
                },
                Tag {
                    name: "Index".to_string(),
                    value: i.to_string(),
                },
            ];
            service.store_data(&data, tags).unwrap();
        }

        // タグでクエリ
        let params = QueryParams {
            tags: vec![Tag {
                name: "Type".to_string(),
                value: "Test".to_string(),
            }],
            limit: Some(2),
            sort_by: Some(SortBy::Timestamp(SortOrder::Descending)),
        };

        let results = service.query_by_tags(params).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_batch_store() {
        let service = ArweaveStorageServiceImpl::default();

        let items = vec![
            (
                b"data1".to_vec(),
                vec![Tag {
                    name: "ID".to_string(),
                    value: "1".to_string(),
                }],
            ),
            (
                b"data2".to_vec(),
                vec![Tag {
                    name: "ID".to_string(),
                    value: "2".to_string(),
                }],
            ),
        ];

        let result = service.batch_store(items).unwrap();
        assert_eq!(result.successful.len(), 2);
        assert_eq!(result.failed.len(), 0);
    }
}
