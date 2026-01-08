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
#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
pub trait SecretRepository: Repository<Secret, SecretId> {}

/// Repository interface for Secret entity (WASM version)
#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
pub trait SecretRepository: Repository<Secret, SecretId> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::errors::DomainResult;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockSecretRepository {
        storage: RwLock<HashMap<SecretId, Secret>>,
    }

    impl MockSecretRepository {
        fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Repository<Secret, SecretId> for MockSecretRepository {
        async fn save(&self, entity: &Secret) -> DomainResult<()> {
            self.storage
                .write()
                .unwrap()
                .insert(entity.id().clone(), entity.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &SecretId) -> DomainResult<Option<Secret>> {
            Ok(self.storage.read().unwrap().get(id).cloned())
        }

        async fn delete(&self, id: &SecretId) -> DomainResult<()> {
            self.storage.write().unwrap().remove(id);
            Ok(())
        }

        async fn exists(&self, id: &SecretId) -> DomainResult<bool> {
            Ok(self.storage.read().unwrap().contains_key(id))
        }

        async fn find_by_ids(&self, ids: &[SecretId]) -> DomainResult<Vec<Secret>> {
            let storage = self.storage.read().unwrap();
            Ok(ids
                .iter()
                .filter_map(|id| storage.get(id).cloned())
                .collect())
        }
    }

    #[async_trait]
    impl SecretRepository for MockSecretRepository {}

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn secret_repository_is_send_sync() {
        assert_send_sync::<MockSecretRepository>();
    }

    #[tokio::test]
    async fn test_secret_not_found() {
        let repo = MockSecretRepository::new();
        let id = SecretId::generate();
        let result = repo.find_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_save_and_find_secret() {
        let repo = MockSecretRepository::new();
        let secret = Secret::new(2, 3, vec![1u8; 33]).unwrap();
        let id = secret.id().clone();

        repo.save(&secret).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), &id);
    }

    #[tokio::test]
    async fn test_batch_find_secrets() {
        let repo = MockSecretRepository::new();
        let secrets: Vec<_> = (0..3)
            .map(|i| Secret::new(2, 3, vec![(i + 1) as u8; 33]).unwrap())
            .collect();

        for secret in &secrets {
            repo.save(secret).await.unwrap();
        }

        let ids: Vec<_> = secrets.iter().map(|s| s.id().clone()).collect();
        let found = repo.find_by_ids(&ids[0..2]).await.unwrap();

        assert_eq!(found.len(), 2);
    }
}
