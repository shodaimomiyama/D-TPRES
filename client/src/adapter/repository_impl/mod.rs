//! Repository implementation module
//!
//! Provides Arweave-based repository implementations for domain entities.
//! Uses ArweaveClient trait for abstraction, enabling dependency injection
//! and mock testing.

use async_trait::async_trait;

use crate::adapter::errors::AdapterError;

pub mod capsule_impl;
pub mod cfrag_impl;
pub mod kfrag_impl;
pub mod secret_impl;
pub mod share_impl;

pub use capsule_impl::ArweaveCapsuleRepository;
pub use cfrag_impl::ArweaveCFragRepository;
pub use kfrag_impl::ArweaveKFragRepository;
pub use secret_impl::ArweaveSecretRepository;
pub use share_impl::ArweaveShareCollectionRepository;

/// Arweave tag structure for metadata
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Tag {
    pub name: String,
    pub value: String,
}

impl Tag {
    /// Create a new tag
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Common tag names for FORMIX entities
pub mod tag_names {
    /// Application name tag
    pub const APP_NAME: &str = "App-Name";

    /// Entity type tag
    pub const ENTITY_TYPE: &str = "Entity-Type";

    /// Entity ID tag
    pub const ENTITY_ID: &str = "Entity-Id";

    /// Secret ID tag (for entity relationships)
    pub const SECRET_ID: &str = "Secret-Id";

    /// Soft-delete flag tag
    pub const DELETED: &str = "Deleted";

    /// KFrag ID tag (for CFrag relationships)
    pub const KFRAG_ID: &str = "KFrag-Id";

    /// Holder index tag
    pub const HOLDER_INDEX: &str = "Holder-Index";
}

/// Common tag values for FORMIX
pub mod tag_values {
    /// Application name value
    pub const APP_NAME: &str = "FORMIX";

    /// Entity type: Secret
    pub const ENTITY_SECRET: &str = "Secret";

    /// Entity type: ShareCollection
    pub const ENTITY_SHARE_COLLECTION: &str = "ShareCollection";

    /// Entity type: Capsule
    pub const ENTITY_CAPSULE: &str = "Capsule";

    /// Entity type: KFrag
    pub const ENTITY_KFRAG: &str = "KFrag";

    /// Entity type: CFrag
    pub const ENTITY_CFRAG: &str = "CFrag";
}

/// A query result containing full transaction metadata (tx_id, tags, timestamp).
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Arweave transaction ID
    pub tx_id: String,
    /// Tags attached to the transaction
    pub tags: Vec<Tag>,
    /// Block timestamp in seconds (0 if unavailable/not yet confirmed)
    pub timestamp: u64,
}

/// Arweave client trait for abstracting Arweave operations
///
/// This trait defines the interface for Arweave storage operations.
/// Implementations can be swapped for testing (MockArweaveClient)
/// or production use (actual Arweave client).
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(not(target_arch = "wasm32"))]
pub trait ArweaveClient: Send + Sync {
    /// Retrieve data by transaction ID
    ///
    /// Returns `Ok(Some(data))` if found, `Ok(None)` if not found,
    /// or `Err` on connection/storage errors.
    async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError>;

    /// Post data to Arweave with associated tags
    ///
    /// Returns the transaction ID on success.
    async fn post(&self, data: &[u8], tags: Vec<Tag>) -> Result<String, AdapterError>;

    /// Query for transaction IDs matching the given tags
    ///
    /// Returns a list of matching transaction IDs.
    async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError>;

    /// Query transactions with full metadata (tags + timestamp).
    ///
    /// Returns a list of [`QueryResult`] entries each containing the tx_id,
    /// the tags attached at upload time, and the block timestamp.
    async fn query_with_meta(&self, tags: Vec<Tag>) -> Result<Vec<QueryResult>, AdapterError>;
}

/// Arweave client trait for abstracting Arweave operations (WASM version)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait ArweaveClient {
    /// Retrieve data by transaction ID
    ///
    /// Returns `Ok(Some(data))` if found, `Ok(None)` if not found,
    /// or `Err` on connection/storage errors.
    async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError>;

    /// Post data to Arweave with associated tags
    ///
    /// Returns the transaction ID on success.
    async fn post(&self, data: &[u8], tags: Vec<Tag>) -> Result<String, AdapterError>;

    /// Query for transaction IDs matching the given tags
    ///
    /// Returns a list of matching transaction IDs.
    async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError>;

    /// Query transactions with full metadata (tags + timestamp).
    ///
    /// Returns a list of [`QueryResult`] entries each containing the tx_id,
    /// the tags attached at upload time, and the block timestamp.
    async fn query_with_meta(&self, tags: Vec<Tag>) -> Result<Vec<QueryResult>, AdapterError>;
}

/// Helper functions for creating common tags
pub mod tag_helpers {
    use super::{tag_names, tag_values, Tag};

    /// Create application name tag
    pub fn app_tag() -> Tag {
        Tag::new(tag_names::APP_NAME, tag_values::APP_NAME)
    }

    /// Create entity type tag
    pub fn entity_type_tag(entity_type: &str) -> Tag {
        Tag::new(tag_names::ENTITY_TYPE, entity_type)
    }

    /// Create entity ID tag
    pub fn entity_id_tag(id: &str) -> Tag {
        Tag::new(tag_names::ENTITY_ID, id)
    }

    /// Create secret ID tag
    pub fn secret_id_tag(secret_id: &str) -> Tag {
        Tag::new(tag_names::SECRET_ID, secret_id)
    }

    /// Create deleted flag tag
    pub fn deleted_tag(deleted: bool) -> Tag {
        Tag::new(tag_names::DELETED, if deleted { "true" } else { "false" })
    }

    /// Create holder index tag
    pub fn holder_index_tag(index: u8) -> Tag {
        Tag::new(tag_names::HOLDER_INDEX, index.to_string())
    }

    /// Create kfrag ID tag
    pub fn kfrag_id_tag(kfrag_id: &str) -> Tag {
        Tag::new(tag_names::KFRAG_ID, kfrag_id)
    }
}

/// Mock Arweave client module for testing
#[cfg(test)]
pub mod mock {
    use super::{AdapterError, ArweaveClient, QueryResult, Tag};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::RwLock;

    struct StoredEntry {
        data: Vec<u8>,
        tags: Vec<Tag>,
    }

    pub struct MockArweaveClient {
        storage: RwLock<HashMap<String, StoredEntry>>,
        tx_counter: AtomicU64,
    }

    impl MockArweaveClient {
        pub fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
                tx_counter: AtomicU64::new(0),
            }
        }

        fn generate_tx_id(&self) -> String {
            let id = self.tx_counter.fetch_add(1, Ordering::SeqCst);
            format!("mock_tx_{id}")
        }

        fn tags_match(entry_tags: &[Tag], query_tags: &[Tag]) -> bool {
            query_tags.iter().all(|query_tag| {
                entry_tags.iter().any(|entry_tag| {
                    entry_tag.name == query_tag.name && entry_tag.value == query_tag.value
                })
            })
        }

        pub fn clear(&self) {
            let mut storage = self.storage.write().unwrap();
            storage.clear();
        }

        pub fn len(&self) -> usize {
            let storage = self.storage.read().unwrap();
            storage.len()
        }

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }
    }

    impl Default for MockArweaveClient {
        fn default() -> Self {
            Self::new()
        }
    }

    #[async_trait]
    impl ArweaveClient for MockArweaveClient {
        async fn get(&self, tx_id: &str) -> Result<Option<Vec<u8>>, AdapterError> {
            let storage = self.storage.read().unwrap();
            Ok(storage.get(tx_id).map(|entry| entry.data.clone()))
        }

        async fn post(&self, data: &[u8], tags: Vec<Tag>) -> Result<String, AdapterError> {
            let tx_id = self.generate_tx_id();
            let entry = StoredEntry {
                data: data.to_vec(),
                tags,
            };

            let mut storage = self.storage.write().unwrap();
            storage.insert(tx_id.clone(), entry);

            Ok(tx_id)
        }

        async fn query(&self, tags: Vec<Tag>) -> Result<Vec<String>, AdapterError> {
            let storage = self.storage.read().unwrap();
            let mut matching_ids: Vec<String> = storage
                .iter()
                .filter(|(_, entry)| Self::tags_match(&entry.tags, &tags))
                .map(|(tx_id, _)| tx_id.clone())
                .collect();

            matching_ids.sort_by(|a, b| {
                let extract_num = |s: &str| {
                    s.strip_prefix("mock_tx_")
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0)
                };
                extract_num(a).cmp(&extract_num(b))
            });

            Ok(matching_ids)
        }

        async fn query_with_meta(&self, tags: Vec<Tag>) -> Result<Vec<QueryResult>, AdapterError> {
            let storage = self.storage.read().unwrap();
            let mut results: Vec<QueryResult> = storage
                .iter()
                .filter(|(_, entry)| Self::tags_match(&entry.tags, &tags))
                .map(|(tx_id, entry)| QueryResult {
                    tx_id: tx_id.clone(),
                    tags: entry.tags.clone(),
                    timestamp: 0,
                })
                .collect();

            results.sort_by(|a, b| {
                let extract_num = |s: &str| {
                    s.strip_prefix("mock_tx_")
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0)
                };
                extract_num(&a.tx_id).cmp(&extract_num(&b.tx_id))
            });

            Ok(results)
        }
    }
}
