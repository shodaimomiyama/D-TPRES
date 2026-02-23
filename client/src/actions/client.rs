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

#[cfg(feature = "production-ao")]
#[derive(serde::Deserialize)]
pub struct DeployConfig {
    pub module_id: String,
    pub process_id: String,
    pub gateways: Option<GatewayConfig>,
}

#[cfg(feature = "production-ao")]
#[derive(serde::Deserialize)]
pub struct GatewayConfig {
    pub ao_mu: Option<String>,
    pub ao_cu: Option<String>,
    pub arweave: Option<String>,
}

#[cfg(feature = "production-ao")]
pub struct ProductionFormixClient {
    process_id: String,
    module_id: String,
    wallet_address: String,
    actions: Arc<ProductionActionsContainer>,
}

#[cfg(feature = "production-ao")]
use crate::actions::di::{ProductionActionsContainer, ProductionStorageService};
#[cfg(feature = "production-ao")]
use crate::adapter::external::ao::{AOConfig, ArweaveJWK, DataItemSigner, ProductionAOClient};

#[cfg(feature = "production-ao")]
impl ProductionFormixClient {
    /// Create a ProductionFormixClient from deploy JSON and wallet JWK files
    ///
    /// # Arguments
    /// * `deploy_path` - Path to deploy config JSON (contains module_id, process_id, gateways)
    /// * `wallet_path` - Path to Arweave JWK wallet file
    ///
    /// # Errors
    /// Returns `ActionError::WorkflowFailed` if file reading or parsing fails
    pub fn from_deploy_file(deploy_path: &str, wallet_path: &str) -> ActionResult<Self> {
        let deploy_json = std::fs::read_to_string(deploy_path).map_err(|e| {
            ActionError::workflow_failed(format!("Failed to read deploy config: {e}"))
        })?;
        let deploy: DeployConfig = serde_json::from_str(&deploy_json).map_err(|e| {
            ActionError::workflow_failed(format!("Failed to parse deploy config: {e}"))
        })?;

        let wallet_json = std::fs::read_to_string(wallet_path).map_err(|e| {
            ActionError::workflow_failed(format!("Failed to read wallet file: {e}"))
        })?;
        let jwk: ArweaveJWK = serde_json::from_str(&wallet_json).map_err(|e| {
            ActionError::workflow_failed(format!("Failed to parse wallet JWK: {e}"))
        })?;

        let ao_config = if let Some(ref gw) = deploy.gateways {
            let mu = gw.ao_mu.as_deref().unwrap_or("https://mu.ao-testnet.xyz");
            let cu = gw.ao_cu.as_deref().unwrap_or("https://cu.ao-testnet.xyz");
            let arweave = gw.arweave.as_deref().unwrap_or("https://arweave.net");
            AOConfig::new(mu, cu, arweave, 30_000)
                .map_err(|e| ActionError::workflow_failed(format!("Invalid AO config: {e}")))?
        } else {
            AOConfig::default()
        };

        let wallet_address = DataItemSigner::new(&jwk)
            .map_err(|e| ActionError::workflow_failed(format!("Failed to create signer: {e}")))?
            .owner_address();

        let ao_client = Arc::new(ProductionAOClient::new(ao_config, &jwk).map_err(|e| {
            ActionError::workflow_failed(format!("Failed to create AO client: {e}"))
        })?);

        let actions = Arc::new(ProductionActionsContainer::with_production_ao(ao_client));

        Ok(Self {
            process_id: deploy.process_id,
            module_id: deploy.module_id,
            wallet_address,
            actions,
        })
    }

    pub fn process_id(&self) -> &str {
        &self.process_id
    }

    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    pub fn wallet_address(&self) -> &str {
        &self.wallet_address
    }

    /// Create a ShareBuilder for the share operation
    pub fn share(
        &self,
    ) -> ShareBuilder<
        CoreCryptoServiceImpl,
        ProductionStorageService,
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
    ) -> RecoverBuilder<CoreCryptoServiceImpl, ProductionStorageService, NotSet, NotSet, NotSet>
    {
        RecoverBuilder::new(Arc::clone(&self.actions), self.process_id.clone())
    }

    /// Generate a PRE key pair
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.actions.generate_keypair()
    }
}
