use formix::actions::error::ActionError;
use formix::controller::error::ValidationError;
use formix::usecase::error::WorkflowError;

#[test]
fn test_action_error_validation_failed_constructor() {
    let err = ActionError::validation_failed("test_code", "test message");
    match err {
        ActionError::ValidationFailed { code, message } => {
            assert_eq!(code, "test_code");
            assert_eq!(message, "test message");
        }
        _ => panic!("Expected ValidationFailed"),
    }
}

#[test]
fn test_action_error_workflow_failed_constructor() {
    let err = ActionError::workflow_failed("workflow error");
    match err {
        ActionError::WorkflowFailed { message } => {
            assert_eq!(message, "workflow error");
        }
        _ => panic!("Expected WorkflowFailed"),
    }
}

#[test]
fn test_action_error_resource_not_found_constructor() {
    let err = ActionError::resource_not_found("secret");
    match err {
        ActionError::ResourceNotFound { resource } => {
            assert_eq!(resource, "secret");
        }
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[test]
fn test_action_error_crypto_error_constructor() {
    let err = ActionError::crypto_error("crypto failed");
    match err {
        ActionError::CryptoError { message } => {
            assert_eq!(message, "crypto failed");
        }
        _ => panic!("Expected CryptoError"),
    }
}

#[test]
fn test_action_error_display_validation_failed() {
    let err = ActionError::validation_failed("test", "Test error");
    let display = err.to_string();
    assert!(display.contains("test"));
    assert!(display.contains("Test error"));
    assert!(display.contains("Validation failed"));
}

#[test]
fn test_action_error_display_workflow_failed() {
    let err = ActionError::workflow_failed("workflow error");
    let display = err.to_string();
    assert!(display.contains("Workflow failed"));
    assert!(display.contains("workflow error"));
}

#[test]
fn test_action_error_display_resource_not_found() {
    let err = ActionError::resource_not_found("secret");
    let display = err.to_string();
    assert!(display.contains("Resource not found"));
    assert!(display.contains("secret"));
}

#[test]
fn test_action_error_display_crypto_error() {
    let err = ActionError::crypto_error("crypto failed");
    let display = err.to_string();
    assert!(display.contains("Crypto error"));
    assert!(display.contains("crypto failed"));
}

#[test]
fn test_action_error_from_validation_error() {
    let val_err = ValidationError::new("test_code", "test message");
    let action_err: ActionError = val_err.into();

    match action_err {
        ActionError::ValidationFailed { code, message } => {
            assert_eq!(code, "test_code");
            assert_eq!(message, "test message");
        }
        _ => panic!("Expected ValidationFailed"),
    }
}

#[test]
fn test_action_error_from_workflow_error_validation() {
    let workflow_err = WorkflowError::ValidationError("validation failed".to_string());
    let action_err: ActionError = workflow_err.into();

    match action_err {
        ActionError::ValidationFailed { code, message } => {
            assert_eq!(code, "workflow_validation");
            assert_eq!(message, "validation failed");
        }
        _ => panic!("Expected ValidationFailed"),
    }
}

#[test]
fn test_action_error_from_workflow_error_resource_not_found() {
    let workflow_err = WorkflowError::ResourceNotFound("secret".to_string());
    let action_err: ActionError = workflow_err.into();

    match action_err {
        ActionError::ResourceNotFound { resource } => {
            assert_eq!(resource, "secret");
        }
        _ => panic!("Expected ResourceNotFound"),
    }
}

#[test]
fn test_action_error_from_workflow_error_crypto() {
    let workflow_err = WorkflowError::CryptoError("crypto failed".to_string());
    let action_err: ActionError = workflow_err.into();

    match action_err {
        ActionError::CryptoError { message } => {
            assert_eq!(message, "crypto failed");
        }
        _ => panic!("Expected CryptoError"),
    }
}

#[test]
fn test_action_error_from_workflow_error_partial_storage_failure() {
    let workflow_err = WorkflowError::PartialStorageFailure {
        capsule_tx_id: "tx_capsule_001".to_string(),
        successful_share_tx_ids: vec!["tx_share_0".to_string(), "tx_share_2".to_string()],
        failed_shares: vec![
            ("1".to_string(), "storage error".to_string()),
            ("3".to_string(), "timeout".to_string()),
        ],
        failed_count: 2,
        total_count: 4,
    };
    let action_err: ActionError = workflow_err.into();

    match action_err {
        ActionError::PartialStorageFailure {
            capsule_tx_id,
            successful_share_tx_ids,
            failed_shares,
            message,
        } => {
            assert_eq!(capsule_tx_id, "tx_capsule_001");
            assert_eq!(successful_share_tx_ids.len(), 2);
            assert_eq!(failed_shares.len(), 2);
            assert!(message.contains("2 of 4"));
        }
        _ => panic!("Expected PartialStorageFailure"),
    }
}

#[test]
fn test_action_error_display_partial_storage_failure() {
    let err = ActionError::PartialStorageFailure {
        capsule_tx_id: "tx_001".to_string(),
        successful_share_tx_ids: vec!["tx_s1".to_string()],
        failed_shares: vec![("1".to_string(), "err".to_string())],
        message: "1 of 2 share storage operations failed".to_string(),
    };
    let display = err.to_string();
    assert!(display.contains("Partial storage failure"));
    assert!(display.contains("1 of 2"));
}
