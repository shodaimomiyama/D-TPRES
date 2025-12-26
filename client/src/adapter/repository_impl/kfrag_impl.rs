//! ArweaveKFragRepository implementation
//!
//! Implements KFragRepository trait for Arweave-based persistence.
//! Handles sensitive cryptographic data with Zeroize for memory safety.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::adapter::errors::AdapterError;
use crate::domain::entities::KFrag;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{KFragId, SecretId};
use crate::repositories::{KFragRepository, Repository};

use super::{tag_helpers, tag_names, tag_values, ArweaveClient, Tag};

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

        let tags = vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_KFRAG),
            tag_helpers::entity_id_tag(id.as_str()),
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
        let tags = self.create_secret_query_tags(secret_id);
        let tx_ids = self.client.query(tags).await?;

        let mut results = Vec::with_capacity(tx_ids.len());

        for tx_id in tx_ids {
            let bytes = match self.client.get(&tx_id).await? {
                Some(bytes) => bytes,
                None => continue,
            };

            let stored: StoredKFrag = match bincode::deserialize(&bytes) {
                Ok(stored) => stored,
                Err(_) => continue,
            };

            if stored.deleted {
                continue;
            }

            results.push(stored.to_entity());
        }

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
