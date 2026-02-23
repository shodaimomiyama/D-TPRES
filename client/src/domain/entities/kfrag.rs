//! KFrag Entity
//!
//! Represents a re-encryption key fragment distributed to Holder-Process.
//! Contains sensitive cryptographic data that must be zeroized on drop.

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{KFragId, SecretId};

/// KFrag (Key Fragment) entity
///
/// Holds a serialized Umbral KeyFrag used for proxy re-encryption.
/// Distributed to Holder-Processes for re-encryption operations.
///
/// # Security
/// - Implements Zeroize to clear sensitive kfrag_data on drop
/// - holder_process_id is not zeroized as it's not sensitive
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct KFrag {
    #[zeroize(skip)]
    id: KFragId,
    #[zeroize(skip)]
    secret_id: SecretId,
    #[zeroize(skip)]
    holder_index: u8,
    #[zeroize(skip)]
    holder_process_id: Option<String>,
    kfrag_data: Vec<u8>,
    #[zeroize(skip)]
    created_at: u64,
}

impl KFrag {
    /// Create a new KFrag
    ///
    /// # Arguments
    /// * `secret_id` - ID of the parent Secret
    /// * `holder_index` - Index of the holder (1..=n)
    /// * `threshold_n` - Total number of holders for validation
    /// * `kfrag_data` - Serialized Umbral KeyFrag bytes
    ///
    /// # Errors
    /// Returns error if:
    /// - holder_index is 0 or greater than threshold_n
    /// - kfrag_data is empty
    pub fn new(
        secret_id: SecretId,
        holder_index: u8,
        threshold_n: u8,
        kfrag_data: Vec<u8>,
    ) -> Result<Self, DomainError> {
        if holder_index == 0 || holder_index > threshold_n {
            return Err(DomainError::EntityValidation {
                entity_type: "KFrag".to_string(),
                field: "holder_index".to_string(),
                message: format!(
                    "Invalid holder index {holder_index}: must be in range 1..={threshold_n}"
                ),
            });
        }

        if kfrag_data.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "KFrag".to_string(),
                field: "kfrag_data".to_string(),
                message: "KFrag data cannot be empty".to_string(),
            });
        }

        Ok(Self {
            id: KFragId::generate(),
            secret_id,
            holder_index,
            holder_process_id: None,
            kfrag_data,
            created_at: current_timestamp(),
        })
    }

    /// Reconstruct from stored data (for repository use)
    pub const fn from_stored(
        id: KFragId,
        secret_id: SecretId,
        holder_index: u8,
        holder_process_id: Option<String>,
        kfrag_data: Vec<u8>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            secret_id,
            holder_index,
            holder_process_id,
            kfrag_data,
            created_at,
        }
    }

    /// Get the KFrag ID
    pub const fn id(&self) -> &KFragId {
        &self.id
    }

    /// Get the parent secret ID
    pub const fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }

    /// Get the holder index
    pub const fn holder_index(&self) -> u8 {
        self.holder_index
    }

    /// Get the assigned Holder-Process ID if set
    pub fn holder_process_id(&self) -> Option<&str> {
        self.holder_process_id.as_deref()
    }

    /// Get the serialized KFrag data
    #[allow(clippy::missing_const_for_fn)]
    pub fn kfrag_data(&self) -> &[u8] {
        &self.kfrag_data
    }

    /// Get creation timestamp
    pub const fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Assign this KFrag to a Holder-Process
    pub fn set_holder_process_id(&mut self, process_id: String) {
        self.holder_process_id = Some(process_id);
    }
}

impl std::fmt::Debug for KFrag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hide sensitive kfrag_data in debug output
        f.debug_struct("KFrag")
            .field("id", &self.id)
            .field("secret_id", &self.secret_id)
            .field("holder_index", &self.holder_index)
            .field("holder_process_id", &self.holder_process_id)
            .field("kfrag_data", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl Clone for KFrag {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            secret_id: self.secret_id.clone(),
            holder_index: self.holder_index,
            holder_process_id: self.holder_process_id.clone(),
            kfrag_data: self.kfrag_data.clone(),
            created_at: self.created_at,
        }
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
