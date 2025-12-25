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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockCapsuleRepository {
        storage: RwLock<HashMap<CapsuleId, Capsule>>,
    }

    impl MockCapsuleRepository {
        fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Repository<Capsule, CapsuleId> for MockCapsuleRepository {
        async fn save(&self, entity: &Capsule) -> DomainResult<()> {
            self.storage
                .write()
                .unwrap()
                .insert(entity.id().clone(), entity.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &CapsuleId) -> DomainResult<Option<Capsule>> {
            Ok(self.storage.read().unwrap().get(id).cloned())
        }

        async fn delete(&self, id: &CapsuleId) -> DomainResult<()> {
            self.storage.write().unwrap().remove(id);
            Ok(())
        }

        async fn exists(&self, id: &CapsuleId) -> DomainResult<bool> {
            Ok(self.storage.read().unwrap().contains_key(id))
        }

        async fn find_by_ids(&self, ids: &[CapsuleId]) -> DomainResult<Vec<Capsule>> {
            let storage = self.storage.read().unwrap();
            Ok(ids
                .iter()
                .filter_map(|id| storage.get(id).cloned())
                .collect())
        }
    }

    #[async_trait]
    impl CapsuleRepository for MockCapsuleRepository {
        async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Option<Capsule>> {
            let storage = self.storage.read().unwrap();
            Ok(storage
                .values()
                .find(|c| c.secret_id() == secret_id)
                .cloned())
        }
    }

    fn create_test_capsule(secret_id: SecretId) -> Capsule {
        Capsule::new(secret_id, vec![1, 2, 3, 4], vec![5, 6, 7, 8]).unwrap()
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn capsule_repository_is_send_sync() {
        assert_send_sync::<MockCapsuleRepository>();
    }

    #[tokio::test]
    async fn test_capsule_not_found() {
        let repo = MockCapsuleRepository::new();
        let id = CapsuleId::generate();
        let result = repo.find_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_capsule_by_secret_id() {
        let repo = MockCapsuleRepository::new();
        let secret_id = SecretId::generate();
        let capsule = create_test_capsule(secret_id.clone());

        repo.save(&capsule).await.unwrap();

        let found = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().secret_id(), &secret_id);
    }

    #[tokio::test]
    async fn test_find_capsule_by_secret_id_not_found() {
        let repo = MockCapsuleRepository::new();
        let secret_id = SecretId::generate();
        let result = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_batch_find_capsules() {
        let repo = MockCapsuleRepository::new();
        let capsules: Vec<_> = (0..3)
            .map(|_| create_test_capsule(SecretId::generate()))
            .collect();

        for capsule in &capsules {
            repo.save(capsule).await.unwrap();
        }

        let ids: Vec<_> = capsules.iter().map(|c| c.id().clone()).collect();
        let found = repo.find_by_ids(&ids[0..2]).await.unwrap();

        assert_eq!(found.len(), 2);
    }
}
