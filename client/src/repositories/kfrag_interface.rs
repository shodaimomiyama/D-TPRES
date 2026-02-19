//! KFragRepository trait definition
//!
//! Repository interface for KFrag entity persistence operations.

use async_trait::async_trait;

use crate::domain::entities::KFrag;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{KFragId, SecretId};

use super::Repository;

/// Repository interface for KFrag entity
///
/// KFrag (Key Fragment) contains sensitive cryptographic material and implements
/// Zeroize + ZeroizeOnDrop for secure memory handling. Each KFrag is associated
/// with a Secret via SecretId and identified by a holder_index.
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait KFragRepository: Repository<KFrag, KFragId> {
    /// Find all KFrags associated with a Secret
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<KFrag>>;

    /// Find a specific KFrag by Secret ID and holder index
    async fn find_by_holder_index(
        &self,
        secret_id: &SecretId,
        holder_index: u8,
    ) -> DomainResult<Option<KFrag>>;

    /// Delete all KFrags associated with a Secret
    async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()>;
}

/// Repository interface for KFrag entity (WASM version)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait KFragRepository: Repository<KFrag, KFragId> {
    /// Find all KFrags associated with a Secret
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<KFrag>>;

    /// Find a specific KFrag by Secret ID and holder index
    async fn find_by_holder_index(
        &self,
        secret_id: &SecretId,
        holder_index: u8,
    ) -> DomainResult<Option<KFrag>>;

    /// Delete all KFrags associated with a Secret
    async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()>;
}
