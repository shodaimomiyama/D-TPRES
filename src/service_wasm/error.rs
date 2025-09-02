//! Error types for WASM service layer

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Crypto operation not supported in WASM: {0}")]
    WasmNotSupported(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub type ServiceResult<T> = Result<T, ServiceError>;

impl ServiceError {
    pub fn not_supported(msg: &str) -> Self {
        ServiceError::WasmNotSupported(msg.to_string())
    }
    
    pub fn validation_error(msg: &str) -> Self {
        ServiceError::ValidationError(msg.to_string())
    }
    
    pub fn serialization_error(msg: &str) -> Self {
        ServiceError::SerializationError(msg.to_string())
    }
}