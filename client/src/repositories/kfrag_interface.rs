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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    struct MockKFragRepository {
        storage: RwLock<HashMap<KFragId, KFrag>>,
    }

    impl MockKFragRepository {
        fn new() -> Self {
            Self {
                storage: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Repository<KFrag, KFragId> for MockKFragRepository {
        async fn save(&self, entity: &KFrag) -> DomainResult<()> {
            self.storage
                .write()
                .unwrap()
                .insert(entity.id().clone(), entity.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: &KFragId) -> DomainResult<Option<KFrag>> {
            Ok(self.storage.read().unwrap().get(id).cloned())
        }

        async fn delete(&self, id: &KFragId) -> DomainResult<()> {
            self.storage.write().unwrap().remove(id);
            Ok(())
        }

        async fn exists(&self, id: &KFragId) -> DomainResult<bool> {
            Ok(self.storage.read().unwrap().contains_key(id))
        }

        async fn find_by_ids(&self, ids: &[KFragId]) -> DomainResult<Vec<KFrag>> {
            let storage = self.storage.read().unwrap();
            Ok(ids
                .iter()
                .filter_map(|id| storage.get(id).cloned())
                .collect())
        }
    }

    #[async_trait]
    impl KFragRepository for MockKFragRepository {
        async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<KFrag>> {
            let storage = self.storage.read().unwrap();
            Ok(storage
                .values()
                .filter(|k| k.secret_id() == secret_id)
                .cloned()
                .collect())
        }

        async fn find_by_holder_index(
            &self,
            secret_id: &SecretId,
            holder_index: u8,
        ) -> DomainResult<Option<KFrag>> {
            let storage = self.storage.read().unwrap();
            Ok(storage
                .values()
                .find(|k| k.secret_id() == secret_id && k.holder_index() == holder_index)
                .cloned())
        }

        async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()> {
            let mut storage = self.storage.write().unwrap();
            storage.retain(|_, v| v.secret_id() != secret_id);
            Ok(())
        }
    }

    fn create_test_kfrag(secret_id: SecretId, holder_index: u8) -> KFrag {
        KFrag::new(secret_id, holder_index, 3, vec![1, 2, 3, 4]).unwrap()
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn kfrag_repository_is_send_sync() {
        assert_send_sync::<MockKFragRepository>();
    }

    #[tokio::test]
    async fn test_kfrag_not_found() {
        let repo = MockKFragRepository::new();
        let id = KFragId::generate();
        let result = repo.find_by_id(&id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_find_kfrags_by_secret_id() {
        let repo = MockKFragRepository::new();
        let secret_id = SecretId::generate();
        let kfrags: Vec<_> = (1..=3)
            .map(|i| create_test_kfrag(secret_id.clone(), i))
            .collect();

        for kfrag in &kfrags {
            repo.save(kfrag).await.unwrap();
        }

        let found = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert_eq!(found.len(), 3);
    }

    #[tokio::test]
    async fn test_find_kfrag_by_holder_index() {
        let repo = MockKFragRepository::new();
        let secret_id = SecretId::generate();
        let kfrag = create_test_kfrag(secret_id.clone(), 2);

        repo.save(&kfrag).await.unwrap();

        let found = repo.find_by_holder_index(&secret_id, 2).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().holder_index(), 2);
    }

    #[tokio::test]
    async fn test_find_kfrag_by_holder_index_not_found() {
        let repo = MockKFragRepository::new();
        let secret_id = SecretId::generate();
        let result = repo.find_by_holder_index(&secret_id, 1).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_delete_kfrags_by_secret_id() {
        let repo = MockKFragRepository::new();
        let secret_id = SecretId::generate();
        let other_secret_id = SecretId::generate();

        let kfrag1 = create_test_kfrag(secret_id.clone(), 1);
        let kfrag2 = create_test_kfrag(secret_id.clone(), 2);
        let kfrag3 = create_test_kfrag(other_secret_id.clone(), 1);

        repo.save(&kfrag1).await.unwrap();
        repo.save(&kfrag2).await.unwrap();
        repo.save(&kfrag3).await.unwrap();

        repo.delete_by_secret_id(&secret_id).await.unwrap();

        let remaining = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(remaining.is_empty());

        let other_remaining = repo.find_by_secret_id(&other_secret_id).await.unwrap();
        assert_eq!(other_remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_batch_find_kfrags() {
        let repo = MockKFragRepository::new();
        let secret_id = SecretId::generate();
        let kfrags: Vec<_> = (1..=3)
            .map(|i| create_test_kfrag(secret_id.clone(), i))
            .collect();

        for kfrag in &kfrags {
            repo.save(kfrag).await.unwrap();
        }

        let ids: Vec<_> = kfrags.iter().map(|k| k.id().clone()).collect();
        let found = repo.find_by_ids(&ids[0..2]).await.unwrap();

        assert_eq!(found.len(), 2);
    }
}
