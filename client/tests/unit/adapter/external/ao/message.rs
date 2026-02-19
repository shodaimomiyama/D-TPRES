use formix::adapter::external::ao::{
    AOEvent, AOMessageTags, AOResponse, Binary, CapsuleStatus, ExecuteMsg, QueryMsg,
    ValidateMessage,
};

#[test]
fn test_binary_creation() {
    let data = vec![1, 2, 3, 4, 5];
    let binary = Binary::new(data.clone());
    assert_eq!(binary.len(), 5);
    assert!(!binary.is_empty());
    assert_eq!(binary.as_slice(), &data);
}

#[test]
fn test_binary_from_vec() {
    let data = vec![1, 2, 3];
    let binary: Binary = data.clone().into();
    assert_eq!(binary.into_vec(), data);
}

#[test]
fn test_binary_serialization() {
    let binary = Binary::new(vec![1, 2, 3, 4]);
    let json = serde_json::to_string(&binary).unwrap();
    let deserialized: Binary = serde_json::from_str(&json).unwrap();
    assert_eq!(binary, deserialized);
}

#[test]
fn test_execute_msg_delegate_kfrag_serialization() {
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kfrag-001".to_string(),
        kfrag: Binary::new(vec![1, 2, 3, 4]),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("kfrag_id"));

    let deserialized: ExecuteMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn test_execute_msg_delegate_capsule_serialization() {
    let msg = ExecuteMsg::DelegateCapsule {
        kfrag_id: "kfrag-001".to_string(),
        capsule_id: "capsule-001".to_string(),
        capsule: Binary::new(vec![5, 6, 7, 8]),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("capsule_id"));

    let deserialized: ExecuteMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn test_query_msg_get_cfrag_serialization() {
    let msg = QueryMsg::GetCFrag {
        kfrag_id: "kfrag-001".to_string(),
        capsule_id: "capsule-001".to_string(),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("kfrag_id"));

    let deserialized: QueryMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn test_query_msg_list_capsules_serialization() {
    let msg = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "kfrag-001".to_string(),
        start_after: Some("capsule-000".to_string()),
        limit: Some(10),
    };

    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("kfrag_id"));

    let deserialized: QueryMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(msg, deserialized);
}

#[test]
fn test_validate_execute_msg_delegate_kfrag() {
    let valid_msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "valid-kfrag-id".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };
    assert!(valid_msg.validate().is_ok());

    let empty_id_msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "".to_string(),
        kfrag: Binary::new(vec![1, 2, 3]),
    };
    assert!(empty_id_msg.validate().is_err());

    let empty_data_msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "valid-kfrag-id".to_string(),
        kfrag: Binary::new(vec![]),
    };
    assert!(empty_data_msg.validate().is_err());
}

#[test]
fn test_validate_query_msg_list_capsules() {
    let valid_msg = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "valid-kfrag-id".to_string(),
        start_after: None,
        limit: Some(50),
    };
    assert!(valid_msg.validate().is_ok());

    let invalid_limit_zero = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "valid-kfrag-id".to_string(),
        start_after: None,
        limit: Some(0),
    };
    assert!(invalid_limit_zero.validate().is_err());

    let invalid_limit_over = QueryMsg::ListCapsulesByKFrag {
        kfrag_id: "valid-kfrag-id".to_string(),
        start_after: None,
        limit: Some(101),
    };
    assert!(invalid_limit_over.validate().is_err());
}

#[test]
fn test_ao_response_creation() {
    let response = AOResponse::success();
    assert!(response.success);
    assert!(response.data.is_none());
    assert!(response.events.is_empty());

    let response_with_data = AOResponse::success_with_data(Binary::new(vec![1, 2, 3]));
    assert!(response_with_data.success);
    assert!(response_with_data.data.is_some());

    let event = AOEvent::new("kfrag_delegated");
    let response_with_events = AOResponse::success_with_events(vec![event]);
    assert_eq!(response_with_events.events.len(), 1);
}

#[test]
fn test_ao_event_creation() {
    let mut event = AOEvent::new("test_event");
    event.add_attribute("key1", "value1");
    event.add_attribute("key2", "value2");

    assert_eq!(event.event_type, "test_event");
    assert_eq!(event.attributes.len(), 2);
    assert_eq!(event.attributes[0].key, "key1");
    assert_eq!(event.attributes[0].value, "value1");
}

#[test]
fn test_ao_event_with_attributes() {
    let event = AOEvent::with_attributes(
        "kfrag_delegated",
        vec![("kfrag_id", "kfrag-001"), ("process_id", "proc-001")],
    );

    assert_eq!(event.event_type, "kfrag_delegated");
    assert_eq!(event.attributes.len(), 2);
}

#[test]
fn test_ao_message_tags_execute() {
    let tags = AOMessageTags::new_execute(
        "DelegateKFrag",
        r#"{"kfrag_id":"test"}"#,
        "process-001",
        "actor-001",
        "2024-01-01T00:00:00Z",
    );

    assert_eq!(tags.app_name, "cwao");
    assert_eq!(tags.action, "DelegateKFrag");
    assert_eq!(tags.read_only, "False");
    assert!(!tags.is_read_only());
}

#[test]
fn test_ao_message_tags_query() {
    let tags = AOMessageTags::new_query(
        "GetCFrag",
        r#"{"kfrag_id":"test"}"#,
        "process-001",
        "actor-001",
        "2024-01-01T00:00:00Z",
    );

    assert_eq!(tags.app_name, "cwao");
    assert_eq!(tags.action, "GetCFrag");
    assert_eq!(tags.read_only, "True");
    assert!(tags.is_read_only());
}

#[test]
fn test_capsule_status_serialization() {
    let status = CapsuleStatus::Pending;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"pending\"");

    let status = CapsuleStatus::Reencrypted;
    let json = serde_json::to_string(&status).unwrap();
    assert_eq!(json, "\"reencrypted\"");
}
