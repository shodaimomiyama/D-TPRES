//! ArweaveCapsuleRepository implementation
//!
//! Implements CapsuleRepository trait for Arweave-based persistence.

#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::adapter::errors::AdapterError;
use crate::domain::entities::Capsule;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{CapsuleId, SecretId};
use crate::repositories::{CapsuleRepository, Repository};

use super::{ArweaveClient, Tag, tag_helpers, tag_names, tag_values};

/// Serializable representation of Capsule for Arweave storage
#[derive(Serialize, Deserialize)]
struct StoredCapsule {
    id: String,
    secret_id: String,
    capsule_data: Vec<u8>,
    owner_public_key: Vec<u8>,
    arweave_tx_id: Option<String>,
    created_at: u64,
    deleted: bool,
}

impl StoredCapsule {
    fn from_entity(capsule: &Capsule) -> Self {
        Self {
            id: capsule.id().as_str().to_string(),
            secret_id: capsule.secret_id().as_str().to_string(),
            capsule_data: capsule.capsule_data().to_vec(),
            owner_public_key: capsule.owner_public_key().to_vec(),
            arweave_tx_id: capsule.arweave_tx_id().map(String::from),
            created_at: capsule.created_at(),
            deleted: false,
        }
    }

    fn to_entity(&self) -> Capsule {
        Capsule::from_stored(
            CapsuleId::new(self.id.clone()),
            SecretId::new(self.secret_id.clone()),
            self.capsule_data.clone(),
            self.owner_public_key.clone(),
            self.arweave_tx_id.clone(),
            self.created_at,
        )
    }
}

/// Arweave-based CapsuleRepository implementation
pub struct ArweaveCapsuleRepository<C: ArweaveClient> {
    client: C,
}

impl<C: ArweaveClient> ArweaveCapsuleRepository<C> {
    /// Create a new ArweaveCapsuleRepository with the given client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn create_tags(&self, capsule: &Capsule) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CAPSULE),
            tag_helpers::entity_id_tag(capsule.id().as_str()),
            tag_helpers::secret_id_tag(capsule.secret_id().as_str()),
            tag_helpers::deleted_tag(false),
        ]
    }

    fn create_query_tags(&self, id: &CapsuleId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CAPSULE),
            tag_helpers::entity_id_tag(id.as_str()),
        ]
    }

    fn create_secret_query_tags(&self, secret_id: &SecretId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CAPSULE),
            Tag::new(tag_names::SECRET_ID, secret_id.as_str()),
        ]
    }

    async fn find_tx_id(&self, id: &CapsuleId) -> Result<Option<String>, AdapterError> {
        let tags = self.create_query_tags(id);
        let tx_ids = self.client.query(tags).await?;
        Ok(tx_ids.into_iter().next_back())
    }
}

#[async_trait]
impl<C: ArweaveClient> Repository<Capsule, CapsuleId> for ArweaveCapsuleRepository<C> {
    async fn save(&self, entity: &Capsule) -> DomainResult<()> {
        let stored = StoredCapsule::from_entity(entity);
        let bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = self.create_tags(entity);
        self.client.post(&bytes, tags).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &CapsuleId) -> DomainResult<Option<Capsule>> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredCapsule = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete(&self, id: &CapsuleId) -> DomainResult<()> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(()),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(()),
        };

        let mut stored: StoredCapsule = bincode::deserialize(&bytes).map_err(|e| {
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
            tag_helpers::entity_type_tag(tag_values::ENTITY_CAPSULE),
            tag_helpers::entity_id_tag(id.as_str()),
            tag_helpers::deleted_tag(true),
        ];

        self.client.post(&new_bytes, tags).await?;
        Ok(())
    }

    async fn exists(&self, id: &CapsuleId) -> DomainResult<bool> {
        match self.find_by_id(id).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn find_by_ids(&self, ids: &[CapsuleId]) -> DomainResult<Vec<Capsule>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(capsule) = self.find_by_id(id).await? {
                results.push(capsule);
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl<C: ArweaveClient> CapsuleRepository for ArweaveCapsuleRepository<C> {
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Option<Capsule>> {
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

        let stored: StoredCapsule = bincode::deserialize(&bytes).map_err(|e| {
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
    use crate::repositories::Repository;

    fn create_test_capsule(secret_id: SecretId) -> Capsule {
        Capsule::new(secret_id, vec![1u8; 100], vec![2u8; 32]).unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();
        let capsule = create_test_capsule(secret_id);
        let id = capsule.id().clone();

        repo.save(&capsule).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap();

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), &id);
        assert_eq!(found.capsule_data().len(), 100);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let id = CapsuleId::generate();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_by_secret_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();
        let capsule = create_test_capsule(secret_id.clone());

        repo.save(&capsule).await.unwrap();
        let found = repo.find_by_secret_id(&secret_id).await.unwrap();

        assert!(found.is_some());
        assert_eq!(found.unwrap().secret_id(), &secret_id);
    }

    #[tokio::test]
    async fn test_find_by_secret_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();

        let found = repo.find_by_secret_id(&secret_id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();
        let capsule = create_test_capsule(secret_id);
        let id = capsule.id().clone();

        repo.save(&capsule).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_exists() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();
        let capsule = create_test_capsule(secret_id);
        let id = capsule.id().clone();

        assert!(!repo.exists(&id).await.unwrap());
        repo.save(&capsule).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_binary_data_preserved() {
        let client = MockArweaveClient::new();
        let repo = ArweaveCapsuleRepository::new(client);
        let secret_id = SecretId::generate();
        let capsule_data = vec![0u8, 1, 2, 255, 128, 64];
        let owner_pk = vec![10u8; 32];
        let capsule = Capsule::new(secret_id, capsule_data.clone(), owner_pk.clone()).unwrap();
        let id = capsule.id().clone();

        repo.save(&capsule).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap().unwrap();

        assert_eq!(found.capsule_data(), capsule_data.as_slice());
        assert_eq!(found.owner_public_key(), owner_pk.as_slice());
    }
}
