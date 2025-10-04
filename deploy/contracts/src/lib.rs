use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

mod contract;
mod msg;
mod state;
mod owner;
mod holder;
mod requester;
mod ao_integration;

#[cfg(test)]
mod tests;

use msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use contract::ContractError;

// エントリーポイント関数
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    contract::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}

// 公開API（テスト用）
pub use msg::*;
pub use state::*;
