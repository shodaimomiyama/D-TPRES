//! Controller layer dependency injection container
//!
//! Provides centralized access to Controller layer components.

use std::sync::Arc;

use crate::controller::extractor::{RecoverExtractor, ShareExtractor};
use crate::controller::validator::{RecoverValidator, ShareValidator};
use crate::usecase::core::crypto::CryptoService;

/// Container for Controller layer components
///
/// Aggregates all Controller layer components (validators and extractors)
/// for simplified access from the Actions layer.
pub struct ControllerContainer<C: CryptoService> {
    share_validator: ShareValidator<C>,
    recover_validator: RecoverValidator,
    share_extractor: ShareExtractor,
    recover_extractor: RecoverExtractor,
}

impl<C: CryptoService> ControllerContainer<C> {
    /// Create a new ControllerContainer with CryptoService for key validation
    pub fn new(crypto_service: Arc<C>) -> Self {
        Self {
            share_validator: ShareValidator::new(crypto_service),
            recover_validator: RecoverValidator::new(),
            share_extractor: ShareExtractor::new(),
            recover_extractor: RecoverExtractor::new(),
        }
    }

    /// Get reference to ShareValidator
    pub fn share_validator(&self) -> &ShareValidator<C> {
        &self.share_validator
    }

    /// Get reference to RecoverValidator
    pub fn recover_validator(&self) -> &RecoverValidator {
        &self.recover_validator
    }

    /// Get reference to ShareExtractor
    pub fn share_extractor(&self) -> &ShareExtractor {
        &self.share_extractor
    }

    /// Get reference to RecoverExtractor
    pub fn recover_extractor(&self) -> &RecoverExtractor {
        &self.recover_extractor
    }
}
