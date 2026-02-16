//! Service-layer StorageService
//!
//! Composes ArweaveStorageService (sync Arweave ops) and ContractStorage
//! (async AO contract ops) into a single service-layer interface.

use std::sync::Arc;

use async_trait::async_trait;

use crate::service::error::ServiceResult;
use crate::usecase::core::contract_storage::ContractStorage;
use crate::usecase::core::crypto::KeyFragment;
use crate::usecase::core::storage::{
    ArweaveStorageService, ArweaveTransaction, BatchResult, QueryParams, Tag, TransactionStatus,
};

/// Service-layer StorageService trait combining Arweave and Contract operations
#[async_trait]
pub trait StorageService: Send + Sync {
    // Arweave operations (sync, delegated to ArweaveStorageService)
    fn store_data(&self, content: &[u8], tags: Vec<Tag>) -> ServiceResult<String>;
    fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction>;
    fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>>;
    fn check_transaction_status(&self, transaction_id: &str) -> ServiceResult<TransactionStatus>;
    fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult>;
    fn exists(&self, transaction_id: &str) -> ServiceResult<bool>;
    fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String>;

    // Contract operations (async, delegated to ContractStorage)
    async fn send_kfrags_to_contract(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
    ) -> ServiceResult<()>;

    async fn delegate_capsule(&self, capsule_data: &[u8], contract_id: &str) -> ServiceResult<()>;

    async fn retrieve_cfrags(
        &self,
        capsule_id: &str,
        contract_id: &str,
    ) -> ServiceResult<Vec<Vec<u8>>>;

    async fn retrieve_threshold(&self, contract_id: &str) -> ServiceResult<u8>;
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
    fn store_data(&self, content: &[u8], tags: Vec<Tag>) -> ServiceResult<String> {
        self.arweave.store_data(content, tags)
    }

    fn retrieve_data(&self, transaction_id: &str) -> ServiceResult<ArweaveTransaction> {
        self.arweave.retrieve_data(transaction_id)
    }

    fn query_by_tags(&self, params: QueryParams) -> ServiceResult<Vec<ArweaveTransaction>> {
        self.arweave.query_by_tags(params)
    }

    fn check_transaction_status(&self, transaction_id: &str) -> ServiceResult<TransactionStatus> {
        self.arweave.check_transaction_status(transaction_id)
    }

    fn batch_store(&self, items: Vec<(Vec<u8>, Vec<Tag>)>) -> ServiceResult<BatchResult> {
        self.arweave.batch_store(items)
    }

    fn exists(&self, transaction_id: &str) -> ServiceResult<bool> {
        self.arweave.exists(transaction_id)
    }

    fn update_tags(&self, transaction_id: &str, new_tags: Vec<Tag>) -> ServiceResult<String> {
        self.arweave.update_tags(transaction_id, new_tags)
    }

    async fn send_kfrags_to_contract(
        &self,
        kfrags: &[KeyFragment],
        contract_id: &str,
    ) -> ServiceResult<()> {
        self.contract.send_kfrags(kfrags, contract_id).await
    }

    async fn delegate_capsule(&self, capsule_data: &[u8], contract_id: &str) -> ServiceResult<()> {
        self.contract
            .delegate_capsule(capsule_data, contract_id)
            .await
    }

    async fn retrieve_cfrags(
        &self,
        capsule_id: &str,
        contract_id: &str,
    ) -> ServiceResult<Vec<Vec<u8>>> {
        self.contract.retrieve_cfrags(capsule_id, contract_id).await
    }

    async fn retrieve_threshold(&self, contract_id: &str) -> ServiceResult<u8> {
        self.contract.retrieve_threshold(contract_id).await
    }
}
