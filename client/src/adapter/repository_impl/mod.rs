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
}

/// Helper functions for creating common tags
pub mod tag_helpers {
    use super::{Tag, tag_names, tag_values};

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
    use super::{AdapterError, ArweaveClient, Tag};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Entry stored in MockArweaveClient
    struct StoredEntry {
        data: Vec<u8>,
        tags: Vec<Tag>,
    }

    /// Mock implementation of ArweaveClient for testing
    ///
    /// Uses in-memory HashMap storage with RwLock for thread safety.
    /// Supports tag-based query filtering.
    pub struct MockArweaveClient {
        storage: RwLock<HashMap<String, StoredEntry>>,
        tx_counter: AtomicU64,
    }

    impl MockArweaveClient {
        /// Create a new empty MockArweaveClient
        pub fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
                tx_counter: AtomicU64::new(0),
            }
        }

        /// Generate a unique transaction ID
        fn generate_tx_id(&self) -> String {
            let id = self.tx_counter.fetch_add(1, Ordering::SeqCst);
            format!("mock_tx_{id}")
        }

        /// Check if an entry's tags match all query tags
        fn tags_match(entry_tags: &[Tag], query_tags: &[Tag]) -> bool {
            query_tags.iter().all(|query_tag| {
                entry_tags.iter().any(|entry_tag| {
                    entry_tag.name == query_tag.name && entry_tag.value == query_tag.value
                })
            })
        }

        /// Clear all stored data (useful for test cleanup)
        pub fn clear(&self) {
            let mut storage = self.storage.write().unwrap();
            storage.clear();
        }

        /// Get the number of stored entries
        pub fn len(&self) -> usize {
            let storage = self.storage.read().unwrap();
            storage.len()
        }

        /// Check if storage is empty
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

            // Sort by numeric suffix to ensure proper ordering (mock_tx_0, mock_tx_1, ...)
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
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[tokio::test]
        async fn test_mock_client_post_and_get() {
            let client = MockArweaveClient::new();
            let test_data = vec![1u8, 2, 3, 4, 5];
            let tags = vec![Tag::new("key", "value")];

            let tx_id = client.post(&test_data, tags).await.unwrap();
            let retrieved = client.get(&tx_id).await.unwrap();

            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap(), test_data);
        }

        #[tokio::test]
        async fn test_mock_client_get_not_found() {
            let client = MockArweaveClient::new();
            let result = client.get("nonexistent_tx").await.unwrap();
            assert!(result.is_none());
        }

        #[tokio::test]
        async fn test_mock_client_query_by_tags() {
            let client = MockArweaveClient::new();

            let tags1 = vec![Tag::new("type", "Secret"), Tag::new("id", "secret1")];
            let tags2 = vec![Tag::new("type", "Secret"), Tag::new("id", "secret2")];
            let tags3 = vec![Tag::new("type", "Capsule"), Tag::new("id", "capsule1")];

            client.post(&[1], tags1).await.unwrap();
            client.post(&[2], tags2).await.unwrap();
            client.post(&[3], tags3).await.unwrap();

            let query_tags = vec![Tag::new("type", "Secret")];
            let results = client.query(query_tags).await.unwrap();

            assert_eq!(results.len(), 2);
        }

        #[tokio::test]
        async fn test_mock_client_query_no_match() {
            let client = MockArweaveClient::new();

            let tags = vec![Tag::new("type", "Secret")];
            client.post(&[1], tags).await.unwrap();

            let query_tags = vec![Tag::new("type", "Capsule")];
            let results = client.query(query_tags).await.unwrap();

            assert!(results.is_empty());
        }

        #[tokio::test]
        async fn test_mock_client_clear() {
            let client = MockArweaveClient::new();

            client.post(&[1], vec![]).await.unwrap();
            client.post(&[2], vec![]).await.unwrap();

            assert_eq!(client.len(), 2);

            client.clear();

            assert!(client.is_empty());
        }
    }
}
