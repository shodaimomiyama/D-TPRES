#![allow(
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::uninlined_format_args
)]

use formix::adapter::errors::AOCommunicationError;
use formix::adapter::external::ao::{
    AOClient, Binary, ExecuteMsg, GetCFragResponse, ListCapsulesByKFragResponse, QueryMsg,
};
use formix::adapter::external::mock_ao::MockAOClient;

#[tokio::test]
async fn test_mock_ao_client_new() {
    let client = MockAOClient::new();
    assert!(client.get_stored_kfrags("test").is_empty());
}

#[tokio::test]
async fn test_execute_delegate_kfrag() {
    let client = MockAOClient::new();
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kfrag-1".to_string(),
        kfrag: Binary::from(vec![1, 2, 3, 4]),
    };

    let result = client.execute("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].event_type, "delegate_kfrag");

    let stored = client.get_stored_kfrags("process-1");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, "kfrag-1");
    assert_eq!(stored[0].1, vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn test_execute_delegate_capsule() {
    let client = MockAOClient::new();
    let msg = ExecuteMsg::DelegateCapsule {
        kfrag_id: "kfrag-1".to_string(),
        capsule_id: "capsule-1".to_string(),
        capsule: Binary::from(vec![5, 6, 7, 8]),
    };

    let result = client.execute("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.events[0].event_type, "delegate_capsule");

    let stored = client.get_stored_capsules("process-1");
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].0, "capsule-1");
    assert_eq!(stored[0].1, "kfrag-1");
}

#[tokio::test]
async fn test_execute_reencrypt() {
    let client = MockAOClient::new();

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateCapsule {
                kfrag_id: "kfrag-1".to_string(),
                capsule_id: "capsule-1".to_string(),
                capsule: Binary::from(vec![4, 5, 6]),
            },
        )
        .await;

    let result = client
        .execute(
            "process-1",
            ExecuteMsg::Reencrypt {
                kfrag_id: "kfrag-1".to_string(),
                capsule_id: "capsule-1".to_string(),
            },
        )
        .await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.events[0].event_type, "reencrypt");

    let cfrags = client.get_stored_cfrags("process-1");
    assert_eq!(cfrags.len(), 1);
}

#[tokio::test]
async fn test_query_get_cfrag() {
    let client = MockAOClient::new();

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::Reencrypt {
                kfrag_id: "kfrag-1".to_string(),
                capsule_id: "capsule-1".to_string(),
            },
        )
        .await;

    let result = client
        .query(
            "process-1",
            QueryMsg::GetCFrag {
                kfrag_id: "kfrag-1".to_string(),
                capsule_id: "capsule-1".to_string(),
            },
        )
        .await;

    assert!(result.is_ok());
    let binary = result.unwrap();
    let response: GetCFragResponse = serde_json::from_slice(binary.as_slice()).unwrap();
    assert!(!response.cfrag.is_empty());
}

#[tokio::test]
async fn test_query_get_cfrag_not_found() {
    let client = MockAOClient::new();

    let result = client
        .query(
            "process-1",
            QueryMsg::GetCFrag {
                kfrag_id: "kfrag-1".to_string(),
                capsule_id: "capsule-1".to_string(),
            },
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("CFrag not ready"));
}

#[tokio::test]
async fn test_query_list_capsules_by_kfrag() {
    let client = MockAOClient::new();

    for i in 1..=5 {
        let _ = client
            .execute(
                "process-1",
                ExecuteMsg::DelegateCapsule {
                    kfrag_id: "kfrag-1".to_string(),
                    capsule_id: format!("capsule-{i}"),
                    capsule: Binary::from(vec![i as u8]),
                },
            )
            .await;
    }

    let result = client
        .query(
            "process-1",
            QueryMsg::ListCapsulesByKFrag {
                kfrag_id: "kfrag-1".to_string(),
                start_after: None,
                limit: Some(3),
            },
        )
        .await;

    assert!(result.is_ok());
    let binary = result.unwrap();
    let response: ListCapsulesByKFragResponse = serde_json::from_slice(binary.as_slice()).unwrap();
    assert_eq!(response.capsules.len(), 3);
}

#[tokio::test]
async fn test_dry_run() {
    let client = MockAOClient::new();
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kfrag-1".to_string(),
        kfrag: Binary::from(vec![1, 2, 3, 4]),
    };

    let result = client.dry_run("process-1", msg).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.events[0].event_type, "dry_run:delegate_kfrag");

    let stored = client.get_stored_kfrags("process-1");
    assert!(stored.is_empty());
}

#[tokio::test]
async fn test_error_injection() {
    let client = MockAOClient::new();
    client.inject_error(AOCommunicationError::connection_error("test error"));

    let result = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    assert!(result.is_err());
    if let Err(AOCommunicationError::ConnectionError { details }) = result {
        assert_eq!(details, "test error");
    } else {
        panic!("Expected ConnectionError");
    }

    let result2 = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_clear_error() {
    let client = MockAOClient::new();
    client.inject_error(AOCommunicationError::connection_error("test error"));
    client.clear_error();

    let result = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validation_error() {
    let client = MockAOClient::new();
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "".to_string(),
        kfrag: Binary::from(vec![1, 2, 3]),
    };

    let result = client.execute("process-1", msg).await;
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(AOCommunicationError::ValidationError { .. })
    ));
}

#[tokio::test]
async fn test_empty_process_id_error() {
    let client = MockAOClient::new();
    let msg = ExecuteMsg::DelegateKFrag {
        kfrag_id: "kfrag-1".to_string(),
        kfrag: Binary::from(vec![1, 2, 3]),
    };

    let result = client.execute("", msg).await;
    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(AOCommunicationError::InvalidProcessId { .. })
    ));
}

#[tokio::test]
async fn test_clear_storage() {
    let client = MockAOClient::new();

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    assert!(!client.get_stored_kfrags("process-1").is_empty());

    client.clear();

    assert!(client.get_stored_kfrags("process-1").is_empty());
}

#[tokio::test]
async fn test_multiple_processes() {
    let client = MockAOClient::new();

    let _ = client
        .execute(
            "process-1",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-1".to_string(),
                kfrag: Binary::from(vec![1, 2, 3]),
            },
        )
        .await;

    let _ = client
        .execute(
            "process-2",
            ExecuteMsg::DelegateKFrag {
                kfrag_id: "kfrag-2".to_string(),
                kfrag: Binary::from(vec![4, 5, 6]),
            },
        )
        .await;

    let kfrags1 = client.get_stored_kfrags("process-1");
    let kfrags2 = client.get_stored_kfrags("process-2");

    assert_eq!(kfrags1.len(), 1);
    assert_eq!(kfrags2.len(), 1);
    assert_eq!(kfrags1[0].0, "kfrag-1");
    assert_eq!(kfrags2[0].0, "kfrag-2");
}

#[tokio::test]
async fn test_ao_client_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MockAOClient>();
}
