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
    query_handler(deps, env, msg)
}

// tests moved to `tests/`
