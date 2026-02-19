use formix::controller::{MAX_SHARES, MIN_THRESHOLD, ValidationError, error_codes};
use formix::usecase::error::WorkflowError;

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
