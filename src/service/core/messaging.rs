//! MessageRoutingService - メッセージルーティングサービス
//!
//! AOプロセス間の非同期メッセージングとルーティングを管理します。
//! プロセス発見、メッセージ配信、応答収集機能を提供します。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::domain::entities::value_objects::ProcessRole;
use crate::domain::repositories::process::ProcessEntityRepository;
use crate::service::core::crypto::CipherFragment;
use crate::service::error::{ServiceError, ServiceResult, SystemException};

/// メッセージID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageId(String);

impl MessageId {
    /// 新しいメッセージIDを生成
    pub fn new() -> Self {
        // TODO: 実際のUUID生成を使用
        Self(format!(
            "msg_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ))
    }

    /// 文字列からMessageIdを作成
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// 内部の文字列を取得
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// プロセスメッセージ
#[derive(Debug, Clone)]
pub struct ProcessMessage {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub tags: HashMap<String, String>,
    pub created_at: u64,
}

/// メッセージタイプ
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    DistributeKFrag,
    RequestReencryption,
    CollectCFrag,
    HealthCheck,
    Custom(String),
}

/// メッセージレスポンス
#[derive(Debug, Clone)]
pub struct MessageResponse {
    pub message_id: MessageId,
    pub from_process: String,
    pub response_type: ResponseType,
    pub payload: Vec<u8>,
    pub received_at: u64,
}

/// レスポンスタイプ
#[derive(Debug, Clone)]
pub enum ResponseType {
    Success,
    Error(String),
    Partial,
}

/// プロセス情報
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub process_id: String,
    pub roles: Vec<ProcessRole>,
    pub is_online: bool,
    pub capacity: Option<u64>,
    pub last_seen: u64,
}

/// 再暗号化リクエスト
#[derive(Debug, Clone)]
pub struct ReencryptionRequest {
    pub capsule_id: String,
    pub requester_id: String,
    pub threshold: u8,
}

/// MessageRoutingService trait
pub trait MessageRoutingService: Send + Sync {
    /// メッセージ送信
    fn send_message(
        &self,
        target_process_id: &str,
        message: ProcessMessage,
    ) -> ServiceResult<MessageId>;

    /// ブロードキャストメッセージ送信
    fn broadcast_message(
        &self,
        target_processes: &[String],
        message: ProcessMessage,
    ) -> ServiceResult<Vec<MessageId>>;

    /// 応答待機
    fn wait_for_responses(
        &self,
        message_ids: &[MessageId],
        timeout: Duration,
    ) -> ServiceResult<Vec<MessageResponse>>;

    /// 閾値応答待機
    fn wait_for_threshold_responses(
        &self,
        message_ids: &[MessageId],
        threshold: usize,
        timeout: Duration,
    ) -> ServiceResult<Vec<MessageResponse>>;

    /// オンラインプロセス発見
    fn discover_online_processes(
        &self,
        role_filter: Option<ProcessRole>,
        capacity_requirement: Option<u64>,
    ) -> ServiceResult<Vec<ProcessInfo>>;

    /// cFrag収集
    fn collect_cfrags(
        &self,
        holder_processes: &[String],
        reencryption_request: ReencryptionRequest,
        threshold: u8,
    ) -> ServiceResult<Vec<CipherFragment>>;
}

/// MessageRoutingService実装
pub struct MessageRoutingServiceImpl {
    process_repository:
        Arc<dyn ProcessEntityRepository<Error = crate::domain::errors::DomainError>>,
    // AO環境では実際のメッセージ送信は外部システムが担当
    // ここではシミュレーション用の構造を保持
    pending_responses: Arc<RwLock<HashMap<MessageId, Vec<MessageResponse>>>>,
}

impl MessageRoutingServiceImpl {
    /// 新しいMessageRoutingServiceインスタンスを作成
    pub fn new(
        process_repository: Arc<
            dyn ProcessEntityRepository<Error = crate::domain::errors::DomainError>,
        >,
    ) -> Self {
        Self {
            process_repository,
            pending_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// メッセージIDを生成
    fn generate_message_id(&self) -> MessageId {
        MessageId::new()
    }

    /// レスポンスをシミュレート（テスト用）
    fn simulate_response(&self, message_id: &MessageId, from_process: &str) -> MessageResponse {
        MessageResponse {
            message_id: message_id.clone(),
            from_process: from_process.to_string(),
            response_type: ResponseType::Success,
            payload: vec![0u8; 32], // プレースホルダー
            received_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl MessageRoutingService for MessageRoutingServiceImpl {
    fn send_message(
        &self,
        target_process_id: &str,
        _message: ProcessMessage,
    ) -> ServiceResult<MessageId> {
        // 入力検証
        if target_process_id.is_empty() {
            return Err(ServiceError::validation_error(
                "Target process ID cannot be empty",
            ));
        }

        // メッセージID生成
        let message_id = self.generate_message_id();

        // TODO: 実際のAOメッセージ送信実装
        // 現在はプレースホルダー実装

        // レスポンスのシミュレーション（テスト用）
        let response = self.simulate_response(&message_id, target_process_id);
        self.pending_responses
            .write()
            .unwrap()
            .entry(message_id.clone())
            .or_insert_with(Vec::new)
            .push(response);

        Ok(message_id)
    }

    fn broadcast_message(
        &self,
        target_processes: &[String],
        message: ProcessMessage,
    ) -> ServiceResult<Vec<MessageId>> {
        // 入力検証
        if target_processes.is_empty() {
            return Err(ServiceError::validation_error(
                "No target processes specified",
            ));
        }

        let mut message_ids = Vec::new();

        // 各プロセスにメッセージを送信
        for process_id in target_processes {
            match self.send_message(process_id, message.clone()) {
                Ok(id) => message_ids.push(id),
                Err(e) => {
                    // 個別の失敗は記録するが、全体の処理は継続
                    eprintln!("Failed to send message to {}: {}", process_id, e);
                }
            }
        }

        if message_ids.is_empty() {
            Err(ServiceError::System(SystemException::NetworkError(
                "Failed to send any messages".to_string(),
            )))
        } else {
            Ok(message_ids)
        }
    }

    fn wait_for_responses(
        &self,
        message_ids: &[MessageId],
        _timeout: Duration,
    ) -> ServiceResult<Vec<MessageResponse>> {
        // 入力検証
        if message_ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut responses = Vec::new();
        let pending = self.pending_responses.read().unwrap();

        for message_id in message_ids {
            if let Some(msg_responses) = pending.get(message_id) {
                responses.extend(msg_responses.clone());
            }
        }

        Ok(responses)
    }

    fn wait_for_threshold_responses(
        &self,
        message_ids: &[MessageId],
        threshold: usize,
        timeout: Duration,
    ) -> ServiceResult<Vec<MessageResponse>> {
        // 入力検証
        if threshold == 0 {
            return Err(ServiceError::validation_error(
                "Threshold must be greater than 0",
            ));
        }

        let responses = self.wait_for_responses(message_ids, timeout)?;

        if responses.len() >= threshold {
            Ok(responses.into_iter().take(threshold).collect())
        } else {
            Err(ServiceError::Business(
                crate::service::error::BusinessException::ThresholdNotMet {
                    required: threshold,
                    actual: responses.len(),
                },
            ))
        }
    }

    fn discover_online_processes(
        &self,
        role_filter: Option<ProcessRole>,
        capacity_requirement: Option<u64>,
    ) -> ServiceResult<Vec<ProcessInfo>> {
        // TODO: 実際の実装では、AOネットワークからプロセス情報を取得
        // 現在はRepositoryから取得

        let all_processes = self.process_repository.find_all()?;

        let mut process_infos = Vec::new();

        for process in all_processes {
            // ロールフィルタリング
            if let Some(ref filter_role) = role_filter {
                let has_role = process
                    .active_roles
                    .iter()
                    .any(|r| std::mem::discriminant(r) == std::mem::discriminant(filter_role));

                if !has_role {
                    continue;
                }
            }

            // 容量フィルタリング
            if let Some(min_capacity) = capacity_requirement {
                let meets_capacity = process
                    .holder_data
                    .as_ref()
                    .map(|h| h.max_fragment_capacity >= min_capacity)
                    .unwrap_or(false);

                if !meets_capacity {
                    continue;
                }
            }

            process_infos.push(ProcessInfo {
                process_id: process.process_id.clone(),
                roles: process.active_roles.clone(),
                is_online: true, // TODO: 実際のオンライン状態を確認
                capacity: process
                    .holder_data
                    .as_ref()
                    .map(|h| h.max_fragment_capacity),
                last_seen: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }

        Ok(process_infos)
    }

    fn collect_cfrags(
        &self,
        holder_processes: &[String],
        reencryption_request: ReencryptionRequest,
        threshold: u8,
    ) -> ServiceResult<Vec<CipherFragment>> {
        // 入力検証
        if holder_processes.is_empty() {
            return Err(ServiceError::validation_error(
                "No holder processes specified",
            ));
        }

        if threshold == 0 {
            return Err(ServiceError::validation_error(
                "Threshold must be greater than 0",
            ));
        }

        // 再暗号化リクエストメッセージを作成
        let message = ProcessMessage {
            message_type: MessageType::RequestReencryption,
            payload: vec![0u8; 128], // TODO: 実際のリクエストデータをシリアライズ
            tags: {
                let mut tags = HashMap::new();
                tags.insert(
                    "capsule_id".to_string(),
                    reencryption_request.capsule_id.clone(),
                );
                tags.insert(
                    "requester_id".to_string(),
                    reencryption_request.requester_id.clone(),
                );
                tags
            },
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Holder群にブロードキャスト
        let message_ids = self.broadcast_message(holder_processes, message)?;

        // 閾値数の応答を待機
        let responses = self.wait_for_threshold_responses(
            &message_ids,
            threshold as usize,
            Duration::from_secs(30),
        )?;

        // レスポンスからcFragを抽出
        let mut cfrags = Vec::new();
        for (i, response) in responses.iter().enumerate() {
            // TODO: 実際のcFragデシリアライズ
            cfrags.push(CipherFragment {
                fragment_id: i as u8,
                capsule_fragment: response.payload.clone(),
                proof: vec![0u8; 96], // プレースホルダー
            });
        }

        Ok(cfrags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO: Enable when MockProcessEntityRepository is implemented
    // use crate::domain::repositories::process::MockProcessEntityRepository;

    #[test]
    fn test_message_id_generation() {
        println!("\n=== MessageRoutingService: Message ID Generation Test ===");
        println!("【テスト内容】: AOメッセージの一意ID生成機能を検証");
        println!("【テスト対象】: MessageId::new()メソッド");
        println!("【期待結果】: タイムスタンプベースで一意のIDが生成される");
        
        println!("\n1. 最初のメッセージIDを生成...");
        let id1 = MessageId::new();
        println!("   生成されたID: {}", id1.as_str());
        
        // IDの構造を解析
        let id1_parts: Vec<&str> = id1.as_str().split('_').collect();
        if id1_parts.len() == 2 {
            println!("   - プレフィックス: {}", id1_parts[0]);
            println!("   - タイムスタンプ: {} (ミリ秒)", id1_parts[1]);
        }
        
        // 異なるタイムスタンプを保証するため、1ミリ秒待機
        println!("\n2. タイムスタンプの変更を保証するため1ミリ秒待機...");
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        println!("\n3. 2番目のメッセージIDを生成...");
        let id2 = MessageId::new();
        println!("   生成されたID: {}", id2.as_str());
        
        let id2_parts: Vec<&str> = id2.as_str().split('_').collect();
        if id2_parts.len() == 2 {
            println!("   - プレフィックス: {}", id2_parts[0]);
            println!("   - タイムスタンプ: {} (ミリ秒)", id2_parts[1]);
        }
        
        println!("\n4. 一意性の検証:");
        assert_ne!(id1, id2);
        println!("   ✓ ID1とID2は異なる");
        
        if id1_parts.len() == 2 && id2_parts.len() == 2 {
            let timestamp1: u128 = id1_parts[1].parse().unwrap_or(0);
            let timestamp2: u128 = id2_parts[1].parse().unwrap_or(0);
            println!("   ✓ タイムスタンプの差: {}ミリ秒", timestamp2 - timestamp1);
        }
        
        println!("\n✅ テスト成功: メッセージIDは一意に生成されました！");
    }

    // TODO: Enable when MockProcessEntityRepository is implemented
    // #[test]
    // fn test_send_message() {
    //     let mock_repo = MockProcessEntityRepository::new();
    //     let service = MessageRoutingServiceImpl::new(Arc::new(mock_repo));
    //
    //     let message = ProcessMessage {
    //         message_type: MessageType::HealthCheck,
    //         payload: vec![1, 2, 3],
    //         tags: HashMap::new(),
    //         created_at: 0,
    //     };
    //
    //     let result = service.send_message("test-process", message);
    //     assert!(result.is_ok());
    // }

    // TODO: Enable when MockProcessEntityRepository is implemented
    // #[test]
    // fn test_broadcast_validation() {
    //     let mock_repo = MockProcessEntityRepository::new();
    //     let service = MessageRoutingServiceImpl::new(Arc::new(mock_repo));
    //
    //     let message = ProcessMessage {
    //         message_type: MessageType::HealthCheck,
    //         payload: vec![],
    //         tags: HashMap::new(),
    //         created_at: 0,
    //     };
    //
    //     // 空のターゲットリスト
    //     let result = service.broadcast_message(&[], message);
    //     assert!(result.is_err());
    // }
}
