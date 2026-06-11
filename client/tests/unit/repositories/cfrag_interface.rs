use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use formix::domain::{CFrag, CFragId, DomainResult, KFragId, SecretId};
use formix::repositories::{CFragRepository, Repository};

struct MockCFragRepository {
    storage: RwLock<HashMap<CFragId, CFrag>>,
}

impl MockCFragRepository {
    fn new() -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Repository<CFrag, CFragId> for MockCFragRepository {
    async fn save(&self, entity: &CFrag) -> DomainResult<()> {
        self.storage
            .write()
            .unwrap()
            .insert(entity.id().clone(), entity.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &CFragId) -> DomainResult<Option<CFrag>> {
        Ok(self.storage.read().unwrap().get(id).cloned())
    }

    async fn delete(&self, id: &CFragId) -> DomainResult<()> {
        self.storage.write().unwrap().remove(id);
        Ok(())
    }

    async fn exists(&self, id: &CFragId) -> DomainResult<bool> {
        Ok(self.storage.read().unwrap().contains_key(id))
    }

    async fn find_by_ids(&self, ids: &[CFragId]) -> DomainResult<Vec<CFrag>> {
        let storage = self.storage.read().unwrap();
        Ok(ids
            .iter()
            .filter_map(|id| storage.get(id).cloned())
            .collect())
    }
}

#[async_trait]
impl CFragRepository for MockCFragRepository {
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<CFrag>> {
        let storage = self.storage.read().unwrap();
        Ok(storage
            .values()
            .filter(|c| c.secret_id() == secret_id)
            .cloned()
            .collect())
    }

    async fn find_by_kfrag_id(&self, kfrag_id: &KFragId) -> DomainResult<Option<CFrag>> {
        let storage = self.storage.read().unwrap();
        Ok(storage.values().find(|c| c.kfrag_id() == kfrag_id).cloned())
    }

    async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()> {
        self.storage
            .write()
            .unwrap()
            .retain(|_, v| v.secret_id() != secret_id);
        Ok(())
    }

    async fn count_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<usize> {
        let storage = self.storage.read().unwrap();
        Ok(storage
            .values()
            .filter(|c| c.secret_id() == secret_id)
            .count())
    }
}

fn create_test_cfrag(secret_id: SecretId, kfrag_id: KFragId, holder_index: u8) -> CFrag {
    CFrag::new(secret_id, kfrag_id, holder_index, vec![1, 2, 3, 4]).unwrap()
}

const fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn cfrag_repository_is_send_sync() {
    assert_send_sync::<MockCFragRepository>();
}

#[tokio::test]
async fn test_cfrag_not_found() {
    let repo = MockCFragRepository::new();
    let id = CFragId::generate();
    let result = repo.find_by_id(&id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_cfrags_by_secret_id() {
    let repo = MockCFragRepository::new();
    let secret_id = SecretId::generate();
    let cfrags: Vec<_> = (1..=3)
        .map(|i| create_test_cfrag(secret_id.clone(), KFragId::generate(), i))
        .collect();

    for cfrag in &cfrags {
        repo.save(cfrag).await.unwrap();
    }

    let found = repo.find_by_secret_id(&secret_id).await.unwrap();
    assert_eq!(found.len(), 3);
}

#[tokio::test]
async fn test_find_cfrag_by_kfrag_id() {
    let repo = MockCFragRepository::new();
    let secret_id = SecretId::generate();
    let kfrag_id = KFragId::generate();
    let cfrag = create_test_cfrag(secret_id, kfrag_id.clone(), 1);

    repo.save(&cfrag).await.unwrap();

    let found = repo.find_by_kfrag_id(&kfrag_id).await.unwrap();
    assert!(found.is_some());
    assert_eq!(found.unwrap().kfrag_id(), &kfrag_id);
}

#[tokio::test]
async fn test_find_cfrag_by_kfrag_id_not_found() {
    let repo = MockCFragRepository::new();
    let kfrag_id = KFragId::generate();
    let result = repo.find_by_kfrag_id(&kfrag_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_cfrags_by_secret_id() {
    let repo = MockCFragRepository::new();
    let secret_id = SecretId::generate();
    let other_secret_id = SecretId::generate();

    let cfrag1 = create_test_cfrag(secret_id.clone(), KFragId::generate(), 1);
    let cfrag2 = create_test_cfrag(secret_id.clone(), KFragId::generate(), 2);
    let cfrag3 = create_test_cfrag(other_secret_id.clone(), KFragId::generate(), 1);

    repo.save(&cfrag1).await.unwrap();
    repo.save(&cfrag2).await.unwrap();
    repo.save(&cfrag3).await.unwrap();

    repo.delete_by_secret_id(&secret_id).await.unwrap();

    let remaining = repo.find_by_secret_id(&secret_id).await.unwrap();
    assert!(remaining.is_empty());

    let other_remaining = repo.find_by_secret_id(&other_secret_id).await.unwrap();
    assert_eq!(other_remaining.len(), 1);
}

#[tokio::test]
async fn test_count_cfrags_by_secret_id() {
    let repo = MockCFragRepository::new();
    let secret_id = SecretId::generate();

    assert_eq!(repo.count_by_secret_id(&secret_id).await.unwrap(), 0);

    let cfrag1 = create_test_cfrag(secret_id.clone(), KFragId::generate(), 1);
    let cfrag2 = create_test_cfrag(secret_id.clone(), KFragId::generate(), 2);
    repo.save(&cfrag1).await.unwrap();
    repo.save(&cfrag2).await.unwrap();

    assert_eq!(repo.count_by_secret_id(&secret_id).await.unwrap(), 2);
}

#[tokio::test]
async fn test_batch_find_cfrags() {
    let repo = MockCFragRepository::new();
    let secret_id = SecretId::generate();
    let cfrags: Vec<_> = (1..=3)
        .map(|i| create_test_cfrag(secret_id.clone(), KFragId::generate(), i))
        .collect();

    for cfrag in &cfrags {
        repo.save(cfrag).await.unwrap();
    }

    let ids: Vec<_> = cfrags.iter().map(|c| c.id().clone()).collect();
    let found = repo.find_by_ids(&ids[0..2]).await.unwrap();

    assert_eq!(found.len(), 2);
}
