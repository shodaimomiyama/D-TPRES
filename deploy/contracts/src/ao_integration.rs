use cosmwasm_std::{
    to_json_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
};
use serde::{Deserialize, Serialize};

use crate::msg::{ExecuteMsg, ProcessInfo, ProcessStatus};
use crate::state::{
    AOProcessInfo, AOProcessStatus, PendingMessage, ProcessRegistry, ProcessRole,
    CONNECTED_PROCESSES, MESSAGE_QUEUE, PROCESS_REGISTRY,
};

/// AO Network統合のためのヘルパー関数群
pub struct AOIntegration;

impl AOIntegration {
    /// プロセス間メッセージを送信
    pub fn send_ao_message(
        target_process: String,
        message: ExecuteMsg,
    ) -> Result<CosmosMsg, StdError> {
        let wasm_msg = WasmMsg::Execute {
            contract_addr: target_process,
            msg: to_json_binary(&message)?,
            funds: vec![],
        };

        Ok(CosmosMsg::Wasm(wasm_msg))
    }

    /// 新しいプロセスをプロセスレジストリに登録
    pub fn register_process(
        deps: DepsMut,
        env: Env,
        process_id: String,
        wasm_tx_id: String,
        role: ProcessRole,
    ) -> StdResult<()> {
        let process_info = AOProcessInfo {
            process_id: process_id.clone(),
            wasm_tx_id,
            process_role: role,
            spawned_at: env.block.time.seconds(),
            status: AOProcessStatus::Active,
            last_heartbeat: env.block.time.seconds(),
        };

        CONNECTED_PROCESSES.save(deps.storage, process_id, &process_info)?;
        Ok(())
    }

    /// プロセスレジストリを初期化
    pub fn initialize_process_registry(
        deps: DepsMut,
        env: Env,
        process_id: String,
        role: ProcessRole,
    ) -> StdResult<()> {
        let mut registry = ProcessRegistry::new(process_id, role);
        registry.initialization_time = env.block.time.seconds();

        PROCESS_REGISTRY.save(deps.storage, &registry)?;
        Ok(())
    }

    /// Holder-Processを Owner-Process のレジストリに追加
    pub fn add_holder_to_registry(
        deps: DepsMut,
        holder_process_id: String,
    ) -> StdResult<()> {
        let mut registry = PROCESS_REGISTRY.load(deps.storage)?;
        registry.add_holder_process(holder_process_id);
        PROCESS_REGISTRY.save(deps.storage, &registry)?;
        Ok(())
    }

    /// Requester-Processを レジストリに追加
    pub fn add_requester_to_registry(
        deps: DepsMut,
        requester_process_id: String,
    ) -> StdResult<()> {
        let mut registry = PROCESS_REGISTRY.load(deps.storage)?;
        registry.add_requester_process(requester_process_id);
        PROCESS_REGISTRY.save(deps.storage, &registry)?;
        Ok(())
    }

    /// Owner-Processを Holder/Requester のレジストリに設定
    pub fn set_owner_in_registry(
        deps: DepsMut,
        owner_process_id: String,
    ) -> StdResult<()> {
        let mut registry = PROCESS_REGISTRY.load(deps.storage)?;
        registry.set_owner_process(owner_process_id);
        PROCESS_REGISTRY.save(deps.storage, &registry)?;
        Ok(())
    }

    /// プロセスのハートビートを更新
    pub fn update_process_heartbeat(
        deps: DepsMut,
        env: Env,
        process_id: String,
    ) -> StdResult<()> {
        let mut process_info = CONNECTED_PROCESSES.load(deps.storage, process_id.clone())?;
        process_info.update_heartbeat(env.block.time.seconds());
        CONNECTED_PROCESSES.save(deps.storage, process_id, &process_info)?;
        Ok(())
    }

    /// メッセージキューにメッセージを追加
    pub fn queue_message(
        deps: DepsMut,
        env: Env,
        target_process: String,
        message_type: String,
        message_data: Vec<u8>,
    ) -> StdResult<String> {
        let mut pending_msg = PendingMessage::new(target_process, message_type, message_data);
        pending_msg.created_at = env.block.time.seconds();

        let message_id = pending_msg.message_id.clone();
        MESSAGE_QUEUE.save(deps.storage, message_id.clone(), &pending_msg)?;

        Ok(message_id)
    }

    /// キューからメッセージを取得
    pub fn get_queued_message(
        deps: DepsMut,
        message_id: String,
    ) -> StdResult<Option<PendingMessage>> {
        MESSAGE_QUEUE.may_load(deps.storage, message_id)
    }

    /// メッセージの送信を完了としてマーク（キューから削除）
    pub fn complete_message(
        deps: DepsMut,
        message_id: String,
    ) -> StdResult<()> {
        MESSAGE_QUEUE.remove(deps.storage, message_id);
        Ok(())
    }

    /// メッセージの再試行回数を増加
    pub fn retry_message(
        deps: DepsMut,
        message_id: String,
    ) -> StdResult<bool> {
        let mut message = MESSAGE_QUEUE.load(deps.storage, message_id.clone())?;

        if message.can_retry() {
            message.increment_retry();
            MESSAGE_QUEUE.save(deps.storage, message_id, &message)?;
            Ok(true)
        } else {
            // 最大再試行回数に達した場合はキューから削除
            MESSAGE_QUEUE.remove(deps.storage, message_id);
            Ok(false)
        }
    }

    /// 接続されているプロセスのリストを取得
    pub fn get_connected_processes(deps: DepsMut) -> StdResult<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        for item in CONNECTED_PROCESSES.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
            let (_, process_info) = item?;
            processes.push(ProcessInfo {
                process_id: process_info.process_id,
                wasm_tx_id: process_info.wasm_tx_id,
                process_role: process_info.process_role,
                spawned_at: process_info.spawned_at,
                status: Self::convert_ao_status_to_api_status(process_info.status),
            });
        }

        Ok(processes)
    }

    /// プロセス情報を取得
    pub fn get_process_info(
        deps: DepsMut,
        process_id: String,
    ) -> StdResult<Option<ProcessInfo>> {
        if let Some(process_info) = CONNECTED_PROCESSES.may_load(deps.storage, process_id)? {
            Ok(Some(ProcessInfo {
                process_id: process_info.process_id,
                wasm_tx_id: process_info.wasm_tx_id,
                process_role: process_info.process_role,
                spawned_at: process_info.spawned_at,
                status: Self::convert_ao_status_to_api_status(process_info.status),
            }))
        } else {
            Ok(None)
        }
    }

    /// プロセスレジストリを取得
    pub fn get_process_registry(deps: DepsMut) -> StdResult<ProcessRegistry> {
        PROCESS_REGISTRY.load(deps.storage)
    }

    /// 特定の役割のプロセスリストを取得
    pub fn get_processes_by_role(
        deps: DepsMut,
        role: ProcessRole,
    ) -> StdResult<Vec<String>> {
        let mut process_ids = Vec::new();

        for item in CONNECTED_PROCESSES.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
            let (_, process_info) = item?;
            if process_info.process_role == role && process_info.is_active() {
                process_ids.push(process_info.process_id);
            }
        }

        Ok(process_ids)
    }

    /// AO Networkでのプロセス生成をシミュレート（実際のAO SDKでは置き換える）
    pub fn spawn_process_mock(
        _wasm_tx_id: String,
        role: ProcessRole,
        init_data: Vec<u8>,
    ) -> StdResult<String> {
        // モック実装: 実際のAO SDKではspawn APIを呼び出す
        let process_id = format!("{}_{:x}", role.to_string(), init_data.len());
        Ok(process_id)
    }

    // 内部ヘルパー関数
    fn convert_ao_status_to_api_status(status: AOProcessStatus) -> ProcessStatus {
        match status {
            AOProcessStatus::Active => ProcessStatus::Active,
            AOProcessStatus::Inactive => ProcessStatus::Inactive,
            AOProcessStatus::Failed => ProcessStatus::Failed,
            AOProcessStatus::Initializing => ProcessStatus::Initializing,
        }
    }
}

/// Owner-Process用のAO統合ヘルパー
pub struct OwnerAOIntegration;

impl OwnerAOIntegration {
    /// Holder-Processを複数生成
    pub fn spawn_holder_processes(
        mut deps: DepsMut,
        env: Env,
        wasm_tx_id: String,
        holder_count: u32,
    ) -> StdResult<Vec<String>> {
        let mut holder_process_ids = Vec::new();

        for i in 0..holder_count {
            let holder_id = format!("holder_{}", i);
            let init_data = format!("{{\"holder_id\": \"{}\"}}", holder_id).into_bytes();

            let process_id = AOIntegration::spawn_process_mock(
                wasm_tx_id.clone(),
                ProcessRole::Holder,
                init_data,
            )?;

            // プロセスを登録
            AOIntegration::register_process(
                deps.branch(),
                env.clone(),
                process_id.clone(),
                wasm_tx_id.clone(),
                ProcessRole::Holder,
            )?;

            // レジストリに追加
            AOIntegration::add_holder_to_registry(deps.branch(), process_id.clone())?;

            holder_process_ids.push(process_id);
        }

        Ok(holder_process_ids)
    }

    /// Holder-ProcessにkFragを送信
    pub fn send_kfrag_to_holder(
        target_process: String,
        kfrag: crate::msg::KFragDistribution,
        owner_process: String,
    ) -> StdResult<CosmosMsg> {
        let message = ExecuteMsg::SendKFragToHolder {
            target_process: target_process.clone(),
            kfrag,
            owner_process,
        };

        AOIntegration::send_ao_message(target_process, message)
    }
}

/// Holder-Process用のAO統合ヘルパー
pub struct HolderAOIntegration;

impl HolderAOIntegration {
    /// Requester-ProcessにcFragを送信
    pub fn send_cfrag_to_requester(
        target_process: String,
        cfrag: crate::msg::CFragSubmission,
        holder_process: String,
    ) -> StdResult<CosmosMsg> {
        let message = ExecuteMsg::SendCFragToRequester {
            target_process: target_process.clone(),
            cfrag,
            holder_process,
        };

        AOIntegration::send_ao_message(target_process, message)
    }
}

/// Requester-Process用のAO統合ヘルパー
pub struct RequesterAOIntegration;

impl RequesterAOIntegration {
    /// Holder-ProcessにcFrag要求を送信
    pub fn request_cfrag_from_holder(
        target_process: String,
        session_id: String,
        requester_process: String,
    ) -> StdResult<CosmosMsg> {
        let message = ExecuteMsg::RequestCFragFromHolder {
            target_process: target_process.clone(),
            session_id,
            requester_process,
        };

        AOIntegration::send_ao_message(target_process, message)
    }

    /// 複数のHolder-ProcessからcFragを要求
    pub fn request_cfrags_from_holders(
        holder_process_ids: Vec<String>,
        session_id: String,
        requester_process: String,
    ) -> StdResult<Vec<CosmosMsg>> {
        let mut messages = Vec::new();

        for holder_id in holder_process_ids {
            let msg = Self::request_cfrag_from_holder(
                holder_id,
                session_id.clone(),
                requester_process.clone(),
            )?;
            messages.push(msg);
        }

        Ok(messages)
    }
}

/// AO統合機能のテスト用モック
#[cfg(test)]
pub mod mock {
    use super::*;

    pub fn create_mock_process_info(role: ProcessRole) -> AOProcessInfo {
        AOProcessInfo {
            process_id: format!("mock_{}_process", role.to_string()),
            wasm_tx_id: "mock_wasm_tx_id".to_string(),
            process_role: role,
            spawned_at: 1000000,
            status: AOProcessStatus::Active,
            last_heartbeat: 1000000,
        }
    }

    pub fn create_mock_registry() -> ProcessRegistry {
        ProcessRegistry {
            current_process_id: "mock_current_process".to_string(),
            current_role: ProcessRole::Owner,
            initialization_time: 1000000,
            holder_processes: vec![
                "holder_0".to_string(),
                "holder_1".to_string(),
                "holder_2".to_string(),
            ],
            requester_processes: vec!["requester_0".to_string()],
            owner_process: Some("owner_0".to_string()),
        }
    }
}