use crate::state::{BlobMeta, CapsuleStatus};
use cosmwasm_std::Binary;
use serde::{Deserialize, Serialize};

// --------------------- インスタンス化メッセージ ---------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub process_id: String,
}

impl InstantiateMsg {
    pub fn validate(&self) -> Result<(), String> {
        if self.process_id.is_empty() {
            return Err("process_id cannot be empty".to_string());
        }
        if self.process_id.len() > 128 {
            return Err("process_id must be <= 128 characters".to_string());
        }
        Ok(())
    }
}

// --------------------- 実行メッセージ ---------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SubmitKFrag {
        kfrag_id: String,
        kfrag: Binary,
    },
    SubmitCapsule {
        kfrag_id: String,
        capsule_id: String,
        capsule: Binary,
    },
    Reencrypt {
        kfrag_id: String,
        capsule_id: String,
    },
}

// --------------------- クエリメッセージ ---------------------
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetCFrag {
        kfrag_id: String,
        capsule_id: String,
    },
    ListCapsulesByKFrag {
        kfrag_id: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// --------------------- レスポンス型 ---------------------

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GetCFragResponse {
    pub cfrag: Binary,
    pub meta: BlobMeta,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CapsuleInfo {
    pub capsule_id: String,
    pub status: CapsuleStatus,
    pub updated_ts: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ListCapsulesByKFragResponse {
    pub capsules: Vec<CapsuleInfo>,
    pub next_start_after: Option<String>,
}

// --------------------- メッセージバリデーション ---------------------
pub trait ValidateMessage {
    fn validate(&self) -> Result<(), String>;
}

impl ValidateMessage for ExecuteMsg {
    fn validate(&self) -> Result<(), String> {
        match self {
            ExecuteMsg::SubmitKFrag { kfrag_id, kfrag } => {
                validate_kfrag_id(kfrag_id)?;
                validate_binary_data(kfrag, "kfrag")?;
                Ok(())
            }
            ExecuteMsg::SubmitCapsule {
                kfrag_id,
                capsule_id,
                capsule,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                validate_binary_data(capsule, "capsule")?;
                Ok(())
            }
            ExecuteMsg::Reencrypt {
                kfrag_id,
                capsule_id,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                Ok(())
            }
        }
    }
}

impl ValidateMessage for QueryMsg {
    fn validate(&self) -> Result<(), String> {
        match self {
            QueryMsg::GetCFrag {
                kfrag_id,
                capsule_id,
            } => {
                validate_kfrag_id(kfrag_id)?;
                validate_capsule_id(capsule_id)?;
                Ok(())
            }
            QueryMsg::ListCapsulesByKFrag {
                kfrag_id,
                start_after,
                limit,
            } => {
                validate_kfrag_id(kfrag_id)?;
                if let Some(start_after) = start_after {
                    validate_capsule_id(start_after)?;
                }
                if let Some(limit) = limit {
                    if *limit == 0 || *limit > 100 {
                        return Err("limit must be between 1 and 100".to_string());
                    }
                }
                Ok(())
            }
        }
    }
}

// --------------------- バリデーションヘルパー ---------------------
fn validate_kfrag_id(id: &str) -> Result<(), String> {
    validate_id(id, "kfrag_id")
}

fn validate_capsule_id(id: &str) -> Result<(), String> {
    validate_id(id, "capsule_id")
}

fn validate_id(id: &str, field_name: &str) -> Result<(), String> {
    if id.is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }
    if id.len() > 128 {
        return Err(format!("{} must be <= 128 characters", field_name));
    }

    // ASCII安全文字のみ許可
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "{} must contain only ASCII alphanumeric, underscore, or hyphen",
            field_name
        ));
    }

    Ok(())
}

fn validate_binary_data(data: &Binary, field_name: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err(format!("{} cannot be empty", field_name));
    }

    // 最大サイズチェック（128KB）
    if data.len() > 128 * 1024 {
        return Err(format!("{} exceeds maximum size of 128KB", field_name));
    }

    Ok(())
}

// --------------------- AOメッセージタグ ---------------------
// 将来的なAO Network環境での実行時に使用する予定の構造体
// 現在の実装では標準CosmWasmインターフェース（MessageInfo等）を使用
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AOMessageTags {
    pub app_name: String,
    pub action: String,
    pub read_only: String,
    pub input: String,
    pub process_id: String,
    pub actor: String,
    pub ts: String,
}

impl AOMessageTags {
    pub fn new_execute(action: &str, input: &str, process_id: &str, actor: &str, ts: &str) -> Self {
        Self {
            app_name: "cwao".to_string(),
            action: action.to_string(),
            read_only: "False".to_string(),
            input: input.to_string(),
            process_id: process_id.to_string(),
            actor: actor.to_string(),
            ts: ts.to_string(),
        }
    }

    pub fn new_query(action: &str, input: &str, process_id: &str, actor: &str, ts: &str) -> Self {
        Self {
            app_name: "cwao".to_string(),
            action: action.to_string(),
            read_only: "True".to_string(),
            input: input.to_string(),
            process_id: process_id.to_string(),
            actor: actor.to_string(),
            ts: ts.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Binary;

    #[test]
    fn test_instantiate_msg_validation() {
        let valid_msg = InstantiateMsg {
            process_id: "test_process_123".to_string(),
        };
        assert!(valid_msg.validate().is_ok());

        let empty_msg = InstantiateMsg {
            process_id: "".to_string(),
        };
        assert!(empty_msg.validate().is_err());

        let long_msg = InstantiateMsg {
            process_id: "a".repeat(129),
        };
        assert!(long_msg.validate().is_err());
    }

    #[test]
    fn test_execute_msg_validation() {
        let valid_msg = ExecuteMsg::SubmitKFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            kfrag: Binary::from(b"test_kfrag_data"),
        };
        assert!(valid_msg.validate().is_ok());

        let invalid_id_msg = ExecuteMsg::SubmitKFrag {
            kfrag_id: "invalid@id".to_string(),
            kfrag: Binary::from(b"test_kfrag_data"),
        };
        assert!(invalid_id_msg.validate().is_err());

        let empty_data_msg = ExecuteMsg::SubmitKFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            kfrag: Binary::from(b""),
        };
        assert!(empty_data_msg.validate().is_err());
    }

    #[test]
    fn test_query_msg_validation() {
        let valid_msg = QueryMsg::GetCFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            capsule_id: "test_capsule_456".to_string(),
        };
        assert!(valid_msg.validate().is_ok());

        let valid_list_msg = QueryMsg::ListCapsulesByKFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            start_after: Some("test_capsule_456".to_string()),
            limit: Some(50),
        };
        assert!(valid_list_msg.validate().is_ok());

        let invalid_limit_msg = QueryMsg::ListCapsulesByKFrag {
            kfrag_id: "test_kfrag_123".to_string(),
            start_after: None,
            limit: Some(0),
        };
        assert!(invalid_limit_msg.validate().is_err());
    }

    #[test]
    fn test_ao_message_tags() {
        let execute_tags = AOMessageTags::new_execute(
            "SubmitKFrag",
            r#"{"kfrag_id":"K","kfrag":"<base64>"}"#,
            "process_123",
            "wallet_addr",
            "2025-10-17T12:00:00Z",
        );

        assert_eq!(execute_tags.app_name, "cwao");
        assert_eq!(execute_tags.action, "SubmitKFrag");
        assert_eq!(execute_tags.read_only, "False");

        let query_tags = AOMessageTags::new_query(
            "GetCFrag",
            r#"{"kfrag_id":"K","capsule_id":"C"}"#,
            "process_123",
            "wallet_addr",
            "2025-10-17T12:00:00Z",
        );

        assert_eq!(query_tags.read_only, "True");
    }
}
