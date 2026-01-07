//! Adapter layer error definitions
//!
//! Error types for adapter layer operations including storage,
//! serialization, and connection errors.

use std::fmt;

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
