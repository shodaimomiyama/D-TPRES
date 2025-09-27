//! D-TPRES CosmWasm Contract Entry Point
//! 
//! This module provides the entry points for the CosmWasm contract
//! that will run on the AO Network.

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult,
};

pub mod domain;
pub mod usecase;
pub mod ao;
pub mod crypto_core;

#[cfg(feature = "local")]
pub mod local;

#[cfg(feature = "browser")]
pub mod wasm_bindings;

// Conditional compilation for service module
#[cfg(not(target_arch = "wasm32"))]
pub mod service;

#[cfg(target_arch = "wasm32")]
#[path = "service_wasm/mod.rs"]
pub mod service;

// Import crypto service
use crate::crypto_core::{CryptoService, CryptoServiceImpl};

/// Contract instantiation message
#[cw_serde]
pub struct InstantiateMsg {
    /// Process role: "owner", "holder", or "requester"
    pub role: String,
    /// Optional process ID for identification
    pub process_id: Option<String>,
}

/// Contract execution messages
#[cw_serde]
pub enum ExecuteMsg {
    /// Store kFrags received from O-Browser
    StoreKfrags {
        kfrags: Vec<SerializedKeyFragment>,
    },
    /// Process an access request
    ProcessAccessRequest {
        requester_id: String,
        capsule_id: String,
    },
    /// Execute local encryption setup (O-Browser functionality)
    SetupEncryption {
        secret: Vec<u8>,
        threshold: u8,
        total_shares: u8,
    },
}

/// Contract query messages
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get stored kFrags
    #[returns(GetKfragsResponse)]
    GetKfrags {},
    /// Get process information
    #[returns(ProcessInfoResponse)]
    ProcessInfo {},
}

/// Serialized version of KeyFragment for message passing
#[cw_serde]
pub struct SerializedKeyFragment {
    pub id: u8,
    pub key_data: Vec<u8>,
    pub verification_data: Vec<u8>,
    pub precursor: Vec<u8>,
}

/// Response for GetKfrags query
#[cw_serde]
pub struct GetKfragsResponse {
    pub kfrags: Vec<SerializedKeyFragment>,
}

/// Response for ProcessInfo query
#[cw_serde]
pub struct ProcessInfoResponse {
    pub role: String,
    pub process_id: String,
    pub kfrags_count: usize,
}

/// Contract state
#[cw_serde]
pub struct State {
    pub role: String,
    pub process_id: String,
    pub kfrags: Vec<SerializedKeyFragment>,
}

/// State storage key
const STATE_KEY: &str = "state";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    // Validate role
    if !["owner", "holder", "requester"].contains(&msg.role.as_str()) {
        return Err(StdError::generic_err("Invalid role specified"));
    }

    // Initialize state
    let state = State {
        role: msg.role.clone(),
        process_id: msg.process_id.unwrap_or_else(|| {
            format!("process_{}", _env.block.time.seconds())
        }),
        kfrags: vec![],
    };

    // Save state
    deps.storage.set(
        STATE_KEY.as_bytes(),
        &to_json_binary(&state)?.to_vec(),
    );

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("role", msg.role))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::StoreKfrags { kfrags } => execute_store_kfrags(deps, env, info, kfrags),
        ExecuteMsg::ProcessAccessRequest {
            requester_id,
            capsule_id,
        } => execute_process_access_request(deps, env, info, requester_id, capsule_id),
        ExecuteMsg::SetupEncryption {
            secret,
            threshold,
            total_shares,
        } => execute_setup_encryption(deps, env, info, secret, threshold, total_shares),
    }
}

fn execute_store_kfrags(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    kfrags: Vec<SerializedKeyFragment>,
) -> StdResult<Response> {
    // Load current state
    let state_bytes = deps.storage.get(STATE_KEY.as_bytes())
        .ok_or_else(|| StdError::generic_err("State not found"))?;
    let mut state: State = serde_json::from_slice(&state_bytes)
        .map_err(|e| StdError::generic_err(format!("Failed to deserialize state: {}", e)))?;

    // Store kfrags
    state.kfrags = kfrags;
    
    // Save updated state
    deps.storage.set(
        STATE_KEY.as_bytes(),
        &to_json_binary(&state)?.to_vec(),
    );

    Ok(Response::new()
        .add_attribute("method", "store_kfrags")
        .add_attribute("kfrags_count", state.kfrags.len().to_string()))
}

fn execute_process_access_request(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    requester_id: String,
    capsule_id: String,
) -> StdResult<Response> {
    // Load current state
    let state_bytes = deps.storage.get(STATE_KEY.as_bytes())
        .ok_or_else(|| StdError::generic_err("State not found"))?;
    let state: State = serde_json::from_slice(&state_bytes)
        .map_err(|e| StdError::generic_err(format!("Failed to deserialize state: {}", e)))?;

    // MVP: Auto-approve access request
    // In production, this would verify access rights via smart contracts
    
    Ok(Response::new()
        .add_attribute("method", "process_access_request")
        .add_attribute("requester_id", requester_id)
        .add_attribute("capsule_id", capsule_id)
        .add_attribute("status", "approved")
        .add_attribute("kfrags_available", state.kfrags.len().to_string()))
}

fn execute_setup_encryption(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    secret: Vec<u8>,
    threshold: u8,
    total_shares: u8,
) -> StdResult<Response> {
    // This would typically be executed in O-Browser (local environment)
    // For MVP, we simulate the process here
    
    let crypto_service = CryptoServiceImpl::new();
    
    // Generate owner keypair
    let (owner_sk, owner_pk) = crypto_service.generate_keypair()
        .map_err(|e| StdError::generic_err(format!("Failed to generate keypair: {:?}", e)))?;
    
    // Split secret using Shamir's Secret Sharing
    let shares = crypto_service.split_secret_shamir(&secret, threshold, total_shares)
        .map_err(|e| StdError::generic_err(format!("Failed to split secret: {:?}", e)))?;
    
    // Generate symmetric key for share encryption
    let k_o = vec![0u8; 32]; // In production, generate proper random key
    
    // Create capsule (encrypt k_o with owner's public key)
    let (capsule, _ciphertext) = crypto_service.create_pre_capsule(&owner_pk, &k_o)
        .map_err(|e| StdError::generic_err(format!("Failed to create capsule: {:?}", e)))?;
    
    // MVP: Generate requester keypair automatically
    let (_requester_sk, requester_pk) = crypto_service.generate_keypair()
        .map_err(|e| StdError::generic_err(format!("Failed to generate requester keypair: {:?}", e)))?;
    
    // Generate re-encryption key
    let rekey = crypto_service.generate_reencryption_key(&owner_sk, &requester_pk)
        .map_err(|e| StdError::generic_err(format!("Failed to generate reencryption key: {:?}", e)))?;
    
    // Create kFrags
    let kfrags = crypto_service.create_kfrags(&rekey, threshold, total_shares)
        .map_err(|e| StdError::generic_err(format!("Failed to create kfrags: {:?}", e)))?;
    
    // Convert kfrags to serialized format
    let serialized_kfrags: Vec<SerializedKeyFragment> = kfrags.into_iter()
        .map(|kf| SerializedKeyFragment {
            id: kf.id,
            key_data: kf.key_data.clone(),
            verification_data: kf.verification_data.clone(),
            precursor: kf.precursor.clone(),
        })
        .collect();
    
    // Store kfrags in state
    let state_bytes = deps.storage.get(STATE_KEY.as_bytes())
        .ok_or_else(|| StdError::generic_err("State not found"))?;
    let mut state: State = serde_json::from_slice(&state_bytes)
        .map_err(|e| StdError::generic_err(format!("Failed to deserialize state: {}", e)))?;
    
    state.kfrags = serialized_kfrags;
    
    deps.storage.set(
        STATE_KEY.as_bytes(),
        &to_json_binary(&state)?.to_vec(),
    );
    
    Ok(Response::new()
        .add_attribute("method", "setup_encryption")
        .add_attribute("shares_created", total_shares.to_string())
        .add_attribute("threshold", threshold.to_string())
        .add_attribute("kfrags_created", state.kfrags.len().to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetKfrags {} => query_get_kfrags(deps),
        QueryMsg::ProcessInfo {} => query_process_info(deps),
    }
}

fn query_get_kfrags(deps: Deps) -> StdResult<Binary> {
    let state_bytes = deps.storage.get(STATE_KEY.as_bytes())
        .ok_or_else(|| StdError::generic_err("State not found"))?;
    let state: State = serde_json::from_slice(&state_bytes)
        .map_err(|e| StdError::generic_err(format!("Failed to deserialize state: {}", e)))?;
    
    let response = GetKfragsResponse {
        kfrags: state.kfrags,
    };
    
    to_json_binary(&response)
}

fn query_process_info(deps: Deps) -> StdResult<Binary> {
    let state_bytes = deps.storage.get(STATE_KEY.as_bytes())
        .ok_or_else(|| StdError::generic_err("State not found"))?;
    let state: State = serde_json::from_slice(&state_bytes)
        .map_err(|e| StdError::generic_err(format!("Failed to deserialize state: {}", e)))?;
    
    let response = ProcessInfoResponse {
        role: state.role,
        process_id: state.process_id,
        kfrags_count: state.kfrags.len(),
    };
    
    to_json_binary(&response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        
        let msg = InstantiateMsg {
            role: "owner".to_string(),
            process_id: Some("test_process".to_string()),
        };
        
        let res = instantiate(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes.len(), 2);
        assert_eq!(res.attributes[0].value, "instantiate");
        assert_eq!(res.attributes[1].value, "owner");
    }
    
    #[test]
    fn test_store_kfrags() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info("creator", &[]);
        
        // First instantiate
        let init_msg = InstantiateMsg {
            role: "owner".to_string(),
            process_id: None,
        };
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();
        
        // Store kfrags
        let kfrags = vec![
            SerializedKeyFragment {
                id: 0,
                key_data: vec![1, 2, 3],
                verification_data: vec![4, 5, 6],
                precursor: vec![],
            },
        ];
        
        let msg = ExecuteMsg::StoreKfrags { kfrags };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        
        assert_eq!(res.attributes[0].value, "store_kfrags");
        assert_eq!(res.attributes[1].value, "1");
    }
}