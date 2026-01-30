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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_code_message() {
        let err = ValidationError::new("test_code", "test message");
        assert_eq!(err.code(), "test_code");
        assert_eq!(err.message(), "test message");
        assert!(err.field().is_none());
    }

    #[test]
    fn test_validation_error_with_field() {
        let err = ValidationError::with_field("test_code", "test message", "field_name");
        assert_eq!(err.code(), "test_code");
        assert_eq!(err.message(), "test message");
        assert_eq!(err.field(), Some("field_name"));
    }

    #[test]
    fn test_validation_error_to_workflow_error() {
        let val_err = ValidationError::new("test_code", "test message");
        let workflow_err: WorkflowError = val_err.into();
        assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));

        let msg = workflow_err.to_string();
        assert!(msg.contains("[test_code]"));
        assert!(msg.contains("test message"));
    }

    #[test]
    fn test_validation_error_to_workflow_error_with_field() {
        let val_err = ValidationError::with_field("test_code", "test message", "field_name");
        let workflow_err: WorkflowError = val_err.into();
        assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));

        let msg = workflow_err.to_string();
        assert!(msg.contains("[test_code]"));
        assert!(msg.contains("field_name"));
        assert!(msg.contains("test message"));
    }

    #[test]
    fn test_validation_error_display() {
        let err_without_field = ValidationError::new("test_code", "test message");
        let display = err_without_field.to_string();
        assert_eq!(display, "[test_code] test message");

        let err_with_field = ValidationError::with_field("test_code", "test message", "field_name");
        let display_with_field = err_with_field.to_string();
        assert_eq!(display_with_field, "[test_code] field_name: test message");
    }

    #[test]
    fn test_error_codes_constants() {
        assert_eq!(error_codes::SECRET_EMPTY, "secret_empty");
        assert_eq!(error_codes::INVALID_THRESHOLD, "invalid_threshold");
        assert_eq!(
            error_codes::THRESHOLD_EXCEEDS_TOTAL,
            "threshold_exceeds_total"
        );
        assert_eq!(
            error_codes::TOTAL_SHARES_EXCEEDS_MAX,
            "total_shares_exceeds_max"
        );
        assert_eq!(error_codes::THRESHOLD_BELOW_MIN, "threshold_below_min");
        assert_eq!(error_codes::INVALID_OWNER_KEY, "invalid_owner_key");
        assert_eq!(error_codes::INVALID_REQUESTER_KEY, "invalid_requester_key");
        assert_eq!(error_codes::INVALID_SECRET_ID, "invalid_secret_id");
        assert_eq!(error_codes::INVALID_PROCESS_ID, "invalid_process_id");
    }

    #[test]
    fn test_validation_constants() {
        assert_eq!(MIN_THRESHOLD, 2);
        assert_eq!(MAX_SHARES, 20);
    }

    #[test]
    fn test_validation_error_clone_eq() {
        let err1 = ValidationError::new("code", "message");
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }
}
