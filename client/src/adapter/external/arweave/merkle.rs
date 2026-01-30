//! Arweave Merkle tree implementation for data_root calculation
//!
//! Arweave uses a custom Merkle tree structure for chunked data verification.
//! This implementation follows the arweave-js reference implementation.
//!
//! Reference: https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/merkle.ts

// Allow arithmetic operations in this module since they are controlled and safe
// within the bounds of Arweave's chunking algorithm
#![allow(clippy::arithmetic_side_effects)]

use sha2::{Digest, Sha256};

use super::client::base64url_encode;

/// Maximum chunk size (256 KB)
pub const MAX_CHUNK_SIZE: usize = 256 * 1024;

/// Minimum chunk size (32 KB) - used when splitting would create chunks smaller than this
pub const MIN_CHUNK_SIZE: usize = 32 * 1024;

/// Merkle tree node containing the hash ID and byte range
#[derive(Debug, Clone)]
struct MerkleNode {
    id: Vec<u8>,
    max_byte_range: usize,
}

/// Compute SHA-256 hash
fn sha256(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/// Concatenate byte slices
fn concat_buffers(buffers: &[&[u8]]) -> Vec<u8> {
    buffers.iter().flat_map(|b| b.iter().copied()).collect()
}

/// Convert usize to big-endian byte buffer (32 bytes for Arweave compatibility)
#[allow(clippy::cast_possible_truncation)]
fn int_to_buffer(value: usize) -> Vec<u8> {
    let bytes = (value as u64).to_be_bytes();
    // Pad to 32 bytes (Arweave uses 256-bit integers for byte ranges)
    let mut result = vec![0u8; 24];
    result.extend_from_slice(&bytes);
    result
}

/// Split data into chunks following Arweave's chunking algorithm
///
/// Arweave uses a specific algorithm to split data into chunks:
/// - Maximum chunk size is 256KB
/// - Minimum chunk size is 32KB (to avoid very small final chunks)
/// - If remaining data after split would be less than `MIN_CHUNK_SIZE`,
///   the current chunk is reduced to balance sizes
#[allow(clippy::indexing_slicing)]
fn chunk_data(payload: &[u8]) -> Vec<Vec<u8>> {
    if payload.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut cursor = 0;
    let data_len = payload.len();

    while cursor < data_len {
        let remaining = data_len - cursor;

        // If remaining fits in one chunk, take it all
        if remaining <= MAX_CHUNK_SIZE {
            chunks.push(payload[cursor..].to_vec());
            break;
        }

        // Check if splitting at MAX_CHUNK_SIZE would leave a too-small remainder
        let remainder_after_max = remaining - MAX_CHUNK_SIZE;
        let chunk_size = if remainder_after_max > 0 && remainder_after_max < MIN_CHUNK_SIZE {
            // Split more evenly to avoid tiny final chunk
            remaining / 2
        } else {
            MAX_CHUNK_SIZE
        };

        chunks.push(payload[cursor..cursor + chunk_size].to_vec());
        cursor += chunk_size;
    }

    chunks
}

/// Generate a leaf node from chunk data
///
/// Leaf node ID = SHA-256(SHA-256(data_hash) || SHA-256(max_byte_range))
fn generate_leaf(data_hash: &[u8], max_byte_range: usize) -> MerkleNode {
    let range_bytes = int_to_buffer(max_byte_range);
    let id = sha256(&concat_buffers(&[
        &sha256(data_hash),
        &sha256(&range_bytes),
    ]));
    MerkleNode { id, max_byte_range }
}

/// Hash two nodes into a branch node
///
/// Branch node ID = SHA-256(SHA-256(left.id) || SHA-256(right.id) || SHA-256(right.max_byte_range))
fn hash_branch(left: &MerkleNode, right: &MerkleNode) -> MerkleNode {
    let id = sha256(&concat_buffers(&[
        &sha256(&left.id),
        &sha256(&right.id),
        &sha256(&int_to_buffer(right.max_byte_range)),
    ]));
    MerkleNode {
        id,
        max_byte_range: right.max_byte_range,
    }
}

/// Generate leaf nodes from chunks
fn generate_leaves(chunks: &[Vec<u8>]) -> Vec<MerkleNode> {
    let mut byte_offset = 0;
    chunks
        .iter()
        .map(|chunk| {
            let data_hash = sha256(chunk);
            byte_offset += chunk.len();
            generate_leaf(&data_hash, byte_offset)
        })
        .collect()
}

/// Build Merkle tree layers from leaf nodes up to root
fn build_layers(mut nodes: Vec<MerkleNode>) -> MerkleNode {
    if nodes.is_empty() {
        // Empty data case: return empty hash
        return MerkleNode {
            id: vec![0u8; 32],
            max_byte_range: 0,
        };
    }

    while nodes.len() > 1 {
        let mut next_layer = Vec::new();

        for chunk in nodes.chunks(2) {
            match chunk {
                [left, right] => {
                    next_layer.push(hash_branch(left, right));
                }
                [single] => {
                    // Odd node: promote to next layer
                    next_layer.push(single.clone());
                }
                // chunks(2) only returns slices of length 1 or 2
                _ => {}
            }
        }

        nodes = next_layer;
    }

    // Safe: we checked nodes is not empty and loop maintains at least one node
    nodes.into_iter().next().unwrap_or_else(|| MerkleNode {
        id: vec![0u8; 32],
        max_byte_range: 0,
    })
}

/// Compute data_root for payload using Arweave's Merkle tree algorithm
///
/// This is the main public API. It:
/// 1. Splits data into chunks
/// 2. Generates leaf nodes from chunk hashes
/// 3. Builds the Merkle tree
/// 4. Returns the root hash as Base64URL encoded string
pub fn compute_data_root(payload: &[u8]) -> String {
    if payload.is_empty() {
        return String::new();
    }

    let chunks = chunk_data(payload);
    let leaves = generate_leaves(&chunks);
    let root = build_layers(leaves);
    base64url_encode(&root.id)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_data_empty() {
        let chunks = chunk_data(&[]);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_data_small() {
        let payload = vec![0u8; 1000];
        let chunks = chunk_data(&payload);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 1000);
    }

    #[test]
    fn test_chunk_data_exactly_max() {
        let payload = vec![0u8; MAX_CHUNK_SIZE];
        let chunks = chunk_data(&payload);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_chunk_data_multiple() {
        let payload = vec![0u8; MAX_CHUNK_SIZE * 2 + 1000];
        let chunks = chunk_data(&payload);
        assert!(chunks.len() >= 2);
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, payload.len());
    }

    #[test]
    fn test_compute_data_root_empty() {
        let root = compute_data_root(&[]);
        assert!(root.is_empty());
    }

    #[test]
    fn test_compute_data_root_small() {
        let payload = b"hello world";
        let root = compute_data_root(payload);
        assert!(!root.is_empty());
        // Base64URL encoded SHA-256 is 43 characters
        assert!(root.len() >= 40);
    }

    #[test]
    fn test_int_to_buffer() {
        let buf = int_to_buffer(256);
        assert_eq!(buf.len(), 32);
        // Last 8 bytes should be big-endian 256
        assert_eq!(&buf[24..], &[0, 0, 0, 0, 0, 0, 1, 0]);
    }

    #[test]
    fn test_deterministic() {
        let payload = b"test payload for merkle tree";
        let root1 = compute_data_root(payload);
        let root2 = compute_data_root(payload);
        assert_eq!(root1, root2);
    }
}
