//! Option structs for Actions layer functions
//!
//! Provides ShareOptions and RecoverOptions to separate optional parameters
//! from required ones in share() and recover() functions.

use crate::usecase::dto::SecretMetadata;

/// Optional parameters for share() function
///
/// Separates optional parameters from required ones for better API usability.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ShareOptions {
    /// Optional metadata for the secret
    pub metadata: Option<SecretMetadata>,
}

impl ShareOptions {
    /// Create new ShareOptions with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create ShareOptions with metadata
    pub const fn with_metadata(metadata: SecretMetadata) -> Self {
        Self {
            metadata: Some(metadata),
        }
    }

    /// Builder method to set metadata
    #[must_use]
    pub fn metadata(mut self, metadata: SecretMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Optional parameters for recover() function
///
/// Reserved for future extensions. Currently empty.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RecoverOptions {
    // Reserved for future extensions
}

impl RecoverOptions {
    /// Create new RecoverOptions with default values
    pub fn new() -> Self {
        Self::default()
    }
}
