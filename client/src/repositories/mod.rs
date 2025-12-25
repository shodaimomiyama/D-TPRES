//! Repository layer - Repository Interface definitions
//!
//! This module defines the Repository Interface traits for D-TPRES domain entities.
//! Repository interfaces abstract persistence operations and enable Dependency Inversion (DIP).

use async_trait::async_trait;

use crate::domain::errors::DomainResult;

pub mod capsule_interface;
pub mod cfrag_interface;
pub mod kfrag_interface;
pub mod secret_interface;
pub mod share_interface;

pub use capsule_interface::CapsuleRepository;
pub use cfrag_interface::CFragRepository;
pub use kfrag_interface::KFragRepository;
pub use secret_interface::SecretRepository;
pub use share_interface::ShareCollectionRepository;

/// Base Repository trait providing common CRUD operations
///
/// This trait defines the fundamental persistence operations that all entity
/// repositories must implement. It uses async methods for browser/local
/// environment compatibility.
///
/// # Type Parameters
/// - `T`: The entity type being persisted
/// - `ID`: The identifier type for the entity
///
/// # Thread Safety
/// All implementations must be `Send + Sync` to support concurrent access.
#[async_trait]
pub trait Repository<T, ID>: Send + Sync
where
    T: Send + Sync,
    ID: Send + Sync,
{
    /// Save an entity to the repository
    ///
    /// If an entity with the same ID already exists, it will be updated.
    async fn save(&self, entity: &T) -> DomainResult<()>;

    /// Find an entity by its identifier
    ///
    /// Returns `Ok(Some(entity))` if found, `Ok(None)` if not found.
    async fn find_by_id(&self, id: &ID) -> DomainResult<Option<T>>;

    /// Delete an entity by its identifier
    ///
    /// Returns `Ok(())` even if the entity does not exist.
    async fn delete(&self, id: &ID) -> DomainResult<()>;

    /// Check if an entity exists by its identifier
    async fn exists(&self, id: &ID) -> DomainResult<bool>;

    /// Find multiple entities by their identifiers (batch operation)
    ///
    /// Returns only the entities that were found. Missing IDs are silently ignored.
    async fn find_by_ids(&self, ids: &[ID]) -> DomainResult<Vec<T>>;
}
