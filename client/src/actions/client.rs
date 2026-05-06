//! FormixClient - Main entry point for the FORMIX client library
//!
//! Wraps the internal ActionsContainer and provides a clean public API
//! with builder-pattern share/recover operations.

use std::sync::Arc;

use crate::actions::builder::{NotSet, RecoverBuilder, ShareBuilder};
use crate::actions::di::{DefaultActionsContainer, DefaultStorageService};
use crate::actions::error::{ActionError, ActionResult};
use crate::usecase::core::crypto::{
    CryptoServiceImpl as CoreCryptoServiceImpl, PublicKey, SecretKey,
};

/// Client initialization configuration
pub struct InitConfig {
    /// Path to JWK wallet file
    pub wallet_path: String,
    /// AO gateway URL (default: "https://ao.arweave.net")
    pub ao_gateway_url: Option<String>,
    /// Arweave gateway URL (default: "https://arweave.net")
    pub arweave_gateway_url: Option<String>,
}

/// Main FORMIX client providing builder-based share/recover API.
///
/// Designed for single-user (self-service) workflows where the caller
/// acts as both data owner and requester within one AO process.
/// A single `process_id` is used for all operations; separate
/// `owner_process_id` / `requester_process_id` are not needed.
pub struct FormixClient {
    process_id: String,
    wallet_address: String,
    ao_gateway_url: String,
    arweave_gateway_url: String,
    actions: Arc<DefaultActionsContainer>,
}

impl FormixClient {
    /// Initialize a new FormixClient
    ///
    /// Loads the JWK wallet, detects or spawns an AO Process,
    /// and returns a configured client instance.
    ///
    /// # Errors
    /// Currently returns `ActionError::WorkflowFailed` because JWK wallet
    /// loading and AO process detection are not yet implemented.
    /// See: <https://github.com/shodaimomiyama/FORMIX/issues/60>
    pub fn init(_config: InitConfig) -> ActionResult<Self> {
        Err(ActionError::workflow_failed(
            "FormixClient::init is not yet implemented: \
             JWK wallet loading and AO process spawning require Issue #60",
        ))
    }

    /// Create a FormixClient with pre-configured values.
    ///
    /// Intended for testing and scenarios where wallet/process setup
    /// is handled externally.
    ///
    /// # Note: Gateway URLs are currently ignored
    ///
    /// `ao_gateway_url` and `arweave_gateway_url` are stored on the struct but are
    /// **not yet wired into `ActionsContainer`**.  All network I/O uses the default
    /// (in-memory / mock) implementations until Issue #51 / #52 are resolved.
    ///
    /// See: <https://github.com/shodaimomiyama/FORMIX/issues/51>
    pub fn new(
        process_id: String,
        wallet_address: String,
        ao_gateway_url: String,
        arweave_gateway_url: String,
    ) -> Self {
        // TODO(#51/#52): Pass ao_gateway_url / arweave_gateway_url to ActionsContainer
        // so that StorageService and AOClient are configured to use the caller-provided URLs.
        let actions = Arc::new(DefaultActionsContainer::new());

        Self {
            process_id,
            wallet_address,
            ao_gateway_url,
            arweave_gateway_url,
            actions,
        }
    }

    /// Create a FormixClient with pre-configured storage components
    pub fn with_storage(
        process_id: String,
        wallet_address: String,
        ao_gateway_url: String,
        arweave_gateway_url: String,
        arweave: Arc<crate::usecase::core::storage::ArweaveStorageServiceImpl>,
        contract: Arc<
            crate::usecase::core::contract_storage::ContractStorageImpl<
                crate::adapter::external::mock_ao::MockAOClient,
            >,
        >,
    ) -> Self {
        let actions = Arc::new(DefaultActionsContainer::with_storage(arweave, contract));

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
        CoreCryptoServiceImpl,
        DefaultStorageService,
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
    ) -> RecoverBuilder<CoreCryptoServiceImpl, DefaultStorageService, NotSet, NotSet, NotSet> {
        RecoverBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Generate a PRE key pair
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.actions.generate_keypair()
    }
}

