//! SymmetricKey Value Object
//!
//! Represents an AES-256 symmetric key for encrypting Shamir shares.
//! Key material is zeroized on drop for memory safety.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// Size of AES-256 key in bytes
pub const SYMMETRIC_KEY_SIZE: usize = 32;

/// AES-256 symmetric key value object
///
/// Holds a 32-byte AES-256 key (kₒ) used to encrypt Shamir shares.
/// Key material is automatically zeroized when dropped.
///
/// # Security
/// - Implements Zeroize/ZeroizeOnDrop for key material
/// - Does not implement Clone to prevent accidental copying
/// - Debug output redacts the key
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SymmetricKey {
    key: [u8; SYMMETRIC_KEY_SIZE],
}

impl SymmetricKey {
    /// Create a new SymmetricKey from raw bytes
    ///
    /// # Arguments
    /// * `key` - 32-byte AES-256 key
    ///
    /// # Returns
    /// New SymmetricKey instance
    pub const fn new(key: [u8; SYMMETRIC_KEY_SIZE]) -> Self {
        Self { key }
    }

    /// Create from a byte slice
    ///
    /// # Arguments
    /// * `bytes` - Byte slice of exactly 32 bytes
    ///
    /// # Returns
    /// `Some(SymmetricKey)` if slice is 32 bytes, `None` otherwise
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != SYMMETRIC_KEY_SIZE {
            return None;
        }
        let mut key = [0u8; SYMMETRIC_KEY_SIZE];
        key.copy_from_slice(bytes);
        Some(Self { key })
    }

    /// Get the key bytes
    ///
    /// # Security
    /// Handle returned bytes carefully - they contain key material
    pub const fn as_bytes(&self) -> &[u8; SYMMETRIC_KEY_SIZE] {
        &self.key
    }
}

impl std::fmt::Debug for SymmetricKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SymmetricKey")
            .field("key", &"[REDACTED]")
            .finish()
    }
}
