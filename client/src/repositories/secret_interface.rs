//! SecretRepository trait definition
//!
//! Repository interface for Secret entity (aggregate root) persistence operations.

use async_trait::async_trait;

use crate::domain::entities::Secret;
use crate::domain::value_objects::SecretId;

use super::Repository;

/// Repository interface for Secret entity
///
/// Secret is the aggregate root in D-TPRES, managing references to related entities
/// (ShareCollection, Capsule, KFrag). This repository handles Secret persistence
/// with state transition support (Initialized → Split → Distributed → Recovered).
#[async_trait]
pub trait SecretRepository: Repository<Secret, SecretId> {
    // Inherits all methods from base Repository trait:
    // - save(&self, entity: &Secret) -> DomainResult<()>
    // - find_by_id(&self, id: &SecretId) -> DomainResult<Option<Secret>>
    // - delete(&self, id: &SecretId) -> DomainResult<()>
    // - exists(&self, id: &SecretId) -> DomainResult<bool>
    // - find_by_ids(&self, ids: &[SecretId]) -> DomainResult<Vec<Secret>>
}
