pub mod contract;
pub mod handlers;
pub mod msg;
pub mod state;

#[cfg(target_arch = "wasm32")]
mod wasm_entropy {
    getrandom::register_custom_getrandom!(cosmwasm_getrandom);

    fn cosmwasm_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte = (i as u8).wrapping_mul(7).wrapping_add(42);
        }
        Ok(())
    }
}

pub use handlers::{
    ContractError, StoredCFrag, StoredKeyFrag, VerificationData, REPLY_DELEGATE_CAPSULE,
    REPLY_DELEGATE_KFRAG,
};

#[cfg(target_arch = "wasm32")]
mod wasm_entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    use crate::handlers::ContractError;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: crate::msg::InstantiateMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: crate::msg::ExecuteMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: crate::msg::QueryMsg) -> StdResult<Binary> {
        crate::contract::query(deps, env, msg)
    }

    #[entry_point]
    pub fn reply(
        deps: DepsMut,
        env: Env,
        msg: cosmwasm_std::Reply,
    ) -> Result<Response, ContractError> {
        crate::handlers::handle_reply(deps, env, msg)
    }
}

pub use contract::{execute, instantiate, query};
pub use msg::*;
pub use state::*;
