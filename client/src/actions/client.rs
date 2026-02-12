//! DTpresClient - Main entry point for the D-TPRES client library
//!
//! Wraps the internal ActionsContainer and provides a clean public API
//! with builder-pattern share/recover operations.

use std::sync::Arc;

use crate::actions::builder::{NotSet, RecoverBuilder, ShareBuilder};
use crate::actions::di::DefaultActionsContainer;
use crate::actions::error::{ActionError, ActionResult};
use crate::usecase::core::crypto::{CryptoServiceImpl, PublicKey, SecretKey};
use crate::usecase::core::storage::ArweaveStorageServiceImpl;

/// Client initialization configuration
pub struct InitConfig {
    /// Path to JWK wallet file
    pub wallet_path: String,
    /// AO gateway URL (default: "https://ao.arweave.net")
    pub ao_gateway_url: Option<String>,
    /// Arweave gateway URL (default: "https://arweave.net")
    pub arweave_gateway_url: Option<String>,
}

/// Main D-TPRES client providing builder-based share/recover API.
///
/// Designed for single-user (self-service) workflows where the caller
/// acts as both data owner and requester within one AO process.
/// A single `process_id` is used for all operations; separate
/// `owner_process_id` / `requester_process_id` are not needed.
pub struct DTpresClient {
    process_id: String,
    wallet_address: String,
    ao_gateway_url: String,
    arweave_gateway_url: String,
    actions: Arc<DefaultActionsContainer>,
}

impl DTpresClient {
    /// Initialize a new DTpresClient
    ///
    /// Loads the JWK wallet, detects or spawns an AO Process,
    /// and returns a configured client instance.
    ///
    /// # Errors
    /// Currently returns `ActionError::WorkflowFailed` because JWK wallet
    /// loading and AO process detection are not yet implemented.
    /// See: <https://github.com/shodaimomiyama/D-TPRES/issues/60>
    pub fn init(_config: InitConfig) -> ActionResult<Self> {
        Err(ActionError::workflow_failed(
            "DTpresClient::init is not yet implemented: \
             JWK wallet loading and AO process spawning require Issue #60",
        ))
    }

    /// Create a DTpresClient with pre-configured values.
    ///
    /// Intended for testing and scenarios where wallet/process setup
    /// is handled externally.
    pub fn new(
        process_id: String,
        wallet_address: String,
        ao_gateway_url: String,
        arweave_gateway_url: String,
    ) -> Self {
        // TODO(#51/#52): Pass gateway URLs to ActionsContainer
        // when StorageService/AOClient DI integration is implemented
        let actions = Arc::new(DefaultActionsContainer::new());

        Self {
            process_id,
            wallet_address,
            ao_gateway_url,
            arweave_gateway_url,
            actions,
        }
    }

    /// Create a DTpresClient with a pre-configured storage service
    pub fn with_storage(
        process_id: String,
        wallet_address: String,
        ao_gateway_url: String,
        arweave_gateway_url: String,
        storage: Arc<ArweaveStorageServiceImpl>,
    ) -> Self {
        let actions = Arc::new(DefaultActionsContainer::with_storage(storage));

        Self {
            process_id,
            wallet_address,
            ao_gateway_url,
            arweave_gateway_url,
            actions,
        }
    }

    pub fn process_id(&self) -> &str {
        &self.process_id
    }

    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }

    pub fn ao_gateway_url(&self) -> &str {
        &self.ao_gateway_url
    }

    pub fn arweave_gateway_url(&self) -> &str {
        &self.arweave_gateway_url
    }

    /// Create a ShareBuilder for the share operation
    pub fn share(
        &self,
    ) -> ShareBuilder<
        CryptoServiceImpl,
        ArweaveStorageServiceImpl,
        NotSet,
        NotSet,
        NotSet,
        NotSet,
        NotSet,
    > {
        ShareBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Create a RecoverBuilder for the recover operation
    pub fn recover(
        &self,
    ) -> RecoverBuilder<CryptoServiceImpl, ArweaveStorageServiceImpl, NotSet, NotSet> {
        RecoverBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Generate a PRE key pair
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.actions.generate_keypair()
    }
}
