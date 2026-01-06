//! ArweaveShareCollectionRepository implementation
//!
//! Implements ShareCollectionRepository trait for Arweave-based persistence.

#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::adapter::errors::AdapterError;
use crate::domain::entities::share::{EncryptedShareData, ShareCollection};
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{SecretId, ShareCollectionId};
use crate::repositories::{Repository, ShareCollectionRepository};

use super::{ArweaveClient, Tag, tag_helpers, tag_names, tag_values};

/// Serializable representation of EncryptedShareData
#[derive(Serialize, Deserialize)]
struct StoredEncryptedShare {
    index: u8,
    data: Vec<u8>,
}

impl StoredEncryptedShare {
    fn from_entity(share: &EncryptedShareData) -> Self {
        Self {
            index: share.index(),
            data: share.encrypted_data().to_vec(),
        }
    }

    fn to_entity(&self) -> EncryptedShareData {
        EncryptedShareData::new(self.index, self.data.clone())
    }
}

/// Serializable representation of ShareCollection for Arweave storage
#[derive(Serialize, Deserialize)]
struct StoredShareCollection {
    id: String,
    secret_id: String,
    threshold_k: u8,
    threshold_n: u8,
    shares: Vec<StoredEncryptedShare>,
    arweave_tx_id: Option<String>,
    created_at: u64,
    deleted: bool,
}

impl StoredShareCollection {
    fn from_entity(collection: &ShareCollection) -> Self {
        Self {
            id: collection.id().as_str().to_string(),
            secret_id: collection.secret_id().as_str().to_string(),
            threshold_k: collection.threshold_k(),
            threshold_n: collection.threshold_n(),
            shares: collection
                .shares()
                .iter()
                .map(StoredEncryptedShare::from_entity)
                .collect(),
            arweave_tx_id: collection.arweave_tx_id().map(String::from),
            created_at: collection.created_at(),
            deleted: false,
        }
    }

    fn to_entity(&self) -> ShareCollection {
        let shares: Vec<EncryptedShareData> = self
            .shares
            .iter()
            .map(StoredEncryptedShare::to_entity)
            .collect();

        ShareCollection::from_stored(
            ShareCollectionId::new(self.id.clone()),
            SecretId::new(self.secret_id.clone()),
            self.threshold_k,
            self.threshold_n,
            shares,
            self.arweave_tx_id.clone(),
            self.created_at,
        )
    }
}

/// Arweave-based ShareCollectionRepository implementation
pub struct ArweaveShareCollectionRepository<C: ArweaveClient> {
    client: C,
}

impl<C: ArweaveClient> ArweaveShareCollectionRepository<C> {
    /// Create a new ArweaveShareCollectionRepository with the given client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn create_tags(&self, collection: &ShareCollection) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SHARE_COLLECTION),
            tag_helpers::entity_id_tag(collection.id().as_str()),
            tag_helpers::secret_id_tag(collection.secret_id().as_str()),
            tag_helpers::deleted_tag(false),
        ]
    }

    fn create_query_tags(&self, id: &ShareCollectionId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SHARE_COLLECTION),
            tag_helpers::entity_id_tag(id.as_str()),
        ]
    }

    fn create_secret_query_tags(&self, secret_id: &SecretId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SHARE_COLLECTION),
            Tag::new(tag_names::SECRET_ID, secret_id.as_str()),
        ]
    }

    async fn find_tx_id(&self, id: &ShareCollectionId) -> Result<Option<String>, AdapterError> {
        let tags = self.create_query_tags(id);
        let tx_ids = self.client.query(tags).await?;
        Ok(tx_ids.into_iter().next_back())
    }
}

#[async_trait]
impl<C: ArweaveClient> Repository<ShareCollection, ShareCollectionId>
    for ArweaveShareCollectionRepository<C>
{
    async fn save(&self, entity: &ShareCollection) -> DomainResult<()> {
        let stored = StoredShareCollection::from_entity(entity);
        let bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = self.create_tags(entity);
        self.client.post(&bytes, tags).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &ShareCollectionId) -> DomainResult<Option<ShareCollection>> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredShareCollection = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete(&self, id: &ShareCollectionId) -> DomainResult<()> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(()),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(()),
        };

        let mut stored: StoredShareCollection = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(());
        }

        stored.deleted = true;
        let new_bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SHARE_COLLECTION),
            tag_helpers::entity_id_tag(id.as_str()),
            tag_helpers::secret_id_tag(&stored.secret_id),
            tag_helpers::deleted_tag(true),
        ];

        self.client.post(&new_bytes, tags).await?;
        Ok(())
    }

    async fn exists(&self, id: &ShareCollectionId) -> DomainResult<bool> {
        match self.find_by_id(id).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn find_by_ids(&self, ids: &[ShareCollectionId]) -> DomainResult<Vec<ShareCollection>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(collection) = self.find_by_id(id).await? {
                results.push(collection);
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl<C: ArweaveClient> ShareCollectionRepository for ArweaveShareCollectionRepository<C> {
    async fn find_by_secret_id(
        &self,
        secret_id: &SecretId,
    ) -> DomainResult<Option<ShareCollection>> {
        let tags = self.create_secret_query_tags(secret_id);
        let tx_ids = self.client.query(tags).await?;

        let tx_id = match tx_ids.into_iter().next_back() {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredShareCollection = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::repository_impl::mock::MockArweaveClient;
    use crate::domain::entities::share::EncryptedShareData;
    use crate::repositories::Repository;

    fn create_test_share_collection(secret_id: SecretId) -> ShareCollection {
        let shares = vec![
            EncryptedShareData::new(1, vec![1u8; 32]),
            EncryptedShareData::new(2, vec![2u8; 32]),
            EncryptedShareData::new(3, vec![3u8; 32]),
        ];
        ShareCollection::new(secret_id, 2, 3, shares).unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let secret_id = SecretId::generate();
        let collection = create_test_share_collection(secret_id);
        let id = collection.id().clone();

        repo.save(&collection).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap();

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), &id);
        assert_eq!(found.threshold_k(), 2);
        assert_eq!(found.threshold_n(), 3);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let id = ShareCollectionId::generate();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_secret_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let secret_id = SecretId::generate();
        let collection = create_test_share_collection(secret_id.clone());

        repo.save(&collection).await.unwrap();
        let found = repo.find_by_secret_id(&secret_id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().secret_id(), &secret_id);
    }

    #[tokio::test]
    async fn test_find_by_secret_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let secret_id = SecretId::generate();

        let found = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let secret_id = SecretId::generate();
        let collection = create_test_share_collection(secret_id);
        let id = collection.id().clone();

        repo.save(&collection).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_exists() {
        let client = MockArweaveClient::new();
        let repo = ArweaveShareCollectionRepository::new(client);
        let secret_id = SecretId::generate();
        let collection = create_test_share_collection(secret_id);
        let id = collection.id().clone();

        assert!(!repo.exists(&id).await.unwrap());
        repo.save(&collection).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());
    }
}
