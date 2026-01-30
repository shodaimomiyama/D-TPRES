//! ArweaveSecretRepository implementation
//!
//! Implements SecretRepository trait for Arweave-based persistence.

#![allow(clippy::unused_self)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::significant_drop_tightening)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::adapter::errors::AdapterError;
use crate::domain::entities::secret::{Secret, SecretState};
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{CapsuleId, KFragId, SecretId, ShareCollectionId};
use crate::repositories::{Repository, SecretRepository};

use super::{ArweaveClient, Tag, tag_helpers, tag_values};

/// Serializable representation of Secret for Arweave storage
#[derive(Serialize, Deserialize)]
struct StoredSecret {
    id: String,
    threshold_k: u8,
    threshold_n: u8,
    state: String,
    capsule_id: Option<String>,
    share_collection_id: Option<String>,
    kfrag_ids: Vec<String>,
    owner_public_key: Vec<u8>,
    requester_public_key: Option<Vec<u8>>,
    created_at: u64,
    deleted: bool,
}

impl StoredSecret {
    fn from_entity(secret: &Secret) -> Self {
        Self {
            id: secret.id().as_str().to_string(),
            threshold_k: secret.threshold_k(),
            threshold_n: secret.threshold_n(),
            state: secret.state().to_string(),
            capsule_id: secret.capsule_id().map(|id| id.as_str().to_string()),
            share_collection_id: secret
                .share_collection_id()
                .map(|id| id.as_str().to_string()),
            kfrag_ids: secret
                .kfrag_ids()
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
            owner_public_key: secret.owner_public_key().to_vec(),
            requester_public_key: secret.requester_public_key().map(<[u8]>::to_vec),
            created_at: secret.created_at(),
            deleted: false,
        }
    }

    fn to_entity(&self) -> Result<Secret, AdapterError> {
        let state = parse_secret_state(&self.state)?;
        let capsule_id = self.capsule_id.as_ref().map(|s| CapsuleId::new(s.clone()));
        let share_collection_id = self
            .share_collection_id
            .as_ref()
            .map(|s| ShareCollectionId::new(s.clone()));
        let kfrag_ids: Vec<KFragId> = self
            .kfrag_ids
            .iter()
            .map(|s| KFragId::new(s.clone()))
            .collect();

        Ok(Secret::from_stored(
            SecretId::new(self.id.clone()),
            self.threshold_k,
            self.threshold_n,
            state,
            capsule_id,
            share_collection_id,
            kfrag_ids,
            self.owner_public_key.clone(),
            self.requester_public_key.clone(),
            self.created_at,
        ))
    }
}

fn parse_secret_state(state: &str) -> Result<SecretState, AdapterError> {
    match state {
        "Initialized" => Ok(SecretState::Initialized),
        "Split" => Ok(SecretState::Split),
        "Distributed" => Ok(SecretState::Distributed),
        "Recovered" => Ok(SecretState::Recovered),
        _ => Err(AdapterError::serialization_error(
            "deserialize_state",
            "Invalid secret state value",
        )),
    }
}

/// Arweave-based SecretRepository implementation
pub struct ArweaveSecretRepository<C: ArweaveClient> {
    client: C,
}

impl<C: ArweaveClient> ArweaveSecretRepository<C> {
    /// Create a new ArweaveSecretRepository with the given client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn create_tags(&self, secret: &Secret) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SECRET),
            tag_helpers::entity_id_tag(secret.id().as_str()),
            tag_helpers::deleted_tag(false),
        ]
    }

    fn create_query_tags(&self, id: &SecretId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_SECRET),
            tag_helpers::entity_id_tag(id.as_str()),
        ]
    }

    async fn find_tx_id(&self, id: &SecretId) -> Result<Option<String>, AdapterError> {
        let tags = self.create_query_tags(id);
        let tx_ids = self.client.query(tags).await?;
        Ok(tx_ids.into_iter().next_back())
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C: ArweaveClient> Repository<Secret, SecretId> for ArweaveSecretRepository<C> {
    async fn save(&self, entity: &Secret) -> DomainResult<()> {
        let stored = StoredSecret::from_entity(entity);
        let bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = self.create_tags(entity);
        self.client.post(&bytes, tags).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &SecretId) -> DomainResult<Option<Secret>> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredSecret = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        let entity = stored.to_entity()?;
        Ok(Some(entity))
    }

    async fn delete(&self, id: &SecretId) -> DomainResult<()> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(()),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(()),
        };

        let mut stored: StoredSecret = bincode::deserialize(&bytes).map_err(|e| {
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
            tag_helpers::entity_type_tag(tag_values::ENTITY_SECRET),
            tag_helpers::entity_id_tag(id.as_str()),
            tag_helpers::deleted_tag(true),
        ];

        self.client.post(&new_bytes, tags).await?;
        Ok(())
    }

    async fn exists(&self, id: &SecretId) -> DomainResult<bool> {
        match self.find_by_id(id).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn find_by_ids(&self, ids: &[SecretId]) -> DomainResult<Vec<Secret>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(secret) = self.find_by_id(id).await? {
                results.push(secret);
            }
        }

        Ok(results)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C: ArweaveClient> SecretRepository for ArweaveSecretRepository<C> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::repository_impl::mock::MockArweaveClient;
    use crate::repositories::Repository;

    fn create_test_secret() -> Secret {
        Secret::new(3, 5, vec![1u8; 32]).unwrap()
    }

    #[tokio::test]
    async fn test_save_and_find_by_id() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);
        let secret = create_test_secret();
        let id = secret.id().clone();

        repo.save(&secret).await.unwrap();
        let found = repo.find_by_id(&id).await.unwrap();

        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.id(), &id);
        assert_eq!(found.threshold_k(), 3);
        assert_eq!(found.threshold_n(), 5);
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);
        let id = SecretId::generate();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_exists() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);
        let secret = create_test_secret();
        let id = secret.id().clone();

        assert!(!repo.exists(&id).await.unwrap());
        repo.save(&secret).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);
        let secret = create_test_secret();
        let id = secret.id().clone();

        repo.save(&secret).await.unwrap();
        assert!(repo.exists(&id).await.unwrap());

        repo.delete(&id).await.unwrap();
        assert!(!repo.exists(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_nonexistent() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);
        let id = SecretId::generate();

        let result = repo.delete(&id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_by_ids() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);

        let secrets: Vec<_> = (0..3).map(|_| create_test_secret()).collect();
        let ids: Vec<_> = secrets.iter().map(|s| s.id().clone()).collect();

        for secret in &secrets {
            repo.save(secret).await.unwrap();
        }

        let found = repo.find_by_ids(&ids[0..2]).await.unwrap();
        assert_eq!(found.len(), 2);
    }

    #[tokio::test]
    async fn test_find_by_ids_partial() {
        let client = MockArweaveClient::new();
        let repo = ArweaveSecretRepository::new(client);

        let secret = create_test_secret();
        let existing_id = secret.id().clone();
        let nonexistent_id = SecretId::generate();

        repo.save(&secret).await.unwrap();

        let found = repo
            .find_by_ids(&[existing_id, nonexistent_id])
            .await
            .unwrap();
        assert_eq!(found.len(), 1);
    }
}
