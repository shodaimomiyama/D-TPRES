//! ProcessManagementService - プロセス管理サービス
//!
//! ProcessEntityの管理とマルチロール状態管理を担当します。
//! AO環境でのプロセスライフサイクル管理とビジネスロジックを提供します。

use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::entities::process::{
    HolderData, OwnerData, PerformanceMetrics, ProcessEntity, RequesterData,
};
use crate::domain::entities::value_objects::{CryptoPhase, ProcessRole};
use crate::domain::repositories::process::ProcessEntityRepository;
use crate::service::error::{BusinessException, ServiceError, ServiceResult};

/// プロセス設定
#[derive(Debug, Clone)]
pub struct ProcessConfig {
    /// 最大シークレット数
    pub max_secrets: u32,
    /// パフォーマンストラッキング有効化
    pub performance_tracking: bool,
    /// 最大Holder数
    pub max_holders: u32,
    /// 最大同時リクエスト数
    pub max_concurrent_requests: u32,
}

/// ロールデータ
#[derive(Debug, Clone)]
pub enum RoleData {
    Owner(OwnerRoleData),
    Holder(HolderRoleData),
    Requester(RequesterRoleData),
}

/// Ownerロール用データ
#[derive(Debug, Clone)]
pub struct OwnerRoleData {
    pub master_key_id: String,
}

/// Holderロール用データ
#[derive(Debug, Clone)]
pub struct HolderRoleData {
    pub storage_capacity: u64,
    pub online_status: bool,
}

/// Requesterロール用データ
#[derive(Debug, Clone)]
pub struct RequesterRoleData {
    pub request_quota: u32,
}

/// パフォーマンス分析結果
#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    pub average_response_time: f64,
    pub success_rate: f64,
    pub peak_load: u64,
    pub recommendations: Vec<String>,
}

/// ロール分散計画
#[derive(Debug, Clone)]
pub struct RoleDistributionPlan {
    pub recommended_assignments: HashMap<String, Vec<ProcessRole>>,
    pub load_balance_score: f64,
    pub rationale: String,
}

/// 時間範囲
#[derive(Debug, Clone)]
pub struct TimeRange {
    pub start_timestamp: u64,
    pub end_timestamp: u64,
}

/// ProcessManagementService trait
pub trait ProcessManagementService: Send + Sync {
    /// プロセス初期化（ビジネスロジックを含む）
    fn initialize_process_with_validation(
        &self,
        process_id: &str,
        initial_roles: &[ProcessRole],
        config: ProcessConfig,
    ) -> ServiceResult<ProcessEntity>;

    /// ロール追加（互換性チェックを含む）
    fn validate_and_add_role(
        &self,
        process_id: &str,
        role: ProcessRole,
        role_data: RoleData,
    ) -> ServiceResult<()>;

    /// パフォーマンスメトリクス分析
    fn analyze_performance_metrics(
        &self,
        process_id: &str,
        time_range: TimeRange,
    ) -> ServiceResult<PerformanceAnalysis>;

    /// マルチロール最適化（ビジネスロジック）
    fn optimize_role_distribution(
        &self,
        process_ids: &[String],
    ) -> ServiceResult<RoleDistributionPlan>;

    /// 信頼性スコア計算（ビジネスロジック）
    fn calculate_reliability_score(&self, process_id: &str) -> ServiceResult<f64>;

    /// プロセス状態遷移の検証
    fn validate_phase_transition(
        &self,
        process_id: &str,
        new_phase: CryptoPhase,
    ) -> ServiceResult<()>;
}

/// ProcessManagementService実装
pub struct ProcessManagementServiceImpl {
    process_repository:
        Arc<dyn ProcessEntityRepository<Error = crate::domain::errors::DomainError>>,
}

impl ProcessManagementServiceImpl {
    /// 新しいProcessManagementServiceインスタンスを作成
    pub fn new(
        process_repository: Arc<
            dyn ProcessEntityRepository<Error = crate::domain::errors::DomainError>,
        >,
    ) -> Self {
        Self { process_repository }
    }

    /// 初期ロールの検証
    fn validate_initial_role(&self, role: &ProcessRole) -> ServiceResult<()> {
        match role {
            ProcessRole::Owner => Ok(()),
            ProcessRole::Holder => Ok(()),
            ProcessRole::Requester => Ok(()),
        }
    }

    /// 設定の検証
    fn validate_config(&self, config: &ProcessConfig) -> ServiceResult<()> {
        if config.max_secrets == 0 {
            return Err(ServiceError::validation_error(
                "max_secrets must be greater than 0",
            ));
        }

        if config.max_holders == 0 {
            return Err(ServiceError::validation_error(
                "max_holders must be greater than 0",
            ));
        }

        if config.max_concurrent_requests == 0 {
            return Err(ServiceError::validation_error(
                "max_concurrent_requests must be greater than 0",
            ));
        }

        Ok(())
    }

    /// ロール互換性チェック
    fn validate_role_compatibility(
        &self,
        existing_roles: &[ProcessRole],
        new_role: &ProcessRole,
    ) -> ServiceResult<()> {
        // Ownerロールの重複チェック
        if matches!(new_role, ProcessRole::Owner) {
            if existing_roles
                .iter()
                .any(|r| matches!(r, ProcessRole::Owner))
            {
                return Err(ServiceError::Business(BusinessException::RoleConflict(
                    "Owner role already exists".to_string(),
                )));
            }
        }

        // Holderのチェック（容量はHolderDataで管理）
        // ProcessRole自体には容量情報はない

        Ok(())
    }
}

impl ProcessManagementService for ProcessManagementServiceImpl {
    fn initialize_process_with_validation(
        &self,
        process_id: &str,
        initial_roles: &[ProcessRole],
        config: ProcessConfig,
    ) -> ServiceResult<ProcessEntity> {
        // ビジネスロジック：ロール検証
        for role in initial_roles {
            self.validate_initial_role(role)?;
        }

        // ビジネスロジック：設定検証
        self.validate_config(&config)?;

        // ProcessEntityの作成
        let mut process = ProcessEntity {
            process_id: process_id.to_string(),
            process_name: format!("Process {}", process_id),
            active_roles: vec![],
            owner_data: None,
            holder_data: None,
            requester_data: None,
            configuration: HashMap::new(),
            supported_crypto_operations: vec![],
            performance_metrics: PerformanceMetrics {
                successful_operations: 0,
                failed_operations: 0,
                average_response_time_ms: 0,
                last_updated_at: 0,
            },
            created_at: 0, // TODO: Use actual timestamp
            updated_at: 0,
            version: 1,
        };

        // 初期ロールの設定
        for role in initial_roles {
            match role {
                ProcessRole::Owner => {
                    process.owner_data = Some(OwnerData {
                        owner_public_key: vec![0u8; 33], // Placeholder public key
                        secret_indices: HashMap::new(),
                        owner_config: HashMap::new(),
                    });
                }
                ProcessRole::Holder => {
                    process.holder_data = Some(HolderData {
                        held_fragments: HashMap::new(),
                        fragments_by_condition: HashMap::new(),
                        reliability_score: 1.0,
                        completed_reencryptions: 0,
                        max_fragment_capacity: 50, // Default capacity
                        current_load: 0,
                    });
                }
                ProcessRole::Requester => {
                    process.requester_data = Some(RequesterData {
                        active_requests: Vec::new(),
                        active_reencryptions: Vec::new(),
                        completed_requests: 0,
                        success_rate: 0.0,
                        average_processing_time_ms: 0,
                        requester_config: HashMap::new(),
                    });
                }
            }
            process.active_roles.push(role.clone());
        }

        // Repositoryに保存
        self.process_repository.create(&process)?;

        Ok(process)
    }

    fn validate_and_add_role(
        &self,
        process_id: &str,
        new_role: ProcessRole,
        role_data: RoleData,
    ) -> ServiceResult<()> {
        // Repositoryから現在の状態を取得
        let mut process = self
            .process_repository
            .find_by_id(&process_id.to_string())?
            .ok_or_else(|| ServiceError::not_found(format!("Process {} not found", process_id)))?;

        // ビジネスロジック：ロール互換性チェック
        self.validate_role_compatibility(&process.active_roles, &new_role)?;

        // ロールデータの設定
        match (&new_role, role_data) {
            (ProcessRole::Owner, RoleData::Owner(data)) => {
                process.owner_data = Some(OwnerData {
                    owner_public_key: vec![0u8; 33], // TODO: Get from data
                    secret_indices: HashMap::new(),
                    owner_config: {
                        let mut config = HashMap::new();
                        config.insert("master_key_id".to_string(), data.master_key_id);
                        config
                    },
                });
            }
            (ProcessRole::Holder, RoleData::Holder(data)) => {
                process.holder_data = Some(HolderData {
                    held_fragments: HashMap::new(),
                    fragments_by_condition: HashMap::new(),
                    reliability_score: 1.0,
                    completed_reencryptions: 0,
                    max_fragment_capacity: data.storage_capacity,
                    current_load: 0,
                });
            }
            (ProcessRole::Requester, RoleData::Requester(_data)) => {
                process.requester_data = Some(RequesterData {
                    active_requests: Vec::new(),
                    active_reencryptions: Vec::new(),
                    completed_requests: 0,
                    success_rate: 0.0,
                    average_processing_time_ms: 0,
                    requester_config: HashMap::new(),
                });
            }
            _ => {
                return Err(ServiceError::validation_error("Role data mismatch"));
            }
        }

        // プロセス更新
        process.active_roles.push(new_role);
        self.process_repository.update(&process)?;

        Ok(())
    }

    fn analyze_performance_metrics(
        &self,
        process_id: &str,
        _time_range: TimeRange,
    ) -> ServiceResult<PerformanceAnalysis> {
        // プロセスを取得
        let process = self
            .process_repository
            .find_by_id(&process_id.to_string())?
            .ok_or_else(|| ServiceError::not_found(format!("Process {} not found", process_id)))?;

        let metrics = &process.performance_metrics;

        // ビジネスロジック：パフォーマンス分析
        let total_operations = metrics.successful_operations + metrics.failed_operations;
        let success_rate = if total_operations > 0 {
            (metrics.successful_operations as f64 / total_operations as f64) * 100.0
        } else {
            0.0
        };

        let mut recommendations = Vec::new();

        if success_rate < 90.0 {
            recommendations
                .push("Consider reviewing error logs and improving error handling".to_string());
        }

        if metrics.average_response_time_ms > 1000 {
            recommendations
                .push("Response time is high, consider optimizing performance".to_string());
        }

        // Note: uptime_percentage is not tracked in PerformanceMetrics

        Ok(PerformanceAnalysis {
            average_response_time: metrics.average_response_time_ms as f64,
            success_rate,
            peak_load: 0, // TODO: 実装
            recommendations,
        })
    }

    fn optimize_role_distribution(
        &self,
        process_ids: &[String],
    ) -> ServiceResult<RoleDistributionPlan> {
        // ビジネスロジック：ロール分散の最適化
        let mut recommended_assignments = HashMap::new();

        for process_id in process_ids {
            let process = self.process_repository.find_by_id(process_id)?;

            if let Some(process) = process {
                let mut recommendations = Vec::new();

                // Holderロールの推奨
                if !process
                    .active_roles
                    .iter()
                    .any(|r| matches!(r, ProcessRole::Holder))
                {
                    recommendations.push(ProcessRole::Holder);
                }

                recommended_assignments.insert(process_id.clone(), recommendations);
            }
        }

        Ok(RoleDistributionPlan {
            recommended_assignments,
            load_balance_score: 0.85, // プレースホルダー
            rationale: "Optimized for balanced load distribution".to_string(),
        })
    }

    fn calculate_reliability_score(&self, process_id: &str) -> ServiceResult<f64> {
        let process = self
            .process_repository
            .find_by_id(&process_id.to_string())?
            .ok_or_else(|| ServiceError::not_found(format!("Process {} not found", process_id)))?;

        let metrics = &process.performance_metrics;

        // ビジネスロジック：信頼性スコア計算
        let total_operations = metrics.successful_operations + metrics.failed_operations;
        let success_factor = if total_operations > 0 {
            metrics.successful_operations as f64 / total_operations as f64
        } else {
            1.0
        };
        let penalty = metrics.failed_operations as f64 * 0.01;

        let score = success_factor - penalty;
        Ok(score.max(0.0).min(1.0))
    }

    fn validate_phase_transition(
        &self,
        process_id: &str,
        new_phase: CryptoPhase,
    ) -> ServiceResult<()> {
        let process = self
            .process_repository
            .find_by_id(&process_id.to_string())?
            .ok_or_else(|| ServiceError::not_found(format!("Process {} not found", process_id)))?;

        // ビジネスロジック：フェーズ遷移の検証
        // TODO: ProcessEntityにphaseフィールドが必要
        let current_phase = CryptoPhase::Initialize;

        let is_valid = match (process.active_roles.first(), current_phase, &new_phase) {
            // Ownerの遷移
            (Some(ProcessRole::Owner), CryptoPhase::Initialize, CryptoPhase::SecretSharing) => true,
            (
                Some(ProcessRole::Owner),
                CryptoPhase::SecretSharing,
                CryptoPhase::KeyFragmentation,
            ) => true,
            (
                Some(ProcessRole::Owner),
                CryptoPhase::KeyFragmentation,
                CryptoPhase::ProxyReencryption,
            ) => true,

            // Holderの遷移
            (Some(ProcessRole::Holder), CryptoPhase::Initialize, CryptoPhase::KeyFragmentation) => {
                true
            }
            (
                Some(ProcessRole::Holder),
                CryptoPhase::KeyFragmentation,
                CryptoPhase::ProxyReencryption,
            ) => true,

            // Requesterの遷移
            (Some(ProcessRole::Requester), CryptoPhase::Initialize, CryptoPhase::AccessRequest) => {
                true
            }
            (
                Some(ProcessRole::Requester),
                CryptoPhase::AccessRequest,
                CryptoPhase::SecretRecovery,
            ) => true,

            _ => false,
        };

        if !is_valid {
            Err(ServiceError::Business(
                BusinessException::InvalidProcessState(format!(
                    "Cannot transition from {:?} to {:?} for role {:?}",
                    current_phase,
                    new_phase,
                    process.active_roles.first()
                )),
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: MockProcessEntityRepository is not implemented yet
    // use super::*;
    // use crate::domain::repositories::process::MockProcessEntityRepository;

    // TODO: Enable when MockProcessEntityRepository is implemented
    // #[test]
    // fn test_process_initialization() {
    //     let mut mock_repo = MockProcessEntityRepository::new();
    //     mock_repo.expect_save().returning(|_| Ok(()));
    //
    //     let service = ProcessManagementServiceImpl::new(Arc::new(mock_repo));
    //
    //     let config = ProcessConfig {
    //         max_secrets: 100,
    //         performance_tracking: true,
    //         max_holders: 10,
    //         max_concurrent_requests: 50,
    //     };
    //
    //     let result = service.initialize_process_with_validation(
    //         "test-process",
    //         &[ProcessRole::Owner],
    //         config,
    //     );
    //
    //     assert!(result.is_ok());
    // }

    // TODO: Enable when MockProcessEntityRepository is implemented
    // #[test]
    // fn test_reliability_score_calculation() {
    //     let mut mock_repo = MockProcessEntityRepository::new();
    //
    //     let process = ProcessEntity::new(
    //         "test-process".to_string(),
    //         vec![ProcessRole::Owner],
    //         PerformanceMetrics {
    //             total_operations: 100,
    //             successful_operations: 95,
    //             failed_operations: 5,
    //             average_response_time_ms: 50.0,
    //             uptime_percentage: 99.5,
    //             last_updated: 0,
    //         },
    //     );
    //
    //     mock_repo.expect_find_by_id()
    //         .returning(move |_| Ok(Some(process.clone())));
    //
    //     let service = ProcessManagementServiceImpl::new(Arc::new(mock_repo));
    //     let score = service.calculate_reliability_score("test-process").unwrap();
    //
    //     assert!(score > 0.9);
    //     assert!(score <= 1.0);
    // }
}
