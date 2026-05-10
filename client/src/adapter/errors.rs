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

    #[error("RFC-9421 signing failed: {details}")]
    SigningError { details: String },

    #[error("TABM encoding failed: {details}")]
    TabmEncodingError { details: String },

    #[error("Wallet error: {details}")]
    WalletError { details: String },

    #[error("Multipart response parsing failed: {details}")]
    ResponseParsingError { details: String },
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
            AOCommunicationError::SigningError { details } => Self::StorageError {
                operation: "rfc9421_signing".to_string(),
                details,
            },
            AOCommunicationError::TabmEncodingError { details } => Self::SerializationError {
                operation: "tabm_encode".to_string(),
                details,
            },
            AOCommunicationError::WalletError { details } => Self::StorageError {
                operation: "wallet".to_string(),
                details,
            },
            AOCommunicationError::ResponseParsingError { details } => {
                Self::SerializationError {
                    operation: "response_parse".to_string(),
                    details,
                }
            }
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

    pub fn signing_error(details: impl Into<String>) -> Self {
        Self::SigningError {
            details: details.into(),
        }
    }

    pub fn tabm_encoding_error(details: impl Into<String>) -> Self {
        Self::TabmEncodingError {
            details: details.into(),
        }
    }

    pub fn wallet_error(details: impl Into<String>) -> Self {
        Self::WalletError {
            details: details.into(),
        }
    }

    pub fn response_parsing_error(details: impl Into<String>) -> Self {
        Self::ResponseParsingError {
            details: details.into(),
        }
    }
}

/// Result type alias for AO communication operations
pub type AOResult<T> = Result<T, AOCommunicationError>;
