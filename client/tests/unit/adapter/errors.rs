use formix::adapter::errors::{AOCommunicationError, AdapterError};
use formix::domain::DomainError;

#[test]
fn test_ao_communication_error_variants() {
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

    let err =
        AOCommunicationError::partial_send_failure(3, 5, vec!["p1".to_string(), "p2".to_string()]);
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
