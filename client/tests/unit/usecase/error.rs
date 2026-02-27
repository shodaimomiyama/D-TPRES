use formix::domain::DomainError;
use formix::usecase::error::{BusinessException, ServiceError, SystemException, WorkflowError};

#[test]
fn test_error_classification() {
    let business_err = ServiceError::validation_error("Invalid input");
    assert!(business_err.is_business_error());
    assert!(!business_err.is_system_error());

    let system_err = ServiceError::crypto_error("Encryption failed");
    assert!(!system_err.is_business_error());
    assert!(system_err.is_system_error());
}

#[test]
fn test_domain_error_conversion() {
    let domain_err = DomainError::EntityValidation {
        entity_type: "User".to_string(),
        field: "email".to_string(),
        message: "Invalid format".to_string(),
    };

    let service_err: ServiceError = domain_err.into();
    assert!(service_err.is_business_error());
}

#[test]
fn test_workflow_error_validation() {
    let err = WorkflowError::validation("Invalid threshold value");
    assert!(matches!(err, WorkflowError::ValidationError(_)));
    assert!(err.is_recoverable());

    let msg = err.to_string();
    assert!(msg.contains("Validation error"));
    assert!(msg.contains("Invalid threshold value"));
}

#[test]
fn test_workflow_error_from_crypto_service() {
    let service_err = ServiceError::crypto_error("Encryption failed");
    let workflow_err: WorkflowError = service_err.into();

    assert!(matches!(workflow_err, WorkflowError::CryptoError(_)));
    assert!(!workflow_err.is_recoverable());

    let msg = workflow_err.to_string();
    assert!(msg.contains("Crypto operation failed"));
}

#[test]
fn test_workflow_error_from_storage_service() {
    let service_err = ServiceError::storage_error("Arweave upload failed");
    let workflow_err: WorkflowError = service_err.into();

    assert!(matches!(workflow_err, WorkflowError::StorageError(_)));
    assert!(!workflow_err.is_recoverable());

    let msg = workflow_err.to_string();
    assert!(msg.contains("Storage operation failed"));
}

#[test]
fn test_workflow_error_ao_communication() {
    let err = WorkflowError::ao_communication("Failed to send kFrags to Owner-Process");
    assert!(matches!(err, WorkflowError::AOCommunicationError(_)));
    assert!(!err.is_recoverable());

    let msg = err.to_string();
    assert!(msg.contains("AO communication failed"));
    assert!(msg.contains("Failed to send kFrags"));

    let service_err = ServiceError::ao_network_error("Network timeout");
    let workflow_err: WorkflowError = service_err.into();
    assert!(matches!(
        workflow_err,
        WorkflowError::AOCommunicationError(_)
    ));
}

#[test]
fn test_workflow_error_insufficient_cfrags() {
    let err = WorkflowError::insufficient_cfrags(3, 2);
    assert!(matches!(
        err,
        WorkflowError::InsufficientCFrags {
            required: 3,
            actual: 2
        }
    ));
    assert!(err.is_recoverable());

    let msg = err.to_string();
    assert!(msg.contains("Insufficient cFrags"));
    assert!(msg.contains("need 3"));
    assert!(msg.contains("got 2"));

    let service_err = ServiceError::Business(BusinessException::ThresholdNotMet {
        required: 5,
        actual: 3,
    });
    let workflow_err: WorkflowError = service_err.into();
    assert!(matches!(
        workflow_err,
        WorkflowError::InsufficientCFrags {
            required: 5,
            actual: 3
        }
    ));
}

#[test]
fn test_workflow_error_decryption() {
    let err = WorkflowError::decryption("PRE decapsulation");
    assert!(matches!(err, WorkflowError::DecryptionError { .. }));
    assert!(!err.is_recoverable());

    let msg = err.to_string();
    assert!(msg.contains("Decryption failed"));
    assert!(msg.contains("PRE decapsulation"));

    let err2 = WorkflowError::DecryptionError {
        phase: "AES-GCM share decryption".to_string(),
    };
    let msg2 = err2.to_string();
    assert!(msg2.contains("AES-GCM share decryption"));
}

#[test]
fn test_workflow_error_resource_not_found() {
    let err = WorkflowError::not_found("Secret abc123 not found");
    assert!(matches!(err, WorkflowError::ResourceNotFound(_)));
    assert!(err.is_recoverable());

    let msg = err.to_string();
    assert!(msg.contains("Resource not found"));
    assert!(msg.contains("abc123"));

    let service_err = ServiceError::not_found("Capsule not found");
    let workflow_err: WorkflowError = service_err.into();
    assert!(matches!(workflow_err, WorkflowError::ResourceNotFound(_)));
}

#[test]
fn test_workflow_error_from_validation_business() {
    let service_err = ServiceError::validation_error("Invalid parameters");
    let workflow_err: WorkflowError = service_err.into();

    assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));
    assert!(workflow_err.is_recoverable());
}

#[test]
fn test_workflow_error_from_other_business() {
    let service_err = ServiceError::Business(BusinessException::AuthorizationError(
        "Not authorized".into(),
    ));
    let workflow_err: WorkflowError = service_err.into();

    assert!(matches!(workflow_err, WorkflowError::ValidationError(_)));
}

#[test]
fn test_workflow_error_from_other_system() {
    let service_err = ServiceError::System(SystemException::Network("Connection refused".into()));
    let workflow_err: WorkflowError = service_err.into();

    assert!(matches!(workflow_err, WorkflowError::StorageError(_)));
}

#[test]
fn test_workflow_error_is_recoverable() {
    assert!(WorkflowError::validation("test").is_recoverable());
    assert!(WorkflowError::not_found("test").is_recoverable());
    assert!(WorkflowError::insufficient_cfrags(3, 2).is_recoverable());

    assert!(!WorkflowError::crypto("test").is_recoverable());
    assert!(!WorkflowError::storage("test").is_recoverable());
    assert!(!WorkflowError::ao_communication("test").is_recoverable());
    assert!(!WorkflowError::decryption("test").is_recoverable());
    assert!(!WorkflowError::PartialStorageFailure {
        capsule_tx_id: "tx".to_string(),
        successful_share_tx_ids: vec![],
        failed_shares: vec![],
        failed_count: 0,
        total_count: 0,
    }
    .is_recoverable());
}

#[test]
fn test_workflow_error_partial_storage_failure() {
    let err = WorkflowError::PartialStorageFailure {
        capsule_tx_id: "tx_capsule_001".to_string(),
        successful_share_tx_ids: vec!["tx_share_0".to_string()],
        failed_shares: vec![("1".to_string(), "storage error".to_string())],
        failed_count: 1,
        total_count: 2,
    };

    let msg = err.to_string();
    assert!(msg.contains("Partial storage failure"));
    assert!(msg.contains("1 of 2"));
    assert!(!err.is_recoverable());

    match err {
        WorkflowError::PartialStorageFailure {
            capsule_tx_id,
            successful_share_tx_ids,
            failed_shares,
            failed_count,
            total_count,
        } => {
            assert_eq!(capsule_tx_id, "tx_capsule_001");
            assert_eq!(successful_share_tx_ids.len(), 1);
            assert_eq!(failed_shares.len(), 1);
            assert_eq!(failed_count, 1);
            assert_eq!(total_count, 2);
            assert_eq!(failed_shares[0].0, "1");
        }
        _ => panic!("Expected PartialStorageFailure"),
    }
}
