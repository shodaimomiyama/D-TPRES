//! Adapter layer error definitions
//!
//! Error types for adapter layer operations including storage,
//! serialization, connection errors, and AO Network communication errors.

use std::fmt;

use thiserror::Error;

use crate::domain::errors::DomainError;

/// Adapter layer error enumeration
///
/// Represents errors that occur in the adapter layer during
/// persistence and external system interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AdapterError {
    /// Storage operation failed
    StorageError { operation: String, details: String },

    /// Serialization/Deserialization failed
    SerializationError { operation: String, details: String },

    /// Entity not found in storage
    NotFound { entity_type: String, id: String },

    /// Connection to external system failed
    ConnectionError { details: String },

    /// Network operation failed with retry information
    NetworkError {
        context: String,
        message: String,
        retries_attempted: u32,
    },

    /// Configuration error
    ConfigurationError { context: String, message: String },

    /// Validation error
    ValidationError { context: String, message: String },

    /// Query error (GraphQL or similar)
    QueryError { context: String, message: String },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StorageError { operation, details } => {
                write!(f, "Storage error in operation '{operation}': {details}")
            }
            Self::SerializationError { operation, details } => {
                write!(
                    f,
                    "Serialization error in operation '{operation}': {details}"
                )
            }
            Self::NotFound { entity_type, id } => {
                write!(f, "{entity_type} not found: {id}")
            }
            Self::ConnectionError { details } => {
                write!(f, "Connection error: {details}")
            }
            Self::NetworkError {
                context,
                message,
                retries_attempted,
            } => {
                write!(
                    f,
                    "Network error in '{context}' after {retries_attempted} retries: {message}"
                )
            }
            Self::ConfigurationError { context, message } => {
                write!(f, "Configuration error in '{context}': {message}")
            }
            Self::ValidationError { context, message } => {
                write!(f, "Validation error in '{context}': {message}")
            }
            Self::QueryError { context, message } => {
                write!(f, "Query error in '{context}': {message}")
            }
        }
    }
}

impl std::error::Error for AdapterError {}

impl From<AdapterError> for DomainError {
    fn from(err: AdapterError) -> Self {
        match err {
            AdapterError::StorageError { operation, details } => {
                Self::StorageError { operation, details }
            }
            AdapterError::SerializationError { operation, details } => {
                Self::SerializationError { operation, details }
            }
            AdapterError::NotFound { entity_type, id } => Self::NotFound { entity_type, id },
            AdapterError::ConnectionError { details } => Self::StorageError {
                operation: "connection".to_string(),
                details,
            },
            AdapterError::NetworkError {
                context, message, ..
            } => Self::StorageError {
                operation: context,
                details: message,
            },
            AdapterError::ConfigurationError { context, message } => Self::StorageError {
                operation: format!("configuration:{context}"),
                details: message,
            },
            AdapterError::ValidationError { context, message } => Self::StorageError {
                operation: format!("validation:{context}"),
                details: message,
            },
            AdapterError::QueryError { context, message } => Self::StorageError {
                operation: format!("query:{context}"),
                details: message,
            },
        }
    }
}

/// Adapter error creation helpers
impl AdapterError {
    /// Create storage error
    pub fn storage_error(operation: &str, details: &str) -> Self {
        Self::StorageError {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }

    /// Create serialization error
    pub fn serialization_error(operation: &str, details: &str) -> Self {
        Self::SerializationError {
            operation: operation.to_string(),
            details: details.to_string(),
        }
    }

    /// Create not found error
    pub fn not_found(entity_type: &str, id: &str) -> Self {
        Self::NotFound {
            entity_type: entity_type.to_string(),
            id: id.to_string(),
        }
    }

    /// Create connection error
    pub fn connection_error(details: &str) -> Self {
        Self::ConnectionError {
            details: details.to_string(),
        }
    }

    /// Create network error
    pub fn network_error(context: &str, message: &str, retries_attempted: u32) -> Self {
        Self::NetworkError {
            context: context.to_string(),
            message: message.to_string(),
            retries_attempted,
        }
    }

    /// Create configuration error
    pub fn configuration_error(context: &str, message: &str) -> Self {
        Self::ConfigurationError {
            context: context.to_string(),
            message: message.to_string(),
        }
    }

    /// Create validation error
    pub fn validation_error(context: &str, message: &str) -> Self {
        Self::ValidationError {
            context: context.to_string(),
            message: message.to_string(),
        }
    }

    /// Create query error
    pub fn query_error(context: &str, message: &str) -> Self {
        Self::QueryError {
            context: context.to_string(),
            message: message.to_string(),
        }
    }
}

/// Result type alias for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;

/// AO Network communication error enumeration
///
/// Represents errors specific to AO Network communication including
/// connection issues, timeouts, and process-related errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum AOCommunicationError {
    /// Connection to AO Network failed
    #[error("Connection to AO Network failed: {details}")]
    ConnectionError { details: String },

    /// Request timed out
    #[error("Request timed out after {timeout_ms}ms: {operation}")]
    Timeout { operation: String, timeout_ms: u64 },

    /// AO Process not found
    #[error("AO Process not found: {process_id}")]
    ProcessNotFound { process_id: String },

    /// Invalid process ID format
    #[error("Invalid process ID format: {process_id}")]
    InvalidProcessId { process_id: String },

    /// Message serialization failed
    #[error("Serialization error: {details}")]
    SerializationError { details: String },

    /// Response deserialization failed
    #[error("Deserialization error: {details}")]
    DeserializationError { details: String },

    /// Message validation failed
    #[error("Validation error: {details}")]
    ValidationError { details: String },

    /// Insufficient cFrags collected for threshold
    #[error("Insufficient cFrags: collected {collected}, required {required}")]
    InsufficientCFrags { collected: usize, required: usize },

    /// Partial send failure (some processes failed)
    #[error("Partial send failure: {successful}/{total} processes succeeded")]
    PartialSendFailure {
        successful: usize,
        total: usize,
        failed_process_ids: Vec<String>,
    },

    /// Execution error from AO Process
    #[error("Execution error from process '{process_id}': {details}")]
    ExecutionError { process_id: String, details: String },
}

impl From<AOCommunicationError> for AdapterError {
    fn from(err: AOCommunicationError) -> Self {
        match err {
            AOCommunicationError::ConnectionError { details } => Self::ConnectionError { details },
            AOCommunicationError::Timeout {
                operation,
                timeout_ms,
            } => Self::ConnectionError {
                details: format!("Timeout after {timeout_ms}ms during {operation}"),
            },
            AOCommunicationError::ProcessNotFound { process_id } => Self::NotFound {
                entity_type: "AOProcess".to_string(),
                id: process_id,
            },
            AOCommunicationError::InvalidProcessId { process_id } => Self::StorageError {
                operation: "validate_process_id".to_string(),
                details: format!("Invalid process ID: {process_id}"),
            },
            AOCommunicationError::SerializationError { details } => Self::SerializationError {
                operation: "ao_message_serialize".to_string(),
                details,
            },
            AOCommunicationError::DeserializationError { details } => Self::SerializationError {
                operation: "ao_response_deserialize".to_string(),
                details,
            },
            AOCommunicationError::ValidationError { details } => Self::StorageError {
                operation: "ao_message_validate".to_string(),
                details,
            },
            AOCommunicationError::InsufficientCFrags {
                collected,
                required,
            } => Self::StorageError {
                operation: "collect_cfrags".to_string(),
                details: format!("Insufficient cFrags: {collected}/{required}"),
            },
            AOCommunicationError::PartialSendFailure {
                successful,
                total,
                failed_process_ids,
            } => Self::StorageError {
                operation: "broadcast_kfrag".to_string(),
                details: format!(
                    "Partial failure: {successful}/{total} succeeded, failed: {failed_process_ids:?}"
                ),
            },
            AOCommunicationError::ExecutionError {
                process_id,
                details,
            } => Self::StorageError {
                operation: format!("execute_on_{process_id}"),
                details,
            },
        }
    }
}

/// AO communication error creation helpers
impl AOCommunicationError {
    /// Create connection error
    pub fn connection_error(details: impl Into<String>) -> Self {
        Self::ConnectionError {
            details: details.into(),
        }
    }

    /// Create timeout error
    pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
        Self::Timeout {
            operation: operation.into(),
            timeout_ms,
        }
    }

    /// Create process not found error
    pub fn process_not_found(process_id: impl Into<String>) -> Self {
        Self::ProcessNotFound {
            process_id: process_id.into(),
        }
    }

    /// Create invalid process ID error
    pub fn invalid_process_id(process_id: impl Into<String>) -> Self {
        Self::InvalidProcessId {
            process_id: process_id.into(),
        }
    }

    /// Create serialization error
    pub fn serialization_error(details: impl Into<String>) -> Self {
        Self::SerializationError {
            details: details.into(),
        }
    }

    /// Create deserialization error
    pub fn deserialization_error(details: impl Into<String>) -> Self {
        Self::DeserializationError {
            details: details.into(),
        }
    }

    /// Create validation error
    pub fn validation_error(details: impl Into<String>) -> Self {
        Self::ValidationError {
            details: details.into(),
        }
    }

    /// Create insufficient cFrags error
    pub const fn insufficient_cfrags(collected: usize, required: usize) -> Self {
        Self::InsufficientCFrags {
            collected,
            required,
        }
    }

    /// Create partial send failure error
    pub const fn partial_send_failure(
        successful: usize,
        total: usize,
        failed_process_ids: Vec<String>,
    ) -> Self {
        Self::PartialSendFailure {
            successful,
            total,
            failed_process_ids,
        }
    }

    /// Create execution error
    pub fn execution_error(process_id: impl Into<String>, details: impl Into<String>) -> Self {
        Self::ExecutionError {
            process_id: process_id.into(),
            details: details.into(),
        }
    }
}

/// Result type alias for AO communication operations
pub type AOResult<T> = Result<T, AOCommunicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao_communication_error_variants() {
        // Verify all variants can be created
        let _ = AOCommunicationError::connection_error("test");
        let _ = AOCommunicationError::timeout("test_op", 5000);
        let _ = AOCommunicationError::process_not_found("process-123");
        let _ = AOCommunicationError::invalid_process_id("bad-id");
        let _ = AOCommunicationError::serialization_error("failed to serialize");
        let _ = AOCommunicationError::deserialization_error("failed to deserialize");
        let _ = AOCommunicationError::validation_error("invalid field");
        let _ = AOCommunicationError::insufficient_cfrags(2, 3);
        let _ = AOCommunicationError::partial_send_failure(2, 3, vec!["p1".to_string()]);
        let _ = AOCommunicationError::execution_error("process-1", "execution failed");
    }

    #[test]
    fn test_ao_error_display_trait() {
        let err = AOCommunicationError::connection_error("network down");
        assert_eq!(
            err.to_string(),
            "Connection to AO Network failed: network down"
        );

        let err = AOCommunicationError::timeout("query", 3000);
        assert_eq!(err.to_string(), "Request timed out after 3000ms: query");

        let err = AOCommunicationError::process_not_found("process-abc");
        assert_eq!(err.to_string(), "AO Process not found: process-abc");

        let err = AOCommunicationError::invalid_process_id("bad@id");
        assert_eq!(err.to_string(), "Invalid process ID format: bad@id");

        let err = AOCommunicationError::serialization_error("json error");
        assert_eq!(err.to_string(), "Serialization error: json error");

        let err = AOCommunicationError::deserialization_error("parse error");
        assert_eq!(err.to_string(), "Deserialization error: parse error");

        let err = AOCommunicationError::validation_error("missing field");
        assert_eq!(err.to_string(), "Validation error: missing field");

        let err = AOCommunicationError::insufficient_cfrags(2, 5);
        assert_eq!(
            err.to_string(),
            "Insufficient cFrags: collected 2, required 5"
        );

        let err = AOCommunicationError::partial_send_failure(
            3,
            5,
            vec!["p1".to_string(), "p2".to_string()],
        );
        assert_eq!(
            err.to_string(),
            "Partial send failure: 3/5 processes succeeded"
        );

        let err = AOCommunicationError::execution_error("holder-1", "reencryption failed");
        assert_eq!(
            err.to_string(),
            "Execution error from process 'holder-1': reencryption failed"
        );
    }

    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_ao_error_to_adapter_error_conversion() {
        let ao_err = AOCommunicationError::connection_error("network error");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::ConnectionError { details } if details == "network error")
        );

        let ao_err = AOCommunicationError::timeout("execute", 5000);
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::ConnectionError { details } if details.contains("5000ms"))
        );

        let ao_err = AOCommunicationError::process_not_found("proc-1");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::NotFound { entity_type, id } if entity_type == "AOProcess" && id == "proc-1")
        );

        let ao_err = AOCommunicationError::invalid_process_id("bad");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::StorageError { operation, .. } if operation == "validate_process_id")
        );

        let ao_err = AOCommunicationError::serialization_error("ser error");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::SerializationError { operation, .. } if operation == "ao_message_serialize")
        );

        let ao_err = AOCommunicationError::deserialization_error("deser error");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::SerializationError { operation, .. } if operation == "ao_response_deserialize")
        );

        let ao_err = AOCommunicationError::validation_error("val error");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::StorageError { operation, .. } if operation == "ao_message_validate")
        );

        let ao_err = AOCommunicationError::insufficient_cfrags(1, 3);
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::StorageError { operation, .. } if operation == "collect_cfrags")
        );

        let ao_err = AOCommunicationError::partial_send_failure(2, 3, vec!["p1".to_string()]);
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::StorageError { operation, .. } if operation == "broadcast_kfrag")
        );

        let ao_err = AOCommunicationError::execution_error("proc-1", "failed");
        let adapter_err: AdapterError = ao_err.into();
        assert!(
            matches!(adapter_err, AdapterError::StorageError { operation, .. } if operation.contains("proc-1"))
        );
    }

    #[test]
    fn test_ao_error_to_domain_error_conversion() {
        let ao_err = AOCommunicationError::connection_error("test");
        let adapter_err: AdapterError = ao_err.into();
        let domain_err: DomainError = adapter_err.into();
        assert!(matches!(domain_err, DomainError::StorageError { .. }));

        let ao_err = AOCommunicationError::process_not_found("proc-1");
        let adapter_err: AdapterError = ao_err.into();
        let domain_err: DomainError = adapter_err.into();
        assert!(
            matches!(domain_err, DomainError::NotFound { entity_type, .. } if entity_type == "AOProcess")
        );

        let ao_err = AOCommunicationError::serialization_error("error");
        let adapter_err: AdapterError = ao_err.into();
        let domain_err: DomainError = adapter_err.into();
        assert!(matches!(domain_err, DomainError::SerializationError { .. }));
    }

    #[test]
    fn test_ao_error_message_contains_details() {
        let err = AOCommunicationError::connection_error("specific network issue");
        assert!(err.to_string().contains("specific network issue"));

        let err = AOCommunicationError::timeout("long_operation", 10000);
        assert!(err.to_string().contains("long_operation"));
        assert!(err.to_string().contains("10000"));

        let err = AOCommunicationError::process_not_found("unique-process-id-123");
        assert!(err.to_string().contains("unique-process-id-123"));

        let err = AOCommunicationError::execution_error("holder-proc", "detailed error message");
        assert!(err.to_string().contains("holder-proc"));
        assert!(err.to_string().contains("detailed error message"));
    }

    #[test]
    fn test_ao_error_helper_methods() {
        // Test helper methods return correct variants
        let err = AOCommunicationError::connection_error("test");
        assert!(matches!(err, AOCommunicationError::ConnectionError { .. }));

        let err = AOCommunicationError::timeout("op", 100);
        assert!(
            matches!(err, AOCommunicationError::Timeout { operation, timeout_ms } if operation == "op" && timeout_ms == 100)
        );

        let err = AOCommunicationError::process_not_found("p1");
        assert!(
            matches!(err, AOCommunicationError::ProcessNotFound { process_id } if process_id == "p1")
        );

        let err = AOCommunicationError::invalid_process_id("bad");
        assert!(
            matches!(err, AOCommunicationError::InvalidProcessId { process_id } if process_id == "bad")
        );

        let err = AOCommunicationError::insufficient_cfrags(2, 5);
        assert!(matches!(
            err,
            AOCommunicationError::InsufficientCFrags {
                collected: 2,
                required: 5
            }
        ));

        let err = AOCommunicationError::partial_send_failure(1, 3, vec!["a".to_string()]);
        assert!(matches!(
            err,
            AOCommunicationError::PartialSendFailure {
                successful: 1,
                total: 3,
                ..
            }
        ));
    }

    #[test]
    fn test_ao_error_equality() {
        let err1 = AOCommunicationError::connection_error("test");
        let err2 = AOCommunicationError::connection_error("test");
        let err3 = AOCommunicationError::connection_error("other");

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);

        let err1 = AOCommunicationError::insufficient_cfrags(2, 3);
        let err2 = AOCommunicationError::insufficient_cfrags(2, 3);
        let err3 = AOCommunicationError::insufficient_cfrags(1, 3);

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_adapter_error_display_trait() {
        let err = AdapterError::storage_error("save", "disk full");
        assert_eq!(
            err.to_string(),
            "Storage error in operation 'save': disk full"
        );

        let err = AdapterError::serialization_error("encode", "invalid utf8");
        assert_eq!(
            err.to_string(),
            "Serialization error in operation 'encode': invalid utf8"
        );

        let err = AdapterError::not_found("KFrag", "kfrag-123");
        assert_eq!(err.to_string(), "KFrag not found: kfrag-123");

        let err = AdapterError::connection_error("timeout");
        assert_eq!(err.to_string(), "Connection error: timeout");
    }

    #[test]
    fn test_adapter_error_to_domain_error_conversion() {
        let err = AdapterError::storage_error("test", "error");
        let domain_err: DomainError = err.into();
        assert!(
            matches!(domain_err, DomainError::StorageError { operation, details } if operation == "test" && details == "error")
        );

        let err = AdapterError::serialization_error("test", "error");
        let domain_err: DomainError = err.into();
        assert!(
            matches!(domain_err, DomainError::SerializationError { operation, details } if operation == "test" && details == "error")
        );

        let err = AdapterError::not_found("Entity", "id-1");
        let domain_err: DomainError = err.into();
        assert!(
            matches!(domain_err, DomainError::NotFound { entity_type, id } if entity_type == "Entity" && id == "id-1")
        );

        let err = AdapterError::connection_error("network");
        let domain_err: DomainError = err.into();
        assert!(
            matches!(domain_err, DomainError::StorageError { operation, details } if operation == "connection" && details == "network")
        );
    }
}
