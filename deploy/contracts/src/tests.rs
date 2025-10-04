#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_json};
    use crate::msg::{InstantiateMsg, ProcessMetadata, QueryMsg};
    use crate::state::ProcessRole;
    use crate::contract::{instantiate, query};

    #[test]
    fn test_owner_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                threshold_k: 3,
                total_holders_n: 5,
                capsule_txid: "test_capsule_txid".to_string(),
                requester_pubkey: "test_pubkey".to_string(),
            },
        };

        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3);
        assert_eq!(res.attributes[0].value, "instantiate_owner");
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
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.attributes[0].value, "instantiate_holder");
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
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.attributes[0].value, "instantiate_requester");
    }

    #[test]
    fn test_invalid_threshold() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &coins(1000, "earth"));

        // k > n の無効な設定
        let msg = InstantiateMsg {
            process_role: ProcessRole::Owner,
            metadata: ProcessMetadata::Owner {
                threshold_k: 6,
                total_holders_n: 5,
                capsule_txid: "test_capsule_txid".to_string(),
                requester_pubkey: "test_pubkey".to_string(),
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
                threshold_k: 3,
                total_holders_n: 5,
                capsule_txid: "test_capsule_txid".to_string(),
                requester_pubkey: "test_pubkey".to_string(),
            },
        };

        instantiate(deps.as_mut(), env.clone(), info, instantiate_msg).unwrap();

        // メタデータをクエリ
        let query_msg = QueryMsg::GetOwnerMetadata {};
        let res = query(deps.as_ref(), env, query_msg).unwrap();

        let metadata_response: crate::msg::OwnerMetadataResponse = from_json(&res).unwrap();
        assert_eq!(metadata_response.metadata.threshold_k, 3);
        assert_eq!(metadata_response.metadata.total_holders_n, 5);
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
}