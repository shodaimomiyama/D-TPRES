//! ShareCollection Entity
//!
//! Manages n encrypted Shamir shares (C₁...Cₙ) as a single entity.
//! All shares are stored together in one Arweave transaction.

use crate::domain::errors::DomainError;
use crate::domain::value_objects::{SecretId, ShareCollectionId};

/// Individual encrypted share data (Value Object-like internal structure)
#[derive(Debug, Clone)]
pub struct EncryptedShareData {
    /// Share index (1 <= index <= n)
    index: u8,
    /// Encrypted share data: Cᵢ = AES_GCM(kₒ, f(i))
    encrypted_data: Vec<u8>,
}

impl EncryptedShareData {
    /// Create a new EncryptedShareData
    pub fn new(index: u8, encrypted_data: Vec<u8>) -> Self {
        Self {
            index,
            encrypted_data,
        }
    }

    /// Get the share index
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Get the encrypted data
    pub fn encrypted_data(&self) -> &[u8] {
        &self.encrypted_data
    }
}

/// ShareCollection entity
///
/// Holds all n encrypted Shamir shares for a secret.
/// Designed for atomic storage in a single Arweave transaction.
#[derive(Debug, Clone)]
pub struct ShareCollection {
    id: ShareCollectionId,
    secret_id: SecretId,
    threshold_k: u8,
    threshold_n: u8,
    shares: Vec<EncryptedShareData>,
    arweave_tx_id: Option<String>,
    created_at: u64,
}

impl ShareCollection {
    /// Create a new ShareCollection
    ///
    /// # Arguments
    /// * `secret_id` - ID of the parent Secret
    /// * `threshold_k` - Minimum shares required for recovery
    /// * `threshold_n` - Total number of shares
    /// * `shares` - Vector of encrypted share data
    ///
    /// # Errors
    /// Returns error if:
    /// - Number of shares doesn't match threshold_n
    /// - Any share index is out of range
    /// - Duplicate share indices exist
    pub fn new(
        secret_id: SecretId,
        threshold_k: u8,
        threshold_n: u8,
        shares: Vec<EncryptedShareData>,
    ) -> Result<Self, DomainError> {
        // Validate share count
        if shares.len() != threshold_n as usize {
            return Err(DomainError::EntityValidation {
                entity_type: "ShareCollection".to_string(),
                field: "shares".to_string(),
                message: format!(
                    "Share count mismatch: expected {}, got {}",
                    threshold_n,
                    shares.len()
                ),
            });
        }

        // Validate share indices
        let mut seen_indices = std::collections::HashSet::new();
        for share in &shares {
            if share.index == 0 || share.index > threshold_n {
                return Err(DomainError::EntityValidation {
                    entity_type: "ShareCollection".to_string(),
                    field: "share.index".to_string(),
                    message: format!(
                        "Invalid share index {}: must be in range 1..={}",
                        share.index, threshold_n
                    ),
                });
            }

            if !seen_indices.insert(share.index) {
                return Err(DomainError::EntityValidation {
                    entity_type: "ShareCollection".to_string(),
                    field: "share.index".to_string(),
                    message: format!("Duplicate share index: {}", share.index),
                });
            }

            if share.encrypted_data.is_empty() {
                return Err(DomainError::EntityValidation {
                    entity_type: "ShareCollection".to_string(),
                    field: "share.encrypted_data".to_string(),
                    message: format!("Empty encrypted data for share index {}", share.index),
                });
            }
        }

        Ok(Self {
            id: ShareCollectionId::generate(),
            secret_id,
            threshold_k,
            threshold_n,
            shares,
            arweave_tx_id: None,
            created_at: current_timestamp(),
        })
    }

    /// Reconstruct from stored data (for repository use)
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ShareCollectionId,
        secret_id: SecretId,
        threshold_k: u8,
        threshold_n: u8,
        shares: Vec<EncryptedShareData>,
        arweave_tx_id: Option<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            secret_id,
            threshold_k,
            threshold_n,
            shares,
            arweave_tx_id,
            created_at,
        }
    }

    /// Get the collection ID
    pub fn id(&self) -> &ShareCollectionId {
        &self.id
    }

    /// Get the parent secret ID
    pub fn secret_id(&self) -> &SecretId {
        &self.secret_id
    }

    /// Get threshold k
    pub fn threshold_k(&self) -> u8 {
        self.threshold_k
    }

    /// Get threshold n
    pub fn threshold_n(&self) -> u8 {
        self.threshold_n
    }

    /// Get all shares
    pub fn shares(&self) -> &[EncryptedShareData] {
        &self.shares
    }

    /// Get a share by index
    pub fn get_share(&self, index: u8) -> Option<&EncryptedShareData> {
        self.shares.iter().find(|s| s.index == index)
    }

    /// Get shares by multiple indices
    pub fn get_shares_by_indices(&self, indices: &[u8]) -> Vec<&EncryptedShareData> {
        self.shares
            .iter()
            .filter(|s| indices.contains(&s.index))
            .collect()
    }

    /// Get the number of shares
    pub fn shares_count(&self) -> usize {
        self.shares.len()
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
    pub fn created_at(&self) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_shares(n: u8) -> Vec<EncryptedShareData> {
        (1..=n)
            .map(|i| EncryptedShareData::new(i, vec![i; 32]))
            .collect()
    }

    #[test]
    fn test_share_collection_new_valid() {
        let secret_id = SecretId::generate();
        let shares = create_test_shares(3);

        let collection = ShareCollection::new(secret_id.clone(), 2, 3, shares).unwrap();

        assert_eq!(collection.threshold_k(), 2);
        assert_eq!(collection.threshold_n(), 3);
        assert_eq!(collection.shares_count(), 3);
        assert_eq!(collection.secret_id(), &secret_id);
    }

    #[test]
    fn test_share_collection_wrong_share_count() {
        let secret_id = SecretId::generate();
        let shares = create_test_shares(2); // Only 2 shares for n=3

        let result = ShareCollection::new(secret_id, 2, 3, shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_share_collection_invalid_index_zero() {
        let secret_id = SecretId::generate();
        let mut shares = create_test_shares(2);
        shares.push(EncryptedShareData::new(0, vec![1; 32])); // Invalid index 0

        let result = ShareCollection::new(secret_id, 2, 3, shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_share_collection_invalid_index_too_high() {
        let secret_id = SecretId::generate();
        let mut shares = create_test_shares(2);
        shares.push(EncryptedShareData::new(4, vec![1; 32])); // Index 4 > n=3

        let result = ShareCollection::new(secret_id, 2, 3, shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_share_collection_duplicate_index() {
        let secret_id = SecretId::generate();
        let shares = vec![
            EncryptedShareData::new(1, vec![1; 32]),
            EncryptedShareData::new(1, vec![2; 32]), // Duplicate index
            EncryptedShareData::new(2, vec![3; 32]),
        ];

        let result = ShareCollection::new(secret_id, 2, 3, shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_share_collection_empty_data() {
        let secret_id = SecretId::generate();
        let shares = vec![
            EncryptedShareData::new(1, vec![1; 32]),
            EncryptedShareData::new(2, vec![]), // Empty data
            EncryptedShareData::new(3, vec![3; 32]),
        ];

        let result = ShareCollection::new(secret_id, 2, 3, shares);
        assert!(result.is_err());
    }

    #[test]
    fn test_share_collection_get_share() {
        let secret_id = SecretId::generate();
        let shares = create_test_shares(3);

        let collection = ShareCollection::new(secret_id, 2, 3, shares).unwrap();

        let share = collection.get_share(2).unwrap();
        assert_eq!(share.index(), 2);

        assert!(collection.get_share(4).is_none());
    }

    #[test]
    fn test_share_collection_get_shares_by_indices() {
        let secret_id = SecretId::generate();
        let shares = create_test_shares(5);

        let collection = ShareCollection::new(secret_id, 3, 5, shares).unwrap();

        let selected = collection.get_shares_by_indices(&[1, 3, 5]);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_share_collection_set_arweave_tx_id() {
        let secret_id = SecretId::generate();
        let shares = create_test_shares(3);

        let mut collection = ShareCollection::new(secret_id, 2, 3, shares).unwrap();

        assert!(collection.arweave_tx_id().is_none());

        collection.set_arweave_tx_id("tx-123".to_string());

        assert_eq!(collection.arweave_tx_id(), Some("tx-123"));
    }
}
