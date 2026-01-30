use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

mod contract;
mod handlers;
mod msg;
mod state;

// CosmWasm WASM env doesn't have OS-level randomness; provide via getrandom custom backend
getrandom::register_custom_getrandom!(cosmwasm_getrandom);

fn cosmwasm_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    // Deterministic placeholder for WASM compilation.
    // In production, entropy should be injected via message parameters.
    for (i, byte) in buf.iter_mut().enumerate() {
        *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
    }
    Ok(())
}

pub use handlers::{ContractError, REPLY_DELEGATE_CAPSULE, REPLY_DELEGATE_KFRAG};

// エントリーポイント関数
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: msg::InstantiateMsg,
) -> Result<Response, ContractError> {
    contract::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: msg::ExecuteMsg,
) -> Result<Response, ContractError> {
    contract::execute(deps, env, info, msg)
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: msg::QueryMsg) -> StdResult<Binary> {
    contract::query(deps, env, msg)
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: cosmwasm_std::Reply) -> Result<Response, ContractError> {
    handlers::handle_reply(deps, env, msg)
}

// 公開API（テスト用）
pub use msg::*;
pub use state::*;
