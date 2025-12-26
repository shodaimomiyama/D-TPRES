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

/// Common tag names for D-TPRES entities
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

/// Common tag values for D-TPRES
pub mod tag_values {
    /// Application name value
    pub const APP_NAME: &str = "D-TPRES";

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
#[async_trait]
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
