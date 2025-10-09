use cosmwasm_std::{DepsMut, Deps, Env, MessageInfo, Response, Binary, StdResult};
use serde::{Deserialize, Serialize};
use serde_json;

use crate::contract::{execute, query, ContractError};
use crate::msg::{ExecuteMsg, QueryMsg, KFragWithSignature, KFragReceiptData, CapsuleStorageData, CFragSubmission};

/// AO Network用のメッセージハンドラー
///
/// CWAOライブラリから送信されるAO Network形式のメッセージを
/// 標準的なCosmWasmメッセージ形式に変換して処理します。

#[derive(Serialize, Deserialize, Debug)]
pub struct AOMessage {
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Input")]
    pub input: Option<String>,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

/// AO Network用のエグゼキュートハンドラー
pub fn handle_ao_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    ao_msg: AOMessage,
) -> Result<Response, ContractError> {
    // アクション名に基づいてExecuteMsgに変換
    let execute_msg = convert_ao_to_execute_msg(&ao_msg)?;

    // 標準的なexecute関数を呼び出し
    execute(deps, env, info, execute_msg)
}

/// AO Network用のクエリハンドラー
pub fn handle_ao_query(
    deps: Deps,
    env: Env,
    ao_msg: AOMessage,
) -> StdResult<Binary> {
    // アクション名に基づいてQueryMsgに変換
    let query_msg = convert_ao_to_query_msg(&ao_msg)?;

    // 標準的なquery関数を呼び出し
    query(deps, env, query_msg)
}

/// AOメッセージをExecuteMsgに変換
fn convert_ao_to_execute_msg(ao_msg: &AOMessage) -> Result<ExecuteMsg, ContractError> {
    let input_data = ao_msg.input.as_ref()
        .ok_or_else(|| ContractError::ValidationError {
            msg: "Input data is required".to_string()
        })?;

    match ao_msg.action.as_str() {
        "receive_k_frags" => {
            #[derive(Deserialize)]
            struct ReceiveKFragsInput {
                kfrags: Vec<KFragWithSignature>,
            }

            let input: ReceiveKFragsInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse receive_k_frags input: {}", e)
                })?;

            Ok(ExecuteMsg::ReceiveKFrags {
                kfrags: input.kfrags
            })
        }

        "receive_k_frag" => {
            #[derive(Deserialize)]
            struct ReceiveKFragInput {
                kfrag_data: KFragReceiptData,
            }

            let input: ReceiveKFragInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse receive_k_frag input: {}", e)
                })?;

            Ok(ExecuteMsg::ReceiveKFrag {
                kfrag_data: input.kfrag_data
            })
        }

        "generate_c_frag" => {
            #[derive(Deserialize)]
            struct GenerateCFragInput {
                kfrag_id: String,
            }

            let input: GenerateCFragInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse generate_c_frag input: {}", e)
                })?;

            Ok(ExecuteMsg::GenerateCFrag {
                kfrag_id: input.kfrag_id
            })
        }

        "store_capsule" => {
            #[derive(Deserialize)]
            struct StoreCapsuleInput {
                capsule_data: CapsuleStorageData,
            }

            let input: StoreCapsuleInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse store_capsule input: {}", e)
                })?;

            Ok(ExecuteMsg::StoreCapsule {
                capsule_data: input.capsule_data
            })
        }

        "start_c_frag_collection" => {
            #[derive(Deserialize)]
            struct StartCFragCollectionInput {
                session_id: String,
                threshold: u32,
            }

            let input: StartCFragCollectionInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse start_c_frag_collection input: {}", e)
                })?;

            Ok(ExecuteMsg::StartCFragCollection {
                session_id: input.session_id,
                threshold: input.threshold,
            })
        }

        "collect_c_frag" => {
            #[derive(Deserialize)]
            struct CollectCFragInput {
                session_id: String,
                cfrag_data: CFragSubmission,
            }

            let input: CollectCFragInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse collect_c_frag input: {}", e)
                })?;

            Ok(ExecuteMsg::CollectCFrag {
                session_id: input.session_id,
                cfrag_data: input.cfrag_data,
            })
        }

        "initiate_recovery" => {
            #[derive(Deserialize)]
            struct InitiateRecoveryInput {
                session_id: String,
                capsule_data: Vec<u8>,
            }

            let input: InitiateRecoveryInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse initiate_recovery input: {}", e)
                })?;

            Ok(ExecuteMsg::InitiateRecovery {
                session_id: input.session_id,
                capsule_data: input.capsule_data,
            })
        }

        "update_process_status" => {
            #[derive(Deserialize)]
            struct UpdateProcessStatusInput {
                status: String,
            }

            let input: UpdateProcessStatusInput = serde_json::from_str(input_data)
                .map_err(|e| ContractError::ValidationError {
                    msg: format!("Failed to parse update_process_status input: {}", e)
                })?;

            Ok(ExecuteMsg::UpdateProcessStatus {
                status: input.status
            })
        }

        _ => Err(ContractError::ValidationError {
            msg: format!("Unknown action: {}", ao_msg.action)
        })
    }
}

/// AOメッセージをQueryMsgに変換
fn convert_ao_to_query_msg(ao_msg: &AOMessage) -> StdResult<QueryMsg> {
    match ao_msg.action.as_str() {
        "get_owner_metadata" => {
            Ok(QueryMsg::GetOwnerMetadata {})
        }

        "get_k_frags" => {
            if let Some(input_data) = &ao_msg.input {
                #[derive(Deserialize)]
                struct GetKFragsInput {
                    holder_id: Option<String>,
                }

                let input: GetKFragsInput = serde_json::from_str(input_data)
                    .map_err(|e| cosmwasm_std::StdError::generic_err(
                        format!("Failed to parse get_k_frags input: {}", e)
                    ))?;

                Ok(QueryMsg::GetKFrags {
                    holder_id: input.holder_id
                })
            } else {
                Ok(QueryMsg::GetKFrags { holder_id: None })
            }
        }

        "get_holder_assignments" => {
            Ok(QueryMsg::GetHolderAssignments {})
        }

        "get_holder_metadata" => {
            Ok(QueryMsg::GetHolderMetadata {})
        }

        "get_k_frag_by_id" => {
            let input_data = ao_msg.input.as_ref()
                .ok_or_else(|| cosmwasm_std::StdError::generic_err("Input data is required"))?;

            #[derive(Deserialize)]
            struct GetKFragByIdInput {
                kfrag_id: String,
            }

            let input: GetKFragByIdInput = serde_json::from_str(input_data)
                .map_err(|e| cosmwasm_std::StdError::generic_err(
                    format!("Failed to parse get_k_frag_by_id input: {}", e)
                ))?;

            Ok(QueryMsg::GetKFragById {
                kfrag_id: input.kfrag_id
            })
        }

        "get_c_frags" => {
            if let Some(input_data) = &ao_msg.input {
                #[derive(Deserialize)]
                struct GetCFragsInput {
                    processed_only: Option<bool>,
                }

                let input: GetCFragsInput = serde_json::from_str(input_data)
                    .map_err(|e| cosmwasm_std::StdError::generic_err(
                        format!("Failed to parse get_c_frags input: {}", e)
                    ))?;

                Ok(QueryMsg::GetCFrags {
                    processed_only: input.processed_only
                })
            } else {
                Ok(QueryMsg::GetCFrags { processed_only: None })
            }
        }

        "get_cached_capsules" => {
            Ok(QueryMsg::GetCachedCapsules {})
        }

        "get_requester_metadata" => {
            Ok(QueryMsg::GetRequesterMetadata {})
        }

        "get_c_frag_collection" => {
            let input_data = ao_msg.input.as_ref()
                .ok_or_else(|| cosmwasm_std::StdError::generic_err("Input data is required"))?;

            #[derive(Deserialize)]
            struct GetCFragCollectionInput {
                session_id: String,
            }

            let input: GetCFragCollectionInput = serde_json::from_str(input_data)
                .map_err(|e| cosmwasm_std::StdError::generic_err(
                    format!("Failed to parse get_c_frag_collection input: {}", e)
                ))?;

            Ok(QueryMsg::GetCFragCollection {
                session_id: input.session_id
            })
        }

        "get_recovery_session" => {
            let input_data = ao_msg.input.as_ref()
                .ok_or_else(|| cosmwasm_std::StdError::generic_err("Input data is required"))?;

            #[derive(Deserialize)]
            struct GetRecoverySessionInput {
                session_id: String,
            }

            let input: GetRecoverySessionInput = serde_json::from_str(input_data)
                .map_err(|e| cosmwasm_std::StdError::generic_err(
                    format!("Failed to parse get_recovery_session input: {}", e)
                ))?;

            Ok(QueryMsg::GetRecoverySession {
                session_id: input.session_id
            })
        }

        "get_threshold_info" => {
            Ok(QueryMsg::GetThresholdInfo {})
        }

        "get_connected_processes" => {
            Ok(QueryMsg::GetConnectedProcesses {})
        }

        "get_process_info" => {
            let input_data = ao_msg.input.as_ref()
                .ok_or_else(|| cosmwasm_std::StdError::generic_err("Input data is required"))?;

            #[derive(Deserialize)]
            struct GetProcessInfoInput {
                process_id: String,
            }

            let input: GetProcessInfoInput = serde_json::from_str(input_data)
                .map_err(|e| cosmwasm_std::StdError::generic_err(
                    format!("Failed to parse get_process_info input: {}", e)
                ))?;

            Ok(QueryMsg::GetProcessInfo {
                process_id: input.process_id
            })
        }

        "get_process_role" => {
            Ok(QueryMsg::GetProcessRole {})
        }

        "get_process_status" => {
            Ok(QueryMsg::GetProcessStatus {})
        }

        _ => Err(cosmwasm_std::StdError::generic_err(
            format!("Unknown query action: {}", ao_msg.action)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::KFragWithSignature;

    #[test]
    fn test_ao_to_execute_msg_conversion() {
        let ao_msg = AOMessage {
            action: "receive_k_frags".to_string(),
            input: Some(r#"{"kfrags":[{"kfrag_id":"test","kfrag_data":[1,2,3],"signature":[4,5,6]}]}"#.to_string()),
            other: serde_json::Value::Null,
        };

        let execute_msg = convert_ao_to_execute_msg(&ao_msg).unwrap();

        match execute_msg {
            ExecuteMsg::ReceiveKFrags { kfrags } => {
                assert_eq!(kfrags.len(), 1);
                assert_eq!(kfrags[0].kfrag_id, "test");
            }
            _ => panic!("Expected ReceiveKFrags variant"),
        }
    }

    #[test]
    fn test_ao_to_query_msg_conversion() {
        let ao_msg = AOMessage {
            action: "get_k_frags".to_string(),
            input: Some(r#"{"holder_id":"test_holder"}"#.to_string()),
            other: serde_json::Value::Null,
        };

        let query_msg = convert_ao_to_query_msg(&ao_msg).unwrap();

        match query_msg {
            QueryMsg::GetKFrags { holder_id } => {
                assert_eq!(holder_id, Some("test_holder".to_string()));
            }
            _ => panic!("Expected GetKFrags variant"),
        }
    }
}