//! Arweave DeepHash algorithm implementation
//!
//! DeepHash is Arweave's cryptographic hashing scheme used for transaction signing.
//! It uses SHA-384 to hash nested data structures in a deterministic way.
//!
//! Reference: https://github.com/ArweaveTeam/arweave-js/blob/master/src/common/lib/deepHash.ts

use sha2::{Digest, Sha384};

use crate::adapter::errors::AdapterError;

use super::client::base64url_decode;
use super::transaction::EncodedTag;

// =============================================================================
// DeepHash Item Type
// =============================================================================

/// DeepHash input item - either blob (bytes) or nested list
///
/// This type represents the recursive structure that Arweave's DeepHash algorithm
/// processes. It mirrors the TypeScript type:
/// ```typescript
/// type DeepHashChunk = Uint8Array | DeepHashChunks;
/// interface DeepHashChunks extends Array<DeepHashChunk> {}
/// ```
pub enum DeepHashItem {
    /// Raw byte data (blob)
    Blob(Vec<u8>),
    /// Nested list of items
    List(Vec<DeepHashItem>),
}

// =============================================================================
// Internal Helpers
// =============================================================================

/// Concatenate multiple byte slices into a single Vec
fn concat_buffers(buffers: &[&[u8]]) -> Vec<u8> {
    buffers.iter().flat_map(|b| b.iter().copied()).collect()
}

/// Compute SHA-384 hash of input bytes
fn sha384(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha384::new();
    hasher.update(input);
    hasher.finalize().to_vec()
}

/// DeepHash for a single blob (byte array)
///
/// Formula: SHA-384(SHA-384("blob" + len) || SHA-384(blob))
fn deep_hash_blob(blob: &[u8]) -> Vec<u8> {
    let tag = concat_buffers(&[b"blob", blob.len().to_string().as_bytes()]);

    let tag_hash = sha384(&tag);
    let blob_hash = sha384(blob);

    sha384(&concat_buffers(&[&tag_hash, &blob_hash]))
}

// =============================================================================
// DeepHash Main Function
// =============================================================================

/// Compute DeepHash for nested structures
///
/// This function recursively processes `DeepHashItem` following Arweave's algorithm:
/// - For blobs: `SHA-384(SHA-384("blob" + len) || SHA-384(data))`
/// - For lists: Start with `SHA-384("list" + len)`, then accumulate:
///   `SHA-384(acc || deep_hash(item))` for each item
///
/// Empty lists produce `SHA-384("list0")` per Arweave spec (not treated as empty blob).
pub fn deep_hash(item: &DeepHashItem) -> Vec<u8> {
    match item {
        DeepHashItem::Blob(bytes) => deep_hash_blob(bytes),
        DeepHashItem::List(items) => {
            let tag = concat_buffers(&[b"list", items.len().to_string().as_bytes()]);
            let mut acc = sha384(&tag);

            for sub_item in items {
                let item_hash = deep_hash(sub_item);
                acc = sha384(&concat_buffers(&[&acc, &item_hash]));
            }

            acc
        }
    }
}

// =============================================================================
// Signature Data Building
// =============================================================================

/// Build signature data using DeepHash for Arweave format 2 transactions
///
/// The signature message is constructed as a nested structure following Arweave's spec:
/// ```text
/// [
///   "2",                                    // format
///   owner,                                  // decoded bytes
///   target,                                 // decoded bytes
///   quantity,                               // string as bytes
///   reward,                                 // string as bytes
///   last_tx,                                // decoded bytes
///   [[name1, value1], [name2, value2], ...], // tags as nested list
///   data_size,                              // string as bytes
///   data_root                               // decoded bytes
/// ]
/// ```
///
/// # Errors
///
/// Returns `AdapterError::ValidationError` if any tag name or value fails Base64URL decoding.
#[allow(clippy::too_many_arguments)]
pub fn build_signature_message(
    owner: &[u8],
    target: &[u8],
    quantity: &str,
    reward: &str,
    last_tx: &[u8],
    tags: &[EncodedTag],
    data_size: &str,
    data_root: &[u8],
) -> Result<Vec<u8>, AdapterError> {
    // Build tags as nested list: [[name1, value1], [name2, value2], ...]
    let mut tag_pairs: Vec<DeepHashItem> = Vec::with_capacity(tags.len());
    for (i, tag) in tags.iter().enumerate() {
        let name_bytes = base64url_decode(&tag.name).map_err(|e| {
            AdapterError::validation_error(
                "build_signature_message",
                &format!("Invalid Base64URL in tag[{}].name: {}", i, e),
            )
        })?;
        let value_bytes = base64url_decode(&tag.value).map_err(|e| {
            AdapterError::validation_error(
                "build_signature_message",
                &format!("Invalid Base64URL in tag[{}].value: {}", i, e),
            )
        })?;
        tag_pairs.push(DeepHashItem::List(vec![
            DeepHashItem::Blob(name_bytes),
            DeepHashItem::Blob(value_bytes),
        ]));
    }

    // Build the complete signature message structure
    let structure = DeepHashItem::List(vec![
        DeepHashItem::Blob(b"2".to_vec()),
        DeepHashItem::Blob(owner.to_vec()),
        DeepHashItem::Blob(target.to_vec()),
        DeepHashItem::Blob(quantity.as_bytes().to_vec()),
        DeepHashItem::Blob(reward.as_bytes().to_vec()),
        DeepHashItem::Blob(last_tx.to_vec()),
        DeepHashItem::List(tag_pairs),
        DeepHashItem::Blob(data_size.as_bytes().to_vec()),
        DeepHashItem::Blob(data_root.to_vec()),
    ]);

    Ok(deep_hash(&structure))
}
