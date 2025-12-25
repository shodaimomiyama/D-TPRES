//! CapsuleRepository trait definition
//!
//! Repository interface for Capsule entity persistence operations.

use async_trait::async_trait;

use crate::domain::entities::Capsule;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{CapsuleId, SecretId};

use super::Repository;

/// Repository interface for Capsule entity
///
/// Capsule holds the Umbral PRE capsule generated during encryption.
/// It is public data and may have an Arweave TX ID for permanent storage.
#[async_trait]
pub trait CapsuleRepository: Repository<Capsule, CapsuleId> {
    /// Find Capsule by its parent Secret ID
    ///
    /// Returns the Capsule associated with the given SecretId, if it exists.
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Option<Capsule>>;
}
