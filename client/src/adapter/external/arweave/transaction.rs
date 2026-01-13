//! Arweave transaction types and building logic
//!
//! This module contains the data structures for Arweave transactions
//! and helper functions for building transactions ready for signing.

use serde::{Deserialize, Serialize};

use super::client::base64url_decode;
use super::deep_hash::build_signature_message;

// =============================================================================
// Transaction Types
// =============================================================================

/// Arweave transaction structure for posting data (format 2)
#[derive(Debug, Serialize)]
pub struct ArweaveTransaction {
    /// Transaction format (always 2 for format 2)
    pub format: u8,
    /// Transaction ID (calculated from signature hash)
    pub id: String,
    /// Last anchor transaction ID
    pub last_tx: String,
    /// Wallet public key (Base64URL encoded)
    pub owner: String,
    /// Tags with Base64URL encoded name/value
    pub tags: Vec<EncodedTag>,
    /// Target address (empty for data-only transactions)
    pub target: String,
    /// Amount in Winston (0 for data-only transactions)
    pub quantity: String,
    /// Base64URL encoded data
    pub data: String,
    /// Data size as string
    pub data_size: String,
    /// Data root hash (Merkle tree root)
    pub data_root: String,
    /// Transaction reward (fee in Winston)
    pub reward: String,
    /// RSA-PSS signature (Base64URL encoded)
    pub signature: String,
}

/// Tag with Base64URL encoded name and value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedTag {
    pub name: String,
    pub value: String,
}

// =============================================================================
// Signature Data Building
// =============================================================================

/// Build the signature data for an Arweave format 2 transaction
///
/// This function constructs the message that needs to be signed using DeepHash.
pub fn build_signature_data(tx: &ArweaveTransaction) -> Vec<u8> {
    let owner_bytes = base64url_decode(&tx.owner).unwrap_or_default();
    let target_bytes = base64url_decode(&tx.target).unwrap_or_default();
    let last_tx_bytes = base64url_decode(&tx.last_tx).unwrap_or_default();
    let data_root_bytes = base64url_decode(&tx.data_root).unwrap_or_default();

    build_signature_message(
        &owner_bytes,
        &target_bytes,
        &tx.quantity,
        &tx.reward,
        &last_tx_bytes,
        &tx.tags,
        &tx.data_size,
        &data_root_bytes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoded_tag_serialization() {
        let tag = EncodedTag {
            name: "dGVzdA".to_string(),
            value: "dmFsdWU".to_string(),
        };
        let json = serde_json::to_string(&tag).unwrap();
        assert!(json.contains("dGVzdA"));
        assert!(json.contains("dmFsdWU"));
    }
}
