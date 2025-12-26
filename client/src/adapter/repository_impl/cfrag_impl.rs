//! ArweaveCFragRepository implementation
//!
//! Implements CFragRepository trait for Arweave-based persistence.
//! Handles sensitive cryptographic data with Zeroize for memory safety.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::adapter::errors::AdapterError;
use crate::domain::entities::CFrag;
use crate::domain::errors::DomainResult;
use crate::domain::value_objects::{CFragId, KFragId, SecretId};
use crate::repositories::{CFragRepository, Repository};

use super::{tag_helpers, tag_names, tag_values, ArweaveClient, Tag};

/// Serializable representation of CFrag for Arweave storage
#[derive(Serialize, Deserialize)]
struct StoredCFrag {
    id: String,
    secret_id: String,
    kfrag_id: String,
    holder_index: u8,
    cfrag_data: Vec<u8>,
    created_at: u64,
    deleted: bool,
}

impl StoredCFrag {
    fn from_entity(cfrag: &CFrag) -> Self {
        Self {
            id: cfrag.id().as_str().to_string(),
            secret_id: cfrag.secret_id().as_str().to_string(),
            kfrag_id: cfrag.kfrag_id().as_str().to_string(),
            holder_index: cfrag.holder_index(),
            cfrag_data: cfrag.cfrag_data().to_vec(),
            created_at: cfrag.created_at(),
            deleted: false,
        }
    }

    fn to_entity(&self) -> CFrag {
        CFrag::from_stored(
            CFragId::new(self.id.clone()),
            SecretId::new(self.secret_id.clone()),
            KFragId::new(self.kfrag_id.clone()),
            self.holder_index,
            self.cfrag_data.clone(),
            self.created_at,
        )
    }
}

impl Drop for StoredCFrag {
    fn drop(&mut self) {
        self.cfrag_data.zeroize();
    }
}

/// Arweave-based CFragRepository implementation
pub struct ArweaveCFragRepository<C: ArweaveClient> {
    client: C,
}

impl<C: ArweaveClient> ArweaveCFragRepository<C> {
    /// Create a new ArweaveCFragRepository with the given client
    pub fn new(client: C) -> Self {
        Self { client }
    }

    fn create_tags(&self, cfrag: &CFrag) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CFRAG),
            tag_helpers::entity_id_tag(cfrag.id().as_str()),
            tag_helpers::secret_id_tag(cfrag.secret_id().as_str()),
            tag_helpers::kfrag_id_tag(cfrag.kfrag_id().as_str()),
            tag_helpers::holder_index_tag(cfrag.holder_index()),
            tag_helpers::deleted_tag(false),
        ]
    }

    fn create_query_tags(&self, id: &CFragId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CFRAG),
            tag_helpers::entity_id_tag(id.as_str()),
        ]
    }

    fn create_secret_query_tags(&self, secret_id: &SecretId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CFRAG),
            Tag::new(tag_names::SECRET_ID, secret_id.as_str()),
        ]
    }

    fn create_kfrag_query_tags(&self, kfrag_id: &KFragId) -> Vec<Tag> {
        vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CFRAG),
            Tag::new(tag_names::KFRAG_ID, kfrag_id.as_str()),
        ]
    }

    async fn find_tx_id(&self, id: &CFragId) -> Result<Option<String>, AdapterError> {
        let tags = self.create_query_tags(id);
        let tx_ids = self.client.query(tags).await?;
        Ok(tx_ids.into_iter().next_back())
    }
}

#[async_trait]
impl<C: ArweaveClient> Repository<CFrag, CFragId> for ArweaveCFragRepository<C> {
    async fn save(&self, entity: &CFrag) -> DomainResult<()> {
        let stored = StoredCFrag::from_entity(entity);
        let bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = self.create_tags(entity);
        self.client.post(&bytes, tags).await?;

        Ok(())
    }

    async fn find_by_id(&self, id: &CFragId) -> DomainResult<Option<CFrag>> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredCFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete(&self, id: &CFragId) -> DomainResult<()> {
        let tx_id = match self.find_tx_id(id).await? {
            Some(tx_id) => tx_id,
            None => return Ok(()),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(()),
        };

        let mut stored: StoredCFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(());
        }

        stored.deleted = true;
        stored.cfrag_data.zeroize();

        let new_bytes = bincode::serialize(&stored).map_err(|e| {
            AdapterError::serialization_error("serialize", &format!("Failed to serialize: {e}"))
        })?;

        let tags = vec![
            tag_helpers::app_tag(),
            tag_helpers::entity_type_tag(tag_values::ENTITY_CFRAG),
            tag_helpers::entity_id_tag(id.as_str()),
            tag_helpers::deleted_tag(true),
        ];

        self.client.post(&new_bytes, tags).await?;
        Ok(())
    }

    async fn exists(&self, id: &CFragId) -> DomainResult<bool> {
        match self.find_by_id(id).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn find_by_ids(&self, ids: &[CFragId]) -> DomainResult<Vec<CFrag>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(cfrag) = self.find_by_id(id).await? {
                results.push(cfrag);
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl<C: ArweaveClient> CFragRepository for ArweaveCFragRepository<C> {
    async fn find_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<Vec<CFrag>> {
        let tags = self.create_secret_query_tags(secret_id);
        let tx_ids = self.client.query(tags).await?;

        let mut results = Vec::with_capacity(tx_ids.len());

        for tx_id in tx_ids {
            let bytes = match self.client.get(&tx_id).await? {
                Some(bytes) => bytes,
                None => continue,
            };

            let stored: StoredCFrag = match bincode::deserialize(&bytes) {
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

    async fn find_by_kfrag_id(&self, kfrag_id: &KFragId) -> DomainResult<Option<CFrag>> {
        let tags = self.create_kfrag_query_tags(kfrag_id);
        let tx_ids = self.client.query(tags).await?;

        let tx_id = match tx_ids.into_iter().next_back() {
            Some(tx_id) => tx_id,
            None => return Ok(None),
        };

        let bytes = match self.client.get(&tx_id).await? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        let stored: StoredCFrag = bincode::deserialize(&bytes).map_err(|e| {
            AdapterError::serialization_error("deserialize", &format!("Failed to deserialize: {e}"))
        })?;

        if stored.deleted {
            return Ok(None);
        }

        Ok(Some(stored.to_entity()))
    }

    async fn delete_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<()> {
        let cfrags = self.find_by_secret_id(secret_id).await?;

        for cfrag in cfrags {
            self.delete(cfrag.id()).await?;
        }

        Ok(())
    }

    async fn count_by_secret_id(&self, secret_id: &SecretId) -> DomainResult<usize> {
        let cfrags = self.find_by_secret_id(secret_id).await?;
        Ok(cfrags.len())
    }
}
