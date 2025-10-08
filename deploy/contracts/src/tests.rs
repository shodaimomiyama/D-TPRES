#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json};
    use crate::msg::{InstantiateMsg, ProcessMetadata, QueryMsg};
    use crate::state::{ProcessRole, OWNER_METADATA, HOLDER_METADATA, REQUESTER_METADATA};
    use crate::contract::{instantiate, query};

    #[test]
    fn test_owner_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                owner_id: "test_owner".to_string(),
                total_holders_n: 5,
                signer_pubkey: "test_pubkey".to_string(),
                holder_process_ids: None, // RandAO使用
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 5); // action + owner_id + total_holders_n + signer_pubkey + universal_init
        assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "instantiate_owner"));
        assert!(res.attributes.iter().any(|attr| attr.key == "universal_init" && attr.value == "true"));
    }

    #[test]
    fn test_holder_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            process_role: ProcessRole::Holder,
            metadata: ProcessMetadata::Holder {
                holder_id: "test_holder_1".to_string(),
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3); // action + holder_id + universal_init
        assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "instantiate_holder"));
        assert!(res.attributes.iter().any(|attr| attr.key == "universal_init" && attr.value == "true"));
    }

    #[test]
    fn test_requester_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            process_role: ProcessRole::Requester,
            metadata: ProcessMetadata::Requester {
                requester_id: "test_requester_1".to_string(),
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3); // action + requester_id + universal_init
        assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "instantiate_requester"));
        assert!(res.attributes.iter().any(|attr| attr.key == "universal_init" && attr.value == "true"));
    }

    #[test]
    fn test_invalid_threshold() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // Empty owner_id invalid setting
        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                owner_id: "".to_string(),
                total_holders_n: 5,
                signer_pubkey: "test_pubkey".to_string(),
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_query_owner_metadata() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // まずOwnerプロセスを初期化
        let instantiate_msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                owner_id: "test_owner".to_string(),
                total_holders_n: 5,
                signer_pubkey: "test_pubkey".to_string(),
                holder_process_ids: None, // RandAO使用
            },
        };

        instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

        // メタデータをクエリ
        let query_msg = QueryMsg::GetOwnerMetadata {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();

        let metadata_response: crate::msg::OwnerMetadataResponse = from_json(&res).unwrap();
        assert_eq!(metadata_response.metadata.owner_id, "test_owner");
        assert_eq!(metadata_response.metadata.total_holders_n, 5);
    }

    #[test]
    fn test_universal_initialization() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // Initialize as Owner but verify all metadata exists
        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                owner_id: "test_owner".to_string(),
                total_holders_n: 5,
                signer_pubkey: "test_pubkey".to_string(),
                holder_process_ids: None, // RandAO使用
            },
        };

        instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Verify all three metadata types exist in storage
        let owner_meta = OWNER_METADATA.load(deps.as_ref().storage).unwrap();
        let holder_meta = HOLDER_METADATA.load(deps.as_ref().storage).unwrap();
        let requester_meta = REQUESTER_METADATA.load(deps.as_ref().storage).unwrap();

        // Owner metadata should have actual values
        assert_eq!(owner_meta.owner_id, "test_owner");
        assert_eq!(owner_meta.total_holders_n, 5);
        assert_eq!(owner_meta.signer_pubkey, "test_pubkey");

        // Other metadata should have default values
        assert_eq!(holder_meta.holder_id, ""); // Default value
        assert_eq!(requester_meta.requester_id, ""); // Default value
    }

    #[test]
    fn test_query_process_role() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // Holderプロセスを初期化
        let instantiate_msg = InstantiateMsg {
            process_role: ProcessRole::Holder,
            metadata: ProcessMetadata::Holder {
                holder_id: "test_holder_1".to_string(),
            },
        };

        instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

        // プロセス役割をクエリ
        let query_msg = QueryMsg::GetProcessRole {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();

        let role_response: crate::msg::ProcessRoleResponse = from_json(&res).unwrap();
        assert_eq!(role_response.role, ProcessRole::Holder);
    }

    #[test]
    fn test_owner_instantiate_with_predefined_holders() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // プレースホルダー実装：事前定義されたHolder ProcessのIDを使用
        let predefined_holders = vec![
            "holder_process_001".to_string(),
            "holder_process_002".to_string(),
            "holder_process_003".to_string(),
        ];

        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                owner_id: "test_owner_with_predefined".to_string(),
                total_holders_n: 3,
                signer_pubkey: "test_pubkey".to_string(),
                holder_process_ids: Some(predefined_holders.clone()),
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // レスポンス属性を確認
        assert!(res.attributes.iter().any(|attr| attr.key == "action" && attr.value == "instantiate_owner"));
        assert!(res.attributes.iter().any(|attr| attr.key == "universal_init" && attr.value == "true"));

        // メタデータが正しく保存されていることを確認
        let owner_meta = OWNER_METADATA.load(deps.as_ref().storage).unwrap();
        assert_eq!(owner_meta.owner_id, "test_owner_with_predefined");
        assert_eq!(owner_meta.total_holders_n, 3);
        assert_eq!(owner_meta.holder_process_ids, Some(predefined_holders));
    }
}