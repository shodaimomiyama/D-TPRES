//! ShareCollectionRepository trait definition
//!
//! Repository interface for ShareCollection entity persistence operations.

use async_trait::async_trait;

use crate::domain::entities::ShareCollection;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{SecretId, ShareCollectionId};

use super::Repository;

/// Repository interface for ShareCollection entity
///
/// ShareCollection manages encrypted Shamir secret shares. It is associated with
/// a parent Secret via SecretId and may have an Arweave TX ID for permanent storage.
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait ShareCollectionRepository: Repository<ShareCollection, ShareCollectionId> {
    /// Find ShareCollection by its parent Secret ID
    async fn find_by_secret_id(
        &self,
        secret_id: &SecretId,
    ) -> DomainResult<Option<ShareCollection>>;
}

/// Repository interface for ShareCollection entity (WASM version)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait ShareCollectionRepository: Repository<ShareCollection, ShareCollectionId> {
    /// Find ShareCollection by its parent Secret ID
    async fn find_by_secret_id(
        &self,
        secret_id: &SecretId,
    ) -> DomainResult<Option<ShareCollection>>;
}
