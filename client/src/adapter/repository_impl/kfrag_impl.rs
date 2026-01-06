//! ArweaveKFragRepository implementation
//!
//! Implements KFragRepository trait for Arweave-based persistence.
//! Handles sensitive cryptographic data with Zeroize for memory safety.

#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::adapter::errors::AdapterError;
use crate::domain::entities::KFrag;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{KFragId, SecretId};
use crate::repositories::{KFragRepository, Repository};

use super::{ArweaveClient, Tag, tag_helpers, tag_names, tag_values};

/// Serializable representation of KFrag for Arweave storage
#[derive(Serialize, Deserialize)]
struct StoredKFrag {
    id: String,
    secret_id: String,
    holder_index: u8,
    holder_process_id: Option<String>,
    kfrag_data: Vec<u8>,
    created_at: u64,
    deleted: bool,
}

impl StoredKFrag {
    fn from_entity(kfrag: &KFrag) -> Self {
        Self {
            id: kfrag.id().as_str().to_string(),
            secret_id: kfrag.secret_id().as_str().to_string(),
            holder_index: kfrag.holder_index(),
            holder_process_id: kfrag.holder_process_id().map(String::from),
            kfrag_data: kfrag.kfrag_data().to_vec(),
            created_at: kfrag.created_at(),
            deleted: false,
        }
    }

    fn to_entity(&self) -> KFrag {
        KFrag::from_stored(
            KFragId::new(self.id.clone()),
            SecretId::new(self.secret_id.clone()),
            self.holder_index,
            self.holder_process_id.clone(),
            self.kfrag_data.clone(),
            self.created_at,
        )
    }
}

impl Drop for StoredKFrag {
    fn drop(&mut self) {
        self.kfrag_data.zeroize();
    }
}

/// Arweave-based KFragRepository implementation
pub struct ArweaveKFragRepository<C: ArweaveClient> {
    client: C,
}

impl<C: ArweaveClient> ArweaveKFragRepository<C> {
    /// Create a new ArweaveKFragRepository with the given client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn create_tags(&self, kfrag: &KFrag) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            tag_helpers::entity_id_tag(kfrag.id().as_str()),
            tag_helpers::secret_id_tag(kfrag.secret_id().as_str()),
            tag_helpers::holder_index_tag(kfrag.holder_index()),
            tag_helpers::deleted_tag(false),
        ]
    }

    fn create_query_tags(&self, id: &KFragId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            tag_helpers::entity_id_tag(id.as_str()),
        ]
    }

    fn create_secret_query_tags(&self, secret_id: &SecretId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            Tag::new(tag_names::SECRET_ID, secret_id.as_str()),
        ]
    }

    fn create_holder_index_query_tags(&self, secret_id: &SecretId, holder_index: u8) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            Tag::new(tag_names::SECRET_ID, secret_id.as_str()),
            Tag::new(tag_names::HOLDER_INDEX, holder_index.to_string()),
        ]
    }

    async fn find_tx_id(&self, id: &KFragId) -> Result<Option<String>, AdapterError> {
        let tags = self.create_query_tags(id);
        let tx_ids = self.client.query(tags).await?;
        Ok(tx_ids.into_iter().next_back())
    }
}

#[async_trait]
impl<C: ArweaveClient> Repository<KFrag, KFragId> for ArweaveKFragRepository<C> {
    async fn save(&self, entity: &KFrag) -> DomainResult<()> {
        let stored = StoredKFrag::from_entity(entity);
        let bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = self.create_tags(entity);
        self.client.post(&bytes, tags).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &KFragId) -> DomainResult<Option<KFrag>> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredKFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete(&self, id: &KFragId) -> DomainResult<()> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(()),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(()),
        };

        let mut stored: StoredKFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(());
        }

        stored.deleted = true;
        stored.kfrag_data.zeroize();

        let new_bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        // Include SECRET_ID tag for find_by_secret_id to work correctly with soft-delete
        let tags = vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            tag_helpers::entity_id_tag(id.as_str()),
            tag_helpers::secret_id_tag(&stored.secret_id),
            tag_helpers::deleted_tag(true),
        ];

        self.client.post(&new_bytes, tags).await?;
        Ok(())
    }

    async fn exists(&self, id: &KFragId) -> DomainResult<bool> {
        match self.find_by_id(id).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn find_by_ids(&self, ids: &[KFragId]) -> DomainResult<Vec<KFrag>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(kfrag) = self.find_by_id(id).await? {
                results.push(kfrag);
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl<C: ArweaveClient> KFragRepository for ArweaveKFragRepository<C> {
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<KFrag>> {
        use std::collections::HashMap;

        let tags = self.create_secret_query_tags(secret_id);
        let tx_ids = self.client.query(tags).await?;

        // Group entries by entity_id, keeping latest (last tx_id) for each
        let mut latest_by_id: HashMap<String, (String, StoredKFrag)> = HashMap::new();

        for tx_id in tx_ids {
            let bytes = match self.client.get(&tx_id).await? {
                Some(bytes) => bytes,
                None => continue,
            };

            let stored: StoredKFrag = match bincode::deserialize(&bytes) {
                Ok(stored) => stored,
                Err(_) => continue,
            };

            // Always overwrite - tx_ids are sorted so later entries are newer
            latest_by_id.insert(stored.id.clone(), (tx_id, stored));
        }

        // Filter out deleted entries and convert to entities
        let results: Vec<KFrag> = latest_by_id
            .into_values()
            .filter(|(_, stored)| !stored.deleted)
            .map(|(_, stored)| stored.to_entity())
            .collect();

        Ok(results)
    }

    async fn find_by_holder_index(
        &self,
        secret_id: &SecretId,
        holder_index: u8,
    ) -> DomainResult<Option<KFrag>> {
        let tags = self.create_holder_index_query_tags(secret_id, holder_index);
        let tx_ids = self.client.query(tags).await?;

        let tx_id = match tx_ids.into_iter().next_back() {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredKFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()> {
        let kfrags = self.find_by_secret_id(secret_id).await?;

        for kfrag in kfrags {
            self.delete(kfrag.id()).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::repository_impl::mock::MockArweaveClient;
    use crate::repositories::Repository;

    fn create_test_kfrag(secret_id: SecretId, holder_index: u8) -> KFrag {
        KFrag::new(secret_id, holder_index, 5, vec![1u8; 100]).unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let secret_id = SecretId::generate();
        let kfrag = create_test_kfrag(secret_id, 1);
        let id = kfrag.id().clone();

        repo.save(&kfrag).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap();

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), &id);
        assert_eq!(found.holder_index(), 1);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let id = KFragId::generate();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_secret_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
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
    async fn test_find_by_holder_index() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let secret_id = SecretId::generate();

        let kfrag = create_test_kfrag(secret_id.clone(), 2);
        repo.save(&kfrag).await.unwrap();

        let found = repo.find_by_holder_index(&secret_id, 2).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().holder_index(), 2);

        let not_found = repo.find_by_holder_index(&secret_id, 99).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let secret_id = SecretId::generate();
        let kfrag = create_test_kfrag(secret_id, 1);
        let id = kfrag.id().clone();

        repo.save(&kfrag).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_by_secret_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let secret_id = SecretId::generate();
        let other_secret_id = SecretId::generate();

        let kfrags: Vec<_> = (1..=3)
            .map(|i| create_test_kfrag(secret_id.clone(), i))
            .collect();
        let other_kfrag = create_test_kfrag(other_secret_id.clone(), 1);

        for kfrag in &kfrags {
            repo.save(kfrag).await.unwrap();
        }
        repo.save(&other_kfrag).await.unwrap();

        repo.delete_by_secret_id(&secret_id).await.unwrap();

        let remaining = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(remaining.is_empty());

        let other_remaining = repo.find_by_secret_id(&other_secret_id).await.unwrap();
        assert_eq!(other_remaining.len(), 1);
    }

    #[tokio::test]
    async fn test_exists() {
        let client = MockArweaveClient::new();
        let repo = ArweaveKFragRepository::new(client);
        let secret_id = SecretId::generate();
        let kfrag = create_test_kfrag(secret_id, 1);
        let id = kfrag.id().clone();

        assert!(!repo.exists(&id).await.unwrap());
        repo.save(&kfrag).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());
    }
}
