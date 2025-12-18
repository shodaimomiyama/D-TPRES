//! CFrag Entity
//!
//! Represents a re-encrypted capsule fragment produced by Holder-Process.
//! Contains sensitive cryptographic data that must be zeroized on drop.

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{CFragId, KFragId, SecretId};

/// CFrag (Capsule Fragment) entity
///
/// Holds a serialized Umbral CapsuleFrag produced by proxy re-encryption.
/// cFragⱼ = PRE_ReEnc(kFragⱼ, Capsuleₒ)
///
/// # Security
/// - Implements Zeroize to clear sensitive cfrag_data on drop
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct CFrag {
    #[zeroize(skip)]
    id: CFragId,
    #[zeroize(skip)]
    secret_id: SecretId,
    #[zeroize(skip)]
    kfrag_id: KFragId,
    #[zeroize(skip)]
    holder_index: u8,
    cfrag_data: Vec<u8>,
    #[zeroize(skip)]
    created_at: u64,
}

impl CFrag {
    /// Create a new CFrag
    ///
    /// # Arguments
    /// * `secret_id` - ID of the parent Secret
    /// * `kfrag_id` - ID of the KFrag used to produce this CFrag
    /// * `holder_index` - Index of the holder that produced this CFrag
    /// * `cfrag_data` - Serialized Umbral CapsuleFrag bytes
    ///
    /// # Errors
    /// Returns error if cfrag_data is empty
    pub fn new(
        secret_id: SecretId,
        kfrag_id: KFragId,
        holder_index: u8,
        cfrag_data: Vec<u8>,
    ) -> Result<Self, DomainError> {
        if cfrag_data.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "CFrag".to_string(),
                field: "cfrag_data".to_string(),
                message: "CFrag data cannot be empty".to_string(),
            });
        }

        Ok(Self {
            id: CFragId::generate(),
            secret_id,
            kfrag_id,
            holder_index,
            cfrag_data,
            created_at: current_timestamp(),
        })
    }

    /// Reconstruct from stored data (for repository use)
    pub fn from_stored(
        id: CFragId,
        secret_id: SecretId,
        kfrag_id: KFragId,
        holder_index: u8,
        cfrag_data: Vec<u8>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            secret_id,
            kfrag_id,
            holder_index,
            cfrag_data,
            created_at,
        }
    }

    /// Get the CFrag ID
    pub fn id(&self) -> &CFragId {
        &self.id
    }

    /// Get the parent secret ID
    pub fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }

    /// Get the source KFrag ID
    pub fn kfrag_id(&self) -> &KFragId {
        &self.kfrag_id
    }

    /// Get the holder index
    pub fn holder_index(&self) -> u8 {
        self.holder_index
    }

    /// Get the serialized CFrag data
    pub fn cfrag_data(&self) -> &[u8] {
        &self.cfrag_data
    }

    /// Get creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Verify CFrag against capsule data
    ///
    /// # Note
    /// This is a placeholder. In production, this should use
    /// umbral_pre verification functions to validate the CFrag.
    ///
    /// # Arguments
    /// * `capsule_data` - The original capsule data to verify against
    ///
    /// # Returns
    /// Result indicating whether verification passed
    pub fn verify(&self, capsule_data: &[u8]) -> Result<bool, DomainError> {
        // Placeholder verification
        // In production: use umbral_pre::CapsuleFrag::verify()
        if capsule_data.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "CFrag".to_string(),
                field: "capsule_data".to_string(),
                message: "Cannot verify against empty capsule data".to_string(),
            });
        }

        // Placeholder: always return true for valid data
        // Real implementation would perform cryptographic verification
        Ok(!self.cfrag_data.is_empty())
    }
}

impl std::fmt::Debug for CFrag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Hide sensitive cfrag_data in debug output
        f.debug_struct("CFrag")
            .field("id", &self.id)
            .field("secret_id", &self.secret_id)
            .field("kfrag_id", &self.kfrag_id)
            .field("holder_index", &self.holder_index)
            .field("cfrag_data", &"[REDACTED]")
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl Clone for CFrag {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            secret_id: self.secret_id.clone(),
            kfrag_id: self.kfrag_id.clone(),
            holder_index: self.holder_index,
            cfrag_data: self.cfrag_data.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfrag_new_valid() {
        let secret_id = SecretId::generate();
        let kfrag_id = KFragId::generate();
        let cfrag_data = vec![1u8; 150];

        let cfrag = CFrag::new(secret_id.clone(), kfrag_id.clone(), 1, cfrag_data.clone()).unwrap();

        assert_eq!(cfrag.secret_id(), &secret_id);
        assert_eq!(cfrag.kfrag_id(), &kfrag_id);
        assert_eq!(cfrag.holder_index(), 1);
        assert_eq!(cfrag.cfrag_data(), cfrag_data.as_slice());
    }

    #[test]
    fn test_cfrag_empty_data_error() {
        let secret_id = SecretId::generate();
        let kfrag_id = KFragId::generate();

        let result = CFrag::new(secret_id, kfrag_id, 1, vec![]);

        assert!(result.is_err());
        if let Err(DomainError::EntityValidation { field, .. }) = result {
            assert_eq!(field, "cfrag_data");
        } else {
            panic!("Expected EntityValidation error");
        }
    }

    #[test]
    fn test_cfrag_verify_valid() {
        let secret_id = SecretId::generate();
        let kfrag_id = KFragId::generate();
        let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

        let capsule_data = vec![1u8; 50];
        let result = cfrag.verify(&capsule_data);

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_cfrag_verify_empty_capsule_error() {
        let secret_id = SecretId::generate();
        let kfrag_id = KFragId::generate();
        let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

        let result = cfrag.verify(&[]);

        assert!(result.is_err());
    }

    #[test]
    fn test_cfrag_debug_redacted() {
        let secret_id = SecretId::generate();
        let kfrag_id = KFragId::generate();
        let cfrag = CFrag::new(secret_id, kfrag_id, 1, vec![1u8; 100]).unwrap();

        let debug_str = format!("{:?}", cfrag);
        assert!(debug_str.contains("REDACTED"));
    }
}
