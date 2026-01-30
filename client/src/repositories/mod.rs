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
/// On native targets, implementations must be `Send + Sync` to support concurrent access.
/// On WASM targets, these bounds are relaxed for single-threaded browser execution.
#[cfg(not(target_arch = "wasm32"))]
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

/// Base Repository trait for WASM targets (single-threaded, no Send + Sync required)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait Repository<T, ID> {
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

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// Simple entity for testing
    #[derive(Clone, Debug, PartialEq)]
    struct TestEntity {
        id: String,
        data: String,
    }

    /// Mock repository implementation for testing base Repository trait
    struct MockRepository {
        storage: RwLock<HashMap<String, TestEntity>>,
    }

    impl MockRepository {
        fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Repository<TestEntity, String> for MockRepository {
        async fn save(&self, entity: &TestEntity) -> DomainResult<()> {
            self.storage
                .write()
                .unwrap()
                .insert(entity.id.clone(), entity.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &String) -> DomainResult<Option<TestEntity>> {
            Ok(self.storage.read().unwrap().get(id).cloned())
        }

        async fn delete(&self, id: &String) -> DomainResult<()> {
            self.storage.write().unwrap().remove(id);
            Ok(())
        }

        async fn exists(&self, id: &String) -> DomainResult<bool> {
            Ok(self.storage.read().unwrap().contains_key(id))
        }

        async fn find_by_ids(&self, ids: &[String]) -> DomainResult<Vec<TestEntity>> {
            let storage = self.storage.read().unwrap();
            Ok(ids
                .iter()
                .filter_map(|id| storage.get(id).cloned())
                .collect())
        }
    }

    /// Compile-time verification that Repository requires Send + Sync
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn mock_repository_is_send_sync() {
        assert_send_sync::<MockRepository>();
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let repo = MockRepository::new();
        let result = repo.find_by_id(&"nonexistent".to_string()).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: "test-1".to_string(),
            data: "test data".to_string(),
        };

        repo.save(&entity).await.unwrap();
        let found = repo.find_by_id(&"test-1".to_string()).await.unwrap();

        assert_eq!(found, Some(entity));
    }

    #[tokio::test]
    async fn test_exists() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: "test-1".to_string(),
            data: "test data".to_string(),
        };

        assert!(!repo.exists(&"test-1".to_string()).await.unwrap());
        repo.save(&entity).await.unwrap();
        assert!(repo.exists(&"test-1".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: "test-1".to_string(),
            data: "test data".to_string(),
        };

        repo.save(&entity).await.unwrap();
        assert!(repo.exists(&"test-1".to_string()).await.unwrap());

        repo.delete(&"test-1".to_string()).await.unwrap();
        assert!(!repo.exists(&"test-1".to_string()).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_succeeds() {
        let repo = MockRepository::new();
        // Deleting non-existent entity should not error
        let result = repo.delete(&"nonexistent".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_by_ids_batch_retrieval() {
        let repo = MockRepository::new();
        let entities = vec![
            TestEntity {
                id: "test-1".to_string(),
                data: "data 1".to_string(),
            },
            TestEntity {
                id: "test-2".to_string(),
                data: "data 2".to_string(),
            },
            TestEntity {
                id: "test-3".to_string(),
                data: "data 3".to_string(),
            },
        ];

        for entity in &entities {
            repo.save(entity).await.unwrap();
        }

        let ids = vec!["test-1".to_string(), "test-3".to_string()];
        let found = repo.find_by_ids(&ids).await.unwrap();

        assert_eq!(found.len(), 2);
        assert!(found.iter().any(|e| e.id == "test-1"));
        assert!(found.iter().any(|e| e.id == "test-3"));
    }

    #[tokio::test]
    async fn test_find_by_ids_with_missing_ids() {
        let repo = MockRepository::new();
        let entity = TestEntity {
            id: "test-1".to_string(),
            data: "data 1".to_string(),
        };
        repo.save(&entity).await.unwrap();

        // Request includes both existing and non-existing IDs
        let ids = vec![
            "test-1".to_string(),
            "nonexistent".to_string(),
            "also-missing".to_string(),
        ];
        let found = repo.find_by_ids(&ids).await.unwrap();

        // Only the existing entity should be returned
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_find_by_ids_empty_input() {
        let repo = MockRepository::new();
        let found = repo.find_by_ids(&[]).await.unwrap();
        assert!(found.is_empty());
    }
}
