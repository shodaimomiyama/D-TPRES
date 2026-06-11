use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use formix::domain::{
    DomainResult, EncryptedShareData, SecretId, ShareCollection, ShareCollectionId,
};
use formix::repositories::{Repository, ShareCollectionRepository};

struct MockShareCollectionRepository {
    storage: RwLock<HashMap<ShareCollectionId, ShareCollection>>,
}

impl MockShareCollectionRepository {
    fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Repository<ShareCollection, ShareCollectionId> for MockShareCollectionRepository {
    async fn save(&self, entity: &ShareCollection) -> DomainResult<()> {
        self.storage
            .write()
            .unwrap()
            .insert(entity.id().clone(), entity.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &ShareCollectionId) -> DomainResult<Option<ShareCollection>> {
        Ok(self.storage.read().unwrap().get(id).cloned())
    }

    async fn delete(&self, id: &ShareCollectionId) -> DomainResult<()> {
        self.storage.write().unwrap().remove(id);
        Ok(())
    }

    async fn exists(&self, id: &ShareCollectionId) -> DomainResult<bool> {
        Ok(self.storage.read().unwrap().contains_key(id))
    }

    async fn find_by_ids(&self, ids: &[ShareCollectionId]) -> DomainResult<Vec<ShareCollection>> {
        let storage = self.storage.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| storage.get(id).cloned())
            .collect())
    }
}

#[async_trait]
impl ShareCollectionRepository for MockShareCollectionRepository {
    async fn find_by_secret_id(
        &self,
        secret_id: &SecretId,
    ) -> DomainResult<Option<ShareCollection>> {
        let storage = self.storage.read().unwrap();
        Ok(storage
            .values()
            .find(|sc| sc.secret_id() == secret_id)
            .cloned())
    }
}

fn create_test_share_collection(secret_id: SecretId) -> ShareCollection {
    let shares = vec![
        EncryptedShareData::new(1, vec![1, 2, 3]),
        EncryptedShareData::new(2, vec![4, 5, 6]),
        EncryptedShareData::new(3, vec![7, 8, 9]),
    ];
    ShareCollection::new(secret_id, 2, 3, shares).unwrap()
}

const fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn share_collection_repository_is_send_sync() {
    assert_send_sync::<MockShareCollectionRepository>();
}

#[tokio::test]
async fn test_share_collection_not_found() {
    let repo = MockShareCollectionRepository::new();
    let id = ShareCollectionId::generate();
    let result = repo.find_by_id(&id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_by_secret_id() {
    let repo = MockShareCollectionRepository::new();
    let secret_id = SecretId::generate();
    let collection = create_test_share_collection(secret_id.clone());

    repo.save(&collection).await.unwrap();

    let found = repo.find_by_secret_id(&secret_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().secret_id(), &secret_id);
}

#[tokio::test]
async fn test_find_by_secret_id_not_found() {
    let repo = MockShareCollectionRepository::new();
    let secret_id = SecretId::generate();
    let result = repo.find_by_secret_id(&secret_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_batch_find_share_collections() {
    let repo = MockShareCollectionRepository::new();
    let collections: Vec<_> = (0..3)
        .map(|_| create_test_share_collection(SecretId::generate()))
        .collect();

    for collection in &collections {
        repo.save(collection).await.unwrap();
    }

    let ids: Vec<_> = collections.iter().map(|c| c.id().clone()).collect();
    let found = repo.find_by_ids(&ids[0..2]).await.unwrap();

    assert_eq!(found.len(), 2);
}
