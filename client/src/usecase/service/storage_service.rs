//! Service-layer StorageService
//!
//! Composes ArweaveStorageService (sync Arweave ops) and ContractStorage
//! (async AO contract ops) into a single service-layer interface.

use std::sync::Arc;

use async_trait::async_trait;

use crate::service::error::ServiceResult;
use crate::usecase::core::contract_storage::ContractStorage;
use crate::usecase::core::crypto::{CFragData, KeyFragment};
use crate::usecase::core::storage::{
    ArweaveStorageService, ArweaveTransaction, BatchResult, QueryParams, SortBy, SortOrder, Tag,
    TransactionStatus,
};

/// Service-layer StorageService trait combining Arweave and Contract operations
#[async_trait]
pub trait StorageService: Send + Sync {
    // Arweave operations (async, delegated to ArweaveStorageService)
    async fn store_data(&self, content: &[u8], tags: Vec<Tag>) -> ServiceResult<String>;
    async fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction>;
    async fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>>;
    async fn check_transaction_status(
        &self,
        transaction_id: &str,
    ) -> ServiceResult<TransactionStatus>;
    async fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult>;
    async fn exists(&self, transaction_id: &str) -> ServiceResult<bool>;
    async fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String>;

    // Contract operations (async, delegated to ContractStorage)
    async fn send_kfrags_to_contract(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()>;

    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()>;

    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>>;

    async fn retrieve_encrypted_shares(&self, secret_id: &str) -> ServiceResult<Vec<Vec<u8>>>;
}

/// StorageService implementation composing Arweave + Contract sub-services
pub struct StorageServiceImpl<S: ArweaveStorageService, CT: ContractStorage> {
    arweave: Arc<S>,
    contract: Arc<CT>,
}

impl<S: ArweaveStorageService, CT: ContractStorage> StorageServiceImpl<S, CT> {
    pub fn new(arweave: Arc<S>, contract: Arc<CT>) -> Self {
        Self { arweave, contract }
    }
}

#[async_trait]
impl<S: ArweaveStorageService, CT: ContractStorage> StorageService for StorageServiceImpl<S, CT> {
    async fn store_data(&self, content: &[u8], tags: Vec<Tag>) -> ServiceResult<String> {
        self.arweave.store_data(content, tags).await
    }

    async fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction> {
        self.arweave.retrieve_data(transaction_id).await
    }

    async fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>> {
        self.arweave.query_by_tags(params).await
    }

    async fn check_transaction_status(
        &self,
        transaction_id: &str,
    ) -> ServiceResult<TransactionStatus> {
        self.arweave.check_transaction_status(transaction_id).await
    }

    async fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult> {
        self.arweave.batch_store(items).await
    }

    async fn exists(&self, transaction_id: &str) -> ServiceResult<bool> {
        self.arweave.exists(transaction_id).await
    }

    async fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String> {
        self.arweave.update_tags(transaction_id, new_tags).await
    }

    async fn send_kfrags_to_contract(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
        secret_id: &str,
    ) -> ServiceResult<()> {
        self.contract
            .send_kfrags(kfrags, contract_id, secret_id)
            .await
    }

    async fn delegate_capsule(
        &self,
        capsule_data: &[u8],
        contract_id: &str,
        kfrag_id: &str,
        capsule_id: &str,
    ) -> ServiceResult<()> {
        self.contract
            .delegate_capsule(capsule_data, contract_id, kfrag_id, capsule_id)
            .await
    }

    async fn retrieve_cfrags(
        &self,
        secret_id: &str,
        total_shares: u8,
        capsule_id: &str,
        process_id: &str,
    ) -> ServiceResult<Vec<CFragData>> {
        self.contract
            .retrieve_cfrags(secret_id, total_shares, capsule_id, process_id)
            .await
    }

    async fn retrieve_encrypted_shares(&self, secret_id: &str) -> ServiceResult<Vec<Vec<u8>>> {
        let params = QueryParams {
            tags: vec![
                Tag {
                    name: "type".to_string(),
                    value: "encrypted_share".to_string(),
                },
                Tag {
                    name: "secret_id".to_string(),
                    value: secret_id.to_string(),
                },
            ],
            limit: None,
            sort_by: Some(SortBy::Id(SortOrder::Ascending)),
        };

        let transactions = self.arweave.query_by_tags(params).await?;
        Ok(transactions.into_iter().map(|tx| tx.data).collect())
    }
}
