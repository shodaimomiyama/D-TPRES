//! Controller layer for FORMIX client library
//!
//! Provides input validation and DTO extraction for Actions layer.
//! The Controller layer sits between Actions and UseCase layers.

pub mod di;
pub mod error;
pub mod extractor;
pub mod validator;

pub use di::ControllerContainer;
pub use error::{error_codes, ValidationError, MAX_SHARES, MIN_THRESHOLD};
pub use extractor::{RecoverExtractor, ShareExtractor};
pub use validator::{RecoverValidator, ShareValidator};
