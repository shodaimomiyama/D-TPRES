//! Controller layer validation error definitions
//!
//! Defines ValidationError for early input validation before DTO construction.
//! ValidationError can be converted to WorkflowError for unified error handling.

use std::fmt;

use crate::usecase::error::WorkflowError;

pub use crate::usecase::core::crypto::constants::{MAX_SHARES, MIN_THRESHOLD};

/// Error codes for validation failures
pub mod error_codes {
    /// Secret data is empty
    pub const SECRET_EMPTY: &str = "secret_empty";

    /// Threshold value is zero or invalid
    pub const INVALID_THRESHOLD: &str = "invalid_threshold";

    /// Threshold exceeds total shares (k > n)
    pub const THRESHOLD_EXCEEDS_TOTAL: &str = "threshold_exceeds_total";

    /// Total shares exceeds maximum allowed
    pub const TOTAL_SHARES_EXCEEDS_MAX: &str = "total_shares_exceeds_max";

    /// Threshold is below minimum required
    pub const THRESHOLD_BELOW_MIN: &str = "threshold_below_min";

    /// Owner key is invalid or empty
    pub const INVALID_OWNER_KEY: &str = "invalid_owner_key";

    /// Requester key is invalid or empty
    pub const INVALID_REQUESTER_KEY: &str = "invalid_requester_key";

    /// Secret ID is invalid or empty
    pub const INVALID_SECRET_ID: &str = "invalid_secret_id";

    /// Process ID is invalid or empty
    pub const INVALID_PROCESS_ID: &str = "invalid_process_id";

    /// Owner public key is invalid or empty
    pub const INVALID_OWNER_PUBLIC_KEY: &str = "invalid_owner_public_key";

    /// Owner secret key and public key do not match
    pub const KEY_MISMATCH: &str = "key_mismatch";
}

/// Controller layer validation error
///
/// Represents early validation errors before DTO construction.
/// Can be converted to WorkflowError for unified error handling in the service layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    code: String,
    message: String,
    field: Option<String>,
}

impl ValidationError {
    /// Create a new ValidationError with code and message
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }

    /// Create a ValidationError with field name
    pub fn with_field(
        code: impl Into<String>,
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: Some(field.into()),
        }
    }

    /// Get the error code
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Get the error message
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get the field name if present
    #[must_use]
    pub fn field(&self) -> Option<&str> {
        self.field.as_deref()
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = &self.code;
        let message = &self.message;
        if let Some(field) = &self.field {
            write!(f, "[{code}] {field}: {message}")
        } else {
            write!(f, "[{code}] {message}")
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<ValidationError> for WorkflowError {
    fn from(err: ValidationError) -> Self {
        let code = &err.code;
        let message = &err.message;
        let msg = match &err.field {
            Some(field) => format!("[{code}] {field}: {message}"),
            None => format!("[{code}] {message}"),
        };
        WorkflowError::ValidationError(msg)
    }
}
