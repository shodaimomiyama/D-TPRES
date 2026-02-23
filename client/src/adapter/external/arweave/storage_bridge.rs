//! Bridge between ArweaveClient (adapter) and ArweaveStorageService (usecase)
//!
//! Adapts the production ArweaveClientImpl to the ArweaveStorageService trait,
//! converting between adapter-layer and usecase-layer types.

use std::sync::Arc;

use async_trait::async_trait;

use crate::adapter::repository_impl::{ArweaveClient, Tag as AdapterTag};
use crate::service::error::ServiceError;
use crate::usecase::core::storage::{
    ArweaveStorageService, ArweaveTransaction, BatchResult, QueryParams, SortBy, SortOrder, Tag,
    TransactionStatus,
};

/// Bridge implementing ArweaveStorageService by delegating to ArweaveClient
pub struct ProductionArweaveStorageService<C: ArweaveClient> {
    client: Arc<C>,
}

impl<C: ArweaveClient> ProductionArweaveStorageService<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self { client }
    }
}

fn to_adapter_tag(tag: &Tag) -> AdapterTag {
    AdapterTag::new(&tag.name, &tag.value)
}

fn to_adapter_tags(tags: &[Tag]) -> Vec<AdapterTag> {
    tags.iter().map(to_adapter_tag).collect()
}

fn map_adapter_error(err: crate::adapter::errors::AdapterError) -> ServiceError {
    match err {
        crate::adapter::errors::AdapterError::NotFound { entity_type, id } => {
            ServiceError::not_found(format!("{entity_type}: {id}"))
        }
        other => ServiceError::storage_error(other.to_string()),
    }
}

#[async_trait]
impl<C: ArweaveClient> ArweaveStorageService for ProductionArweaveStorageService<C> {
    async fn store_data(
        &self,
        payload: &[u8],
        tags: Vec<Tag>,
    ) -> crate::service::error::ServiceResult<String> {
        if payload.is_empty() {
            return Err(ServiceError::validation_error("Data cannot be empty"));
        }

        self.client
            .post(payload, to_adapter_tags(&tags))
            .await
            .map_err(map_adapter_error)
    }

    async fn retrieve_data(
        &self,
        transaction_id: &str,
    ) -> crate::service::error::ServiceResult<ArweaveTransaction> {
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        let raw_bytes = self
            .client
            .get(transaction_id)
            .await
            .map_err(map_adapter_error)?
            .ok_or_else(|| {
                ServiceError::not_found(format!("Transaction {transaction_id} not found"))
            })?;

        Ok(ArweaveTransaction {
            id: transaction_id.to_string(),
            data: raw_bytes,
            tags: Vec::new(),
            timestamp: 0,
        })
    }

    async fn query_by_tags(
        &self,
        params: QueryParams,
    ) -> crate::service::error::ServiceResult<Vec<ArweaveTransaction>> {
        if params.tags.is_empty() {
            return Err(ServiceError::validation_error(
                "Query must include at least one tag",
            ));
        }

        let adapter_tags = to_adapter_tags(&params.tags);
        let tx_ids = self
            .client
            .query(adapter_tags)
            .await
            .map_err(map_adapter_error)?;

        let mut transactions = Vec::new();
        for tx_id in &tx_ids {
            if let Some(content) = self.client.get(tx_id).await.map_err(map_adapter_error)? {
                transactions.push(ArweaveTransaction {
                    id: tx_id.clone(),
                    data: content,
                    tags: Vec::new(),
                    timestamp: 0,
                });
            }
        }

        if let Some(sort_by) = params.sort_by {
            match sort_by {
                SortBy::Timestamp(order) => {
                    transactions.sort_by(|a, b| match order {
                        SortOrder::Ascending => a.timestamp.cmp(&b.timestamp),
                        SortOrder::Descending => b.timestamp.cmp(&a.timestamp),
                    });
                }
                SortBy::Id(order) => {
                    transactions.sort_by(|a, b| match order {
                        SortOrder::Ascending => a.id.cmp(&b.id),
                        SortOrder::Descending => b.id.cmp(&a.id),
                    });
                }
            }
        }

        if let Some(limit) = params.limit {
            transactions.truncate(limit);
        }

        Ok(transactions)
    }

    async fn check_transaction_status(
        &self,
        transaction_id: &str,
    ) -> crate::service::error::ServiceResult<TransactionStatus> {
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        match self
            .client
            .get(transaction_id)
            .await
            .map_err(map_adapter_error)?
        {
            Some(_) => Ok(TransactionStatus::Confirmed),
            None => Ok(TransactionStatus::Pending),
        }
    }

    async fn batch_store(
        &self,
        items: Vec<(Vec<u8>, Vec<Tag>)>,
    ) -> crate::service::error::ServiceResult<BatchResult> {
        if items.is_empty() {
            return Ok(BatchResult {
                successful: Vec::new(),
                failed: Vec::new(),
            });
        }

        let mut successful = Vec::new();
        let mut failed = Vec::new();

        for (index, (item_payload, tags)) in items.into_iter().enumerate() {
            match self.store_data(&item_payload, tags).await {
                Ok(tx_id) => successful.push(tx_id),
                Err(e) => failed.push((index.to_string(), e.to_string())),
            }
        }

        Ok(BatchResult { successful, failed })
    }

    async fn exists(&self, transaction_id: &str) -> crate::service::error::ServiceResult<bool> {
        if transaction_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Transaction ID cannot be empty",
            ));
        }

        self.client
            .get(transaction_id)
            .await
            .map(|opt| opt.is_some())
            .map_err(map_adapter_error)
    }

    async fn update_tags(
        &self,
        transaction_id: &str,
        new_tags: Vec<Tag>,
    ) -> crate::service::error::ServiceResult<String> {
        let existing = self.retrieve_data(transaction_id).await?;
        self.store_data(&existing.data, new_tags).await
    }
}
