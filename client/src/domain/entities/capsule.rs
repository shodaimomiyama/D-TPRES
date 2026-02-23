//! Capsule Entity
//!
//! Represents an Umbral PRE capsule produced during encryption.
//! Capsuleₒ = PRE_Enc(pkₒ, kₒ) - used for re-encryption to requester's key.

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{CapsuleId, SecretId};

/// Capsule entity
///
/// Holds a serialized Umbral Capsule generated during PRE encryption.
/// The capsule enables proxy re-encryption from owner to requester.
///
/// # Note
/// Capsule data is public cryptographic material (not sensitive).
#[derive(Debug, Clone)]
pub struct Capsule {
    id: CapsuleId,
    secret_id: SecretId,
    capsule_data: Vec<u8>,
    owner_public_key: Vec<u8>,
    arweave_tx_id: Option<String>,
    created_at: u64,
}

impl Capsule {
    /// Create a new Capsule
    ///
    /// # Arguments
    /// * `secret_id` - ID of the parent Secret
    /// * `capsule_data` - Serialized Umbral Capsule bytes
    /// * `owner_public_key` - Owner's PRE public key
    ///
    /// # Errors
    /// Returns error if capsule_data or owner_public_key is empty
    pub fn new(
        secret_id: SecretId,
        capsule_data: Vec<u8>,
        owner_public_key: Vec<u8>,
    ) -> Result<Self, DomainError> {
        if capsule_data.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "Capsule".to_string(),
                field: "capsule_data".to_string(),
                message: "Capsule data cannot be empty".to_string(),
            });
        }

        if owner_public_key.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "Capsule".to_string(),
                field: "owner_public_key".to_string(),
                message: "Owner public key cannot be empty".to_string(),
            });
        }

        Ok(Self {
            id: CapsuleId::generate(),
            secret_id,
            capsule_data,
            owner_public_key,
            arweave_tx_id: None,
            created_at: current_timestamp(),
        })
    }

    /// Reconstruct from stored data (for repository use)
    pub const fn from_stored(
        id: CapsuleId,
        secret_id: SecretId,
        capsule_data: Vec<u8>,
        owner_public_key: Vec<u8>,
        arweave_tx_id: Option<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            secret_id,
            capsule_data,
            owner_public_key,
            arweave_tx_id,
            created_at,
        }
    }

    /// Get the capsule ID
    pub const fn id(&self) -> &CapsuleId {
        &self.id
    }

    /// Get the parent secret ID
    pub const fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }

    /// Get the serialized capsule data
    #[allow(clippy::missing_const_for_fn)]
    pub fn capsule_data(&self) -> &[u8] {
        &self.capsule_data
    }

    /// Get the owner's public key
    #[allow(clippy::missing_const_for_fn)]
    pub fn owner_public_key(&self) -> &[u8] {
        &self.owner_public_key
    }

    /// Get Arweave transaction ID if stored
    pub fn arweave_tx_id(&self) -> Option<&str> {
        self.arweave_tx_id.as_deref()
    }

    /// Set Arweave transaction ID after storage
    pub fn set_arweave_tx_id(&mut self, tx_id: String) {
        self.arweave_tx_id = Some(tx_id);
    }

    /// Get creation timestamp
    pub const fn created_at(&self) -> u64 {
        self.created_at
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
