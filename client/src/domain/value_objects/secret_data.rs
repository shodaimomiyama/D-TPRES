//! SecretData Value Object
//!
//! Represents the actual secret data f(0)=secret.
//! This is a temporary value used during Shamir split operation
//! and must be zeroized after use.

use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::domain::errors::DomainError;

/// Secret data value object containing f(0)=secret
///
/// # Security
/// - Implements Zeroize to clear memory on drop
/// - Does NOT implement Clone to prevent accidental copies
/// - Should be used only temporarily during secret splitting
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct SecretData {
    secret_bytes: Vec<u8>,
}

impl SecretData {
    /// Create a new SecretData from byte vector
    ///
    /// # Errors
    /// Returns `DomainError::EntityValidation` if bytes are empty
    pub fn new(bytes: Vec<u8>) -> Result<Self, DomainError> {
        if bytes.is_empty() {
            return Err(DomainError::EntityValidation {
                entity_type: "SecretData".to_string(),
                field: "secret_bytes".to_string(),
                message: "Secret data cannot be empty".to_string(),
            });
        }
        Ok(Self {
            secret_bytes: bytes,
        })
    }

    /// Get reference to the secret bytes
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.secret_bytes
    }

    /// Get the length of the secret data
    #[allow(clippy::missing_const_for_fn)]
    pub fn len(&self) -> usize {
        self.secret_bytes.len()
    }

    /// Check if the secret data is empty
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_empty(&self) -> bool {
        self.secret_bytes.is_empty()
    }
}

impl std::fmt::Debug for SecretData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretData")
            .field("len", &self.secret_bytes.len())
            .field("secret_bytes", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_data_new_valid() {
        let data = vec![1, 2, 3, 4, 5];
        let secret = SecretData::new(data).unwrap();
        assert_eq!(secret.as_bytes(), &[1, 2, 3, 4, 5]);
        assert_eq!(secret.len(), 5);
        assert!(!secret.is_empty());
    }

    #[test]
    fn test_secret_data_new_empty_error() {
        let result = SecretData::new(vec![]);
        assert!(result.is_err());
        if let Err(DomainError::EntityValidation { field, .. }) = result {
            assert_eq!(field, "secret_bytes");
        } else {
            panic!("Expected EntityValidation error");
        }
    }

    #[test]
    fn test_secret_data_debug_redacted() {
        let secret = SecretData::new(vec![1, 2, 3]).unwrap();
        let debug_str = format!("{:?}", secret);
        assert!(debug_str.contains("REDACTED"));
        assert!(!debug_str.contains("1"));
    }
}
