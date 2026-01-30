//! DTpresClient - Main entry point for the D-TPRES client library
//!
//! Wraps the internal ActionsContainer and provides a clean public API
//! with builder-pattern share/recover operations.

use std::sync::Arc;

use crate::actions::builder::{NotSet, RecoverBuilder, ShareBuilder};
use crate::actions::di::DefaultActionsContainer;
use crate::actions::error::{ActionError, ActionResult};
use crate::usecase::core::crypto::{CryptoServiceImpl, PublicKey, SecretKey};

/// Client initialization configuration
pub struct InitConfig {
    /// Path to JWK wallet file
    pub wallet_path: String,
    /// AO gateway URL (default: "https://ao.arweave.net")
    pub ao_gateway_url: Option<String>,
    /// Arweave gateway URL (default: "https://arweave.net")
    pub arweave_gateway_url: Option<String>,
}

const DEFAULT_AO_GATEWAY: &str = "https://ao.arweave.net";
const DEFAULT_ARWEAVE_GATEWAY: &str = "https://arweave.net";

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
    /// Returns `ActionError` if wallet loading or process setup fails.
    pub fn init(config: InitConfig) -> ActionResult<Self> {
        if !std::path::Path::new(&config.wallet_path).exists() {
            return Err(ActionError::validation_failed(
                "wallet_path",
                format!("wallet file not found: {}", config.wallet_path),
            ));
        }

        let ao_gateway_url = config
            .ao_gateway_url
            .unwrap_or_else(|| DEFAULT_AO_GATEWAY.to_string());
        let arweave_gateway_url = config
            .arweave_gateway_url
            .unwrap_or_else(|| DEFAULT_ARWEAVE_GATEWAY.to_string());

        // TODO: Load JWK wallet from config.wallet_path
        // TODO: Detect existing AO Process or spawn new one
        // For now, use placeholder values until AO integration is complete
        let wallet_address = format!("wallet_{}", &config.wallet_path);
        let process_id = format!("process_{}", &config.wallet_path);

        let actions = Arc::new(DefaultActionsContainer::new());

        Ok(Self {
            process_id,
            wallet_address,
            ao_gateway_url,
            arweave_gateway_url,
            actions,
        })
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
    pub fn share(&self) -> ShareBuilder<CryptoServiceImpl, NotSet, NotSet, NotSet, NotSet, NotSet> {
        ShareBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Create a RecoverBuilder for the recover operation
    pub fn recover(&self) -> RecoverBuilder<CryptoServiceImpl, NotSet, NotSet> {
        RecoverBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Generate a PRE key pair
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.actions.generate_keypair()
    }
}
