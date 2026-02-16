//! Type-State Builder Pattern for share and recover operations
//!
//! Provides compile-time enforcement of required parameters using
//! the type-state pattern. `execute()` is only available when all
//! required fields have been set.

use std::marker::PhantomData;
use std::sync::Arc;

use zeroize::Zeroizing;

use crate::actions::error::{ActionError, ActionResult};
use crate::actions::options::ShareOptions;
use crate::domain::value_objects::SecretId;
use crate::usecase::core::crypto::{CryptoService as CoreCryptoService, PublicKey, SecretKey};
use crate::usecase::dto::{SecretMetadata, SecretRecoveryResult, SecretSharingResult};
use crate::usecase::service::StorageService;

use super::di::ActionsContainer;

/// Marker type indicating a required field has been set
pub struct Set;
/// Marker type indicating a required field has not been set
pub struct NotSet;

/// Type-state builder for the share operation.
///
/// Required fields: secret, threshold, total_shares, owner_key, requester_key.
/// `execute()` is only callable when all type parameters are `Set`.
pub struct ShareBuilder<
    C: CoreCryptoService,
    Ss: StorageService,
    Secret,
    Threshold,
    TotalShares,
    OwnerKey,
    RequesterKey,
> {
    container: Arc<ActionsContainer<C, Ss>>,
    process_id: String,
    secret: Option<Zeroizing<Vec<u8>>>,
    threshold: Option<u8>,
    total_shares: Option<u8>,
    owner_key: Option<SecretKey>,
    requester_key: Option<PublicKey>,
    metadata: Option<SecretMetadata>,
    _marker: PhantomData<(Secret, Threshold, TotalShares, OwnerKey, RequesterKey)>,
}

impl<C: CoreCryptoService, Ss: StorageService>
    ShareBuilder<C, Ss, NotSet, NotSet, NotSet, NotSet, NotSet>
{
    pub(crate) fn new(container: Arc<ActionsContainer<C, Ss>>, process_id: String) -> Self {
        ShareBuilder {
            container,
            process_id,
            secret: None,
            threshold: None,
            total_shares: None,
            owner_key: None,
            requester_key: None,
            metadata: None,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, T, N, O, R> ShareBuilder<C, Ss, NotSet, T, N, O, R> {
    pub fn secret(self, secret: Vec<u8>) -> ShareBuilder<C, Ss, Set, T, N, O, R> {
        ShareBuilder {
            container: self.container,
            process_id: self.process_id,
            secret: Some(Zeroizing::new(secret)),
            threshold: self.threshold,
            total_shares: self.total_shares,
            owner_key: self.owner_key,
            requester_key: self.requester_key,
            metadata: self.metadata,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, N, O, R> ShareBuilder<C, Ss, S, NotSet, N, O, R> {
    pub fn threshold(self, k: u8) -> ShareBuilder<C, Ss, S, Set, N, O, R> {
        ShareBuilder {
            container: self.container,
            process_id: self.process_id,
            secret: self.secret,
            threshold: Some(k),
            total_shares: self.total_shares,
            owner_key: self.owner_key,
            requester_key: self.requester_key,
            metadata: self.metadata,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, T, O, R> ShareBuilder<C, Ss, S, T, NotSet, O, R> {
    pub fn total_shares(self, n: u8) -> ShareBuilder<C, Ss, S, T, Set, O, R> {
        ShareBuilder {
            container: self.container,
            process_id: self.process_id,
            secret: self.secret,
            threshold: self.threshold,
            total_shares: Some(n),
            owner_key: self.owner_key,
            requester_key: self.requester_key,
            metadata: self.metadata,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, T, N, R> ShareBuilder<C, Ss, S, T, N, NotSet, R> {
    pub fn owner_key(self, key: SecretKey) -> ShareBuilder<C, Ss, S, T, N, Set, R> {
        ShareBuilder {
            container: self.container,
            process_id: self.process_id,
            secret: self.secret,
            threshold: self.threshold,
            total_shares: self.total_shares,
            owner_key: Some(key),
            requester_key: self.requester_key,
            metadata: self.metadata,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, T, N, O> ShareBuilder<C, Ss, S, T, N, O, NotSet> {
    pub fn requester_key(self, key: PublicKey) -> ShareBuilder<C, Ss, S, T, N, O, Set> {
        ShareBuilder {
            container: self.container,
            process_id: self.process_id,
            secret: self.secret,
            threshold: self.threshold,
            total_shares: self.total_shares,
            owner_key: self.owner_key,
            requester_key: Some(key),
            metadata: self.metadata,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, T, N, O, R> ShareBuilder<C, Ss, S, T, N, O, R> {
    #[must_use]
    pub fn metadata(mut self, meta: Option<SecretMetadata>) -> Self {
        self.metadata = meta;
        self
    }
}

impl<C: CoreCryptoService, Ss: StorageService> ShareBuilder<C, Ss, Set, Set, Set, Set, Set> {
    #[allow(deprecated, clippy::missing_const_for_fn, clippy::large_futures)]
    pub async fn execute(self) -> ActionResult<SecretSharingResult> {
        let mut secret = self.secret.ok_or_else(|| {
            ActionError::validation_failed("missing_secret", "secret is required")
        })?;
        let threshold = self.threshold.ok_or_else(|| {
            ActionError::validation_failed("missing_threshold", "threshold is required")
        })?;
        let total_shares = self.total_shares.ok_or_else(|| {
            ActionError::validation_failed("missing_total_shares", "total_shares is required")
        })?;
        let owner_secret_key = self.owner_key.ok_or_else(|| {
            ActionError::validation_failed("missing_owner_key", "owner_key is required")
        })?;
        let requester_public_key = self.requester_key.ok_or_else(|| {
            ActionError::validation_failed("missing_requester_key", "requester_key is required")
        })?;

        let owner_public_key = self
            .container
            .crypto_service()
            .derive_public_key(&owner_secret_key)
            .map_err(|e| ActionError::crypto_error(e.to_string()))?;

        let secret = std::mem::take(&mut *secret);
        let options = self.metadata.map(ShareOptions::with_metadata);

        self.container
            .share(
                secret,
                threshold,
                total_shares,
                owner_secret_key,
                owner_public_key,
                requester_public_key,
                self.process_id,
                options,
            )
            .await
    }
}

/// Type-state builder for the recover operation.
///
/// Required fields: secret_id, requester_key, owner_key.
/// `execute()` is only callable when all type parameters are `Set`.
///
/// Uses a single `process_id` (set at construction) because `DTpresClient`
/// is designed for self-service usage where one user owns both the owner
/// and requester roles within the same AO process context.
pub struct RecoverBuilder<
    C: CoreCryptoService,
    Ss: StorageService,
    SecretIdState,
    RequesterKeyState,
    OwnerKeyState,
> {
    container: Arc<ActionsContainer<C, Ss>>,
    process_id: String,
    secret_id: Option<String>,
    requester_key: Option<SecretKey>,
    owner_key: Option<PublicKey>,
    _marker: PhantomData<(SecretIdState, RequesterKeyState, OwnerKeyState)>,
}

impl<C: CoreCryptoService, Ss: StorageService> RecoverBuilder<C, Ss, NotSet, NotSet, NotSet> {
    pub(crate) fn new(container: Arc<ActionsContainer<C, Ss>>, process_id: String) -> Self {
        RecoverBuilder {
            container,
            process_id,
            secret_id: None,
            requester_key: None,
            owner_key: None,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, R, O> RecoverBuilder<C, Ss, NotSet, R, O> {
    pub fn secret_id(self, id: &SecretId) -> RecoverBuilder<C, Ss, Set, R, O> {
        RecoverBuilder {
            container: self.container,
            process_id: self.process_id,
            secret_id: Some(id.as_str().to_string()),
            requester_key: self.requester_key,
            owner_key: self.owner_key,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, O> RecoverBuilder<C, Ss, S, NotSet, O> {
    pub fn requester_key(self, key: SecretKey) -> RecoverBuilder<C, Ss, S, Set, O> {
        RecoverBuilder {
            container: self.container,
            process_id: self.process_id,
            secret_id: self.secret_id,
            requester_key: Some(key),
            owner_key: self.owner_key,
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService, S, R> RecoverBuilder<C, Ss, S, R, NotSet> {
    pub fn owner_key(self, key: PublicKey) -> RecoverBuilder<C, Ss, S, R, Set> {
        RecoverBuilder {
            container: self.container,
            process_id: self.process_id,
            secret_id: self.secret_id,
            requester_key: self.requester_key,
            owner_key: Some(key),
            _marker: PhantomData,
        }
    }
}

impl<C: CoreCryptoService, Ss: StorageService> RecoverBuilder<C, Ss, Set, Set, Set> {
    #[allow(deprecated, clippy::missing_const_for_fn, clippy::large_futures)]
    pub async fn execute(self) -> ActionResult<SecretRecoveryResult> {
        let secret_id = self.secret_id.ok_or_else(|| {
            ActionError::validation_failed("missing_secret_id", "secret_id is required")
        })?;
        let requester_key = self.requester_key.ok_or_else(|| {
            ActionError::validation_failed("missing_requester_key", "requester_key is required")
        })?;
        let owner_key = self.owner_key.ok_or_else(|| {
            ActionError::validation_failed("missing_owner_key", "owner_key is required")
        })?;

        self.container
            .recover(&secret_id, requester_key, owner_key, self.process_id, None)
            .await
    }
}
