//! Actions Layer - Developer-facing API endpoints (Facade Pattern)
//!
//! Provides simple functions for FORMIX operations:
//! - `share()` - Phase 1: Secret splitting and distribution
//! - `recover()` - Phase 3: Secret recovery
//! - `generate_keypair()` - PRE key pair generation
//!
//! # Architecture
//! ```text
//! Developer API
//!        │
//!        ▼
//! ┌──────────────────┐
//! │   Actions Layer  │ ← This module
//! │   (Facade)       │
//! └────────┬─────────┘
//!          │
//!     ┌────┴────┐
//!     ▼         ▼
//! Controller   UseCase
//!    Layer      Layer
//! ```
//!
//! # Example
//! ```rust,ignore
//! use formix::actions::{DefaultActionsContainer, ShareOptions};
//!
//! let container = DefaultActionsContainer::new();
//!
//! // Generate key pairs
//! let (owner_sk, owner_pk) = container.generate_keypair()?;
//! let (requester_sk, requester_pk) = container.generate_keypair()?;
//!
//! // Share a secret
//! let result = container.share(
//!     b"my secret data".to_vec(),
//!     3,  // threshold
//!     5,  // total_shares
//!     owner_sk,
//!     owner_pk,
//!     requester_pk,
//!     "owner_process_123".to_string(),
//!     None,
//! ).await?;
//!
//! // Recover the secret
//! let recovered = container.recover(
//!     &result.secret_id.as_str(),
//!     requester_sk,
//!     owner_pk,
//!     "requester_process_456".to_string(),
//!     None,
//! ).await?;
//! ```

pub mod builder;
pub mod client;
pub mod di;
pub mod error;
pub mod options;

pub use builder::{NotSet, RecoverBuilder, Set, ShareBuilder};
pub use client::{FormixClient, InitConfig};
pub use di::{ActionsContainer, DefaultActionsContainer};
pub use error::{ActionError, ActionResult};
pub use options::{RecoverOptions, ShareOptions};

#[cfg(feature = "production-ao")]
pub use client::{DeployConfig, GatewayConfig, ProductionFormixClient};
#[cfg(feature = "production-ao")]
pub use di::{ProductionActionsContainer, ProductionStorageService};

use zeroize::Zeroizing;

use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{CryptoService as CoreCryptoService, PublicKey, SecretKey};
use crate::usecase::dto::{SecretRecoveryResult, SecretSharingResult};
use crate::usecase::service::StorageService;
use crate::usecase::workflow::secret_recovery_service::SecretRecoveryWorkflowService;
use crate::usecase::workflow::secret_sharing_service::SecretSharingWorkflowService;

impl<C: CoreCryptoService, S: StorageService> ActionsContainer<C, S> {
    /// Share a secret by splitting and distributing it
    ///
    /// This is the main Phase 1 API endpoint that:
    /// 1. Validates input parameters via Controller layer
    /// 2. Creates SecretSharingRequest DTO via Extractor
    /// 3. Executes Phase 1 workflow via WorkflowService
    /// 4. Returns SecretSharingResult with secret_id and kFrags info
    ///
    /// # Arguments
    /// * `secret` - Secret data to split (will be zeroized after use)
    /// * `threshold` - Minimum shares required for reconstruction (k)
    /// * `total_shares` - Total number of shares to generate (n)
    /// * `owner_secret_key` - Owner's secret key for PRE
    /// * `owner_public_key` - Owner's public key for PRE (generated with owner_secret_key)
    /// * `requester_public_key` - Requester's public key for PRE
    /// * `owner_process_id` - Owner-Process ID for AO communication
    /// * `options` - Optional parameters (metadata, etc.)
    ///
    /// # Returns
    /// * `Ok(SecretSharingResult)` - Contains secret_id, tx IDs, and kFrag count
    /// * `Err(ActionError::ValidationFailed)` - Invalid parameters
    /// * `Err(ActionError::WorkflowFailed)` - Workflow execution failed
    ///
    /// # Example
    /// ```rust,ignore
    /// let (owner_sk, owner_pk) = container.generate_keypair()?;
    /// let (_, requester_pk) = container.generate_keypair()?;
    /// let result = container.share(
    ///     b"my secret".to_vec(),
    ///     3, 5,
    ///     owner_sk, owner_pk, requester_pk,
    ///     "owner_process".to_string(),
    ///     None,
    /// ).await?;
    /// println!("Secret ID: {}", result.secret_id);
    /// ```
    #[deprecated(since = "0.2.0", note = "use FormixClient::share() builder instead")]
    #[allow(clippy::too_many_arguments)]
    pub async fn share(
        &self,
        secret: Zeroizing<Vec<u8>>,
        threshold: u8,
        total_shares: u8,
        owner_secret_key: SecretKey,
        owner_public_key: PublicKey,
        requester_public_key: PublicKey,
        owner_process_id: String,
        options: Option<ShareOptions>,
    ) -> ActionResult<SecretSharingResult> {
        // Step 1: Validate parameters via Controller layer
        self.controller().share_validator().validate(
            &secret,
            threshold,
            total_shares,
            &owner_secret_key,
            &owner_public_key,
            &requester_public_key,
        )?;

        // Step 2: Extract DTO via Controller layer
        let metadata = options.and_then(|o| o.metadata);
        let request = self.controller().share_extractor().extract(
            secret,
            owner_secret_key,
            owner_public_key,
            requester_public_key,
            threshold,
            total_shares,
            owner_process_id,
            metadata,
        );

        // Step 3: Execute workflow via UseCase layer
        let result = self
            .workflow_services()
            .secret_sharing_service()
            .execute_secret_sharing(request)
            .await?;

        Ok(result)
    }

    /// Recover a secret using the secret_id
    ///
    /// This is the main Phase 3 API endpoint that:
    /// 1. Validates input parameters via Controller layer
    /// 2. Creates SecretRecoveryRequest DTO via Extractor
    /// 3. Executes Phase 3 workflow via WorkflowService
    /// 4. Returns SecretRecoveryResult with recovered secret (Zeroize on drop)
    ///
    /// # Arguments
    /// * `secret_id` - ID of the secret to recover
    /// * `requester_secret_key` - Requester's secret key for PRE decryption
    /// * `owner_public_key` - Owner's public key (delegating_pk) for PRE decapsulation
    /// * `requester_process_id` - Requester-Process ID for AO communication
    /// * `options` - Optional parameters (reserved for future use)
    ///
    /// # Returns
    /// * `Ok(SecretRecoveryResult)` - Contains recovered_secret (Zeroize on drop)
    /// * `Err(ActionError::ValidationFailed)` - Invalid parameters
    /// * `Err(ActionError::ResourceNotFound)` - Secret not found
    /// * `Err(ActionError::WorkflowFailed)` - Workflow execution failed
    ///
    /// # Security
    /// The `recovered_secret` field in the result implements Zeroize trait
    /// and will be automatically cleared from memory when dropped.
    #[deprecated(since = "0.2.0", note = "use FormixClient::recover() builder instead")]
    pub async fn recover(
        &self,
        secret_id: &str,
        requester_secret_key: SecretKey,
        owner_public_key: PublicKey,
        requester_process_id: String,
        _options: Option<RecoverOptions>,
    ) -> ActionResult<SecretRecoveryResult> {
        // Step 1: Validate parameters via Controller layer
        self.controller().recover_validator().validate(
            secret_id,
            &requester_secret_key,
            &owner_public_key,
            &requester_process_id,
        )?;

        // Step 2: Extract DTO via Controller layer
        let secret_id_value = SecretId::new(secret_id);
        let request = self.controller().recover_extractor().extract(
            secret_id_value,
            requester_secret_key,
            owner_public_key,
            requester_process_id,
        );

        // Step 3: Execute workflow via UseCase layer
        let result = self
            .workflow_services()
            .secret_recovery_service()
            .execute_secret_recovery(request)
            .await?;

        Ok(result)
    }

    /// Generate a PRE key pair for owner or requester operations
    ///
    /// Creates an Umbral PRE-compatible key pair that can be used for:
    /// - Owner: `owner_secret_key` and `owner_public_key` in share()
    /// - Requester: `requester_secret_key` in recover(), `requester_public_key` in share()
    ///
    /// # Returns
    /// * `Ok((SecretKey, PublicKey))` - Generated key pair
    /// * `Err(ActionError::CryptoError)` - Key generation failed
    ///
    /// # Security
    /// - `SecretKey` implements Zeroize trait and is cleared on drop
    /// - Keys are generated using cryptographically secure random
    /// - `SecretKey` is 32 bytes (256-bit)
    ///
    /// # Example
    /// ```rust,ignore
    /// let (secret_key, public_key) = container.generate_keypair()?;
    /// // Use keys for share() or recover()
    /// // secret_key is cleared when dropped
    /// ```
    pub fn generate_keypair(&self) -> ActionResult<(SecretKey, PublicKey)> {
        self.crypto_service()
            .generate_keypair()
            .map_err(|e| ActionError::crypto_error(e.to_string()))
    }
}
