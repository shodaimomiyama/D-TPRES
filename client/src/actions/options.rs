//! Option structs for Actions layer functions
//!
//! Provides ShareOptions and RecoverOptions to separate optional parameters
//! from required ones in share() and recover() functions.

use crate::usecase::dto::SecretMetadata;

/// Optional parameters for share() function
///
/// Separates optional parameters from required ones for better API usability.
#[derive(Debug, Clone, Default)]
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
    pub fn with_metadata(metadata: SecretMetadata) -> Self {
        Self {
            metadata: Some(metadata),
        }
    }

    /// Builder method to set metadata
    pub fn metadata(mut self, metadata: SecretMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Optional parameters for recover() function
///
/// Reserved for future extensions. Currently empty.
#[derive(Debug, Clone, Default)]
pub struct RecoverOptions {
    // Reserved for future extensions
}

impl RecoverOptions {
    /// Create new RecoverOptions with default values
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_options_default() {
        let options = ShareOptions::default();
        assert!(options.metadata.is_none());
    }

    #[test]
    fn test_share_options_new() {
        let options = ShareOptions::new();
        assert!(options.metadata.is_none());
    }

    #[test]
    fn test_share_options_with_metadata() {
        let metadata = SecretMetadata {
            name: Some("test".to_string()),
            description: None,
            expires_at: None,
            tags: vec![],
        };
        let options = ShareOptions::with_metadata(metadata);
        assert!(options.metadata.is_some());
        assert_eq!(options.metadata.unwrap().name.unwrap(), "test");
    }

    #[test]
    fn test_share_options_builder_metadata() {
        let metadata = SecretMetadata {
            name: Some("builder_test".to_string()),
            description: Some("test description".to_string()),
            expires_at: Some(12345),
            tags: vec!["tag1".to_string()],
        };
        let options = ShareOptions::new().metadata(metadata);
        assert!(options.metadata.is_some());
        let m = options.metadata.unwrap();
        assert_eq!(m.name.unwrap(), "builder_test");
        assert_eq!(m.description.unwrap(), "test description");
        assert_eq!(m.expires_at.unwrap(), 12345);
        assert_eq!(m.tags.len(), 1);
    }

    #[test]
    fn test_recover_options_default() {
        let options = RecoverOptions::default();
        let _ = options; // just to confirm it compiles
    }

    #[test]
    fn test_recover_options_new() {
        let options = RecoverOptions::new();
        let _ = options;
    }
}
