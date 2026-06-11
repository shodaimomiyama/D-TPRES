use formix::adapter::external::ao::{
    AOExecuteMsg, AONativeResponse, AOQueryMsg, Binary, GetCFragResponse,
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
fn test_ao_execute_msg_delegate_kfrag() {
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-001", &[1, 2, 3, 4], "holder-1");
    assert_eq!(msg.action(), "DelegateKFrag");
    assert_eq!(msg.data()["kfrag_id"].as_str(), Some("kfrag-001"));
    assert_eq!(msg.data()["holder_process_id"].as_str(), Some("holder-1"));
}

#[test]
fn test_ao_execute_msg_delegate_capsule() {
    let msg = AOExecuteMsg::delegate_capsule("kfrag-001", "capsule-001", &[5, 6, 7, 8], "holder-1");
    assert_eq!(msg.action(), "DelegateCapsule");
    assert_eq!(msg.data()["capsule_id"].as_str(), Some("capsule-001"));
    assert_eq!(msg.data()["holder_process_id"].as_str(), Some("holder-1"));
}

#[test]
fn test_ao_execute_msg_reencrypt() {
    let msg = AOExecuteMsg::reencrypt("kfrag-001", "capsule-001");
    assert_eq!(msg.action(), "Reencrypt");
    assert_eq!(msg.data()["kfrag_id"].as_str(), Some("kfrag-001"));
    assert_eq!(msg.data()["capsule_id"].as_str(), Some("capsule-001"));
}

#[test]
fn test_ao_execute_msg_init() {
    let msg = AOExecuteMsg::init("Owner");
    assert_eq!(msg.action(), "Init");
    assert_eq!(msg.data()["role"].as_str(), Some("Owner"));
}

#[test]
fn test_ao_query_msg_get_cfrag() {
    let msg = AOQueryMsg::get_cfrag("kfrag-001", "capsule-001");
    assert_eq!(msg.action(), "GetCFrag");
    assert_eq!(msg.data()["kfrag_id"].as_str(), Some("kfrag-001"));
    assert_eq!(msg.data()["capsule_id"].as_str(), Some("capsule-001"));
}

#[test]
fn test_ao_query_msg_list_capsules() {
    let msg = AOQueryMsg::list_capsules_by_kfrag("kfrag-001", Some("cap-000"), Some(10));
    assert_eq!(msg.action(), "ListCapsules");
    assert_eq!(msg.data()["kfrag_id"].as_str(), Some("kfrag-001"));
    assert_eq!(msg.data()["start_after"].as_str(), Some("cap-000"));
}

#[test]
fn test_ao_native_response_success() {
    let response = AONativeResponse::success(serde_json::json!({"key": "value"}));
    assert!(response.ok);
    assert!(response.data.is_some());
    assert!(response.error.is_none());
}

#[test]
fn test_ao_native_response_error() {
    let response = AONativeResponse::error_response("something failed");
    assert!(!response.ok);
    assert!(response.data.is_none());
    assert_eq!(response.error, Some("something failed".to_string()));
}

#[test]
fn test_get_cfrag_response_serialization() {
    let response = GetCFragResponse {
        kfrag_id: "kfrag-001".to_string(),
        capsule_id: "capsule-001".to_string(),
        cfrag: vec![1, 2, 3, 4],
    };
    let json = serde_json::to_string(&response).unwrap();
    let deserialized: GetCFragResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.kfrag_id, "kfrag-001");
    assert_eq!(deserialized.cfrag, vec![1, 2, 3, 4]);
}

#[test]
fn test_ao_execute_msg_serialization_roundtrip() {
    let msg = AOExecuteMsg::delegate_kfrag("kfrag-001", &[1, 2, 3], "holder-1");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("DelegateKFrag"));
    let deserialized: AOExecuteMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.action(), "DelegateKFrag");
}

#[test]
fn test_ao_query_msg_serialization_roundtrip() {
    let msg = AOQueryMsg::get_cfrag("kfrag-001", "capsule-001");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("GetCFrag"));
    let deserialized: AOQueryMsg = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.action(), "GetCFrag");
}
