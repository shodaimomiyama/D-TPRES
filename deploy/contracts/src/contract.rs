use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::handlers::{execute_handler, query_handler, ContractError};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, DEFAULT_LIST_LIMIT};

type ContractResult<T = Response> = Result<T, ContractError>;

// --------------------- インスタンス化 ---------------------
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult {
    // メッセージバリデーション
    msg.validate()
        .map_err(|e| ContractError::ValidationError { msg: e })?;

    // 設定の初期化
    let config = Config {
        process_id: msg.process_id.clone(),
        default_list_limit: DEFAULT_LIST_LIMIT,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("process_id", msg.process_id)
        .add_attribute("default_list_limit", DEFAULT_LIST_LIMIT.to_string()))
}

// --------------------- 実行 ---------------------
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> ContractResult {
    execute_handler(deps, env, info, msg)
}

// --------------------- クエリ ---------------------
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    // MessageInfo不要だが、統一インターフェースのためmock作成
    let mock_info = MessageInfo {
        sender: deps.api.addr_validate("querier")?,
        funds: vec![],
    };

    query_handler(deps, env, mock_info, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Binary;

    #[test]
    fn test_instantiate_success() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            process_id: "test_process_123".to_string(),
        };

        let result = instantiate(deps.as_mut(), env, info, msg);
        assert!(result.is_ok());

        // 設定が正しく保存されているかチェック
        let config = CONFIG.load(&deps.storage).unwrap();
        assert_eq!(config.process_id, "test_process_123");
    }

    #[test]
    fn test_instantiate_validation_error() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);

        let msg = InstantiateMsg {
            process_id: "".to_string(), // 空のprocess_idは無効
        };

        let result = instantiate(deps.as_mut(), env, info, msg);
        assert!(matches!(result, Err(ContractError::ValidationError { .. })));
    }

    #[test]
    fn test_execute_submit_kfrag() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // 最初にinstantiateを実行
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // SubmitKFragを実行
        let exec_msg = ExecuteMsg::SubmitKFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            kfrag: Binary::from(b"test_kfrag_data"),
        };

        let result = execute(deps.as_mut(), env, info, exec_msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_submit_capsule_kfrag_not_found() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("sender", &[]);

        // 最初にinstantiateを実行
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        // kFragが存在しない状態でSubmitCapsuleを実行
        let exec_msg = ExecuteMsg::SubmitCapsule {
            kfrag_id: "nonexistent_kfrag".to_string(),
            capsule_id: "test_capsule".to_string(),
            capsule: Binary::from(b"test_capsule_data"),
        };

        let result = execute(deps.as_mut(), env, info, exec_msg);
        assert!(matches!(result, Err(ContractError::KFragNotFound { .. })));
    }

    #[test]
    fn test_query_get_cfrag_not_ready() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 最初にinstantiateを実行
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        // 存在しないcFragをクエリ
        let query_msg = QueryMsg::GetCFrag {
            kfrag_id: "nonexistent_kfrag".to_string(),
            capsule_id: "nonexistent_capsule".to_string(),
        };

        let result = query(deps.as_ref(), env, query_msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_query_list_capsules_empty() {
        let mut deps = mock_dependencies();
        let env = mock_env();

        // 最初にinstantiateを実行
        let init_msg = InstantiateMsg {
            process_id: "test_process".to_string(),
        };
        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), env.clone(), info, init_msg).unwrap();

        // 空のkFragに対してCapsule一覧をクエリ
        let query_msg = QueryMsg::ListCapsulesByKFrag {
            kfrag_id: "test_kfrag".to_string(),
            start_after: None,
            limit: None,
        };

        let result = query(deps.as_ref(), env, query_msg);
        assert!(result.is_ok());

        // レスポンスが空であることを確認
        let response: crate::msg::ListCapsulesByKFragResponse =
            cosmwasm_std::from_json(&result.unwrap()).unwrap();
        assert!(response.capsules.is_empty());
        assert!(response.next_start_after.is_none());
    }
}
