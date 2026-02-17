use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OperationState {
    Pending,
    Running,
    WaitingForInput,
    Completed,
    Failed,
    Stuck,
    Recovered,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub tool_id: String,
    pub action: String,
    pub state: OperationState,
    pub started_at: Instant,
    pub last_progress_at: Instant,
    pub progress_count: u32,
    pub max_retries: u32,
    pub retry_count: u32,
    pub recovery_strategy: Option<RecoveryStrategy>,
    pub metadata: HashMap<String, String>,
}

impl Operation {
    pub fn new(tool_id: &str, action: &str) -> Self {
        let now = Instant::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tool_id: tool_id.to_string(),
            action: action.to_string(),
            state: OperationState::Pending,
            started_at: now,
            last_progress_at: now,
            progress_count: 0,
            max_retries: 3,
            retry_count: 0,
            recovery_strategy: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_recovery_strategy(mut self, strategy: RecoveryStrategy) -> Self {
        self.recovery_strategy = Some(strategy);
        self
    }

    pub fn is_stuck(&self, timeout: Duration) -> bool {
        self.state == OperationState::Running && self.last_progress_at.elapsed() > timeout
    }

    pub fn mark_progress(&mut self) {
        self.last_progress_at = Instant::now();
        self.progress_count += 1;
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn reset_for_retry(&mut self) {
        self.state = OperationState::Pending;
        self.last_progress_at = Instant::now();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecoveryStrategy {
    Restart,
    SkipStep,
    FallbackAlternative,
    SimplifyRequest,
    ReduceParameters,
    TimeoutFallback,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct StuckDetection {
    pub operation_id: String,
    pub stuck_duration: Duration,
    pub last_progress: Duration,
    pub suggestion: String,
}

pub struct SelfHealer {
    operations: Arc<RwLock<HashMap<String, Operation>>>,
    timeout: Duration,
    check_interval: Duration,
    max_concurrent_recoveries: usize,
    active_recoveries: Arc<RwLock<HashMap<String, RecoveryTask>>>,
}

#[derive(Debug, Clone)]
pub struct RecoveryTask {
    pub operation_id: String,
    pub strategy: RecoveryStrategy,
    pub started_at: Instant,
    pub status: RecoveryStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryStatus {
    InProgress,
    Completed,
    Failed,
    PartialSuccess,
}

impl Default for SelfHealer {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfHealer {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(RwLock::new(HashMap::new())),
            timeout: Duration::from_secs(30),
            check_interval: Duration::from_secs(5),
            max_concurrent_recoveries: 2,
            active_recoveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = interval;
        self
    }

    pub fn with_max_concurrent_recoveries(mut self, max: usize) -> Self {
        self.max_concurrent_recoveries = max;
        self
    }

    pub async fn start_operation(&self, tool_id: &str, action: &str) -> String {
        let operation = Operation::new(tool_id, action);
        let id = operation.id.clone();

        let mut ops = self.operations.write().await;
        ops.insert(id.clone(), operation);

        info!("Started operation {} for {}/{}", id, tool_id, action);
        id
    }

    pub async fn update_state(&self, operation_id: &str, state: OperationState) -> bool {
        let mut ops = self.operations.write().await;
        if let Some(op) = ops.get_mut(operation_id) {
            let state_clone = state.clone();
            op.state = state;
            debug!(
                "Operation {} state updated to {:?}",
                operation_id, state_clone
            );
            true
        } else {
            warn!("Operation {} not found", operation_id);
            false
        }
    }

    pub async fn record_progress(&self, operation_id: &str) -> bool {
        let mut ops = self.operations.write().await;
        if let Some(op) = ops.get_mut(operation_id) {
            op.mark_progress();
            debug!(
                "Operation {} progress recorded: {}",
                operation_id, op.progress_count
            );
            true
        } else {
            false
        }
    }

    pub async fn complete_operation(&self, operation_id: &str) -> bool {
        self.update_state(operation_id, OperationState::Completed)
            .await
    }

    pub async fn fail_operation(&self, operation_id: &str) -> bool {
        self.update_state(operation_id, OperationState::Failed)
            .await
    }

    pub async fn check_for_stuck_operations(&self) -> Vec<StuckDetection> {
        let ops = self.operations.read().await;
        let mut stuck_ops = Vec::new();

        for (id, op) in ops.iter() {
            if op.is_stuck(self.timeout) {
                let suggestion = self.generate_recovery_suggestion(&op);
                stuck_ops.push(StuckDetection {
                    operation_id: id.clone(),
                    stuck_duration: op.last_progress_at.elapsed(),
                    last_progress: Duration::from_secs(op.progress_count as u64),
                    suggestion,
                });
            }
        }

        if !stuck_ops.is_empty() {
            warn!("Found {} stuck operations", stuck_ops.len());
        }

        stuck_ops
    }

    fn generate_recovery_suggestion(&self, operation: &Operation) -> String {
        match operation.recovery_strategy.as_ref() {
            Some(RecoveryStrategy::Restart) => {
                format!(
                    "Operation {}/{} appears stuck. Suggestion: Restart the operation from beginning.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::SkipStep) => {
                format!(
                    "Operation {}/{} appears stuck. Suggestion: Skip current step and proceed.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::FallbackAlternative) => {
                format!(
                    "Operation {}/{} appears stuck. Suggestion: Use alternative approach.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::SimplifyRequest) => {
                format!(
                    "Operation {}/{} appears stuck. Suggestion: Simplify the request.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::ReduceParameters) => {
                format!(
                    "Operation {}/{} appears stuck. Suggestion: Reduce number of parameters.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::TimeoutFallback) => {
                format!(
                    "Operation {}/{} timed out. Suggestion: Use faster timeout fallback.",
                    operation.tool_id, operation.action
                )
            }
            Some(RecoveryStrategy::Custom(name)) => {
                format!(
                    "Operation {}/{} appears stuck. Custom strategy: {}",
                    operation.tool_id, operation.action, name
                )
            }
            None => {
                format!(
                    "Operation {}/{} appears stuck. No recovery strategy set.",
                    operation.tool_id, operation.action
                )
            }
        }
    }

    pub async fn attempt_recovery(&self, operation_id: &str, strategy: RecoveryStrategy) -> bool {
        let active = self.active_recoveries.read().await;
        if active.len() >= self.max_concurrent_recoveries {
            warn!(
                "Max concurrent recoveries reached, cannot start new recovery for {}",
                operation_id
            );
            return false;
        }
        drop(active);

        let mut recoveries = self.active_recoveries.write().await;
        recoveries.insert(
            operation_id.to_string(),
            RecoveryTask {
                operation_id: operation_id.to_string(),
                strategy: strategy.clone(),
                started_at: Instant::now(),
                status: RecoveryStatus::InProgress,
            },
        );
        drop(recoveries);

        let mut ops = self.operations.write().await;
        if let Some(op) = ops.get_mut(operation_id) {
            op.state = OperationState::Running;

            match strategy {
                RecoveryStrategy::Restart => {
                    op.reset_for_retry();
                    op.increment_retry();
                    info!(
                        "Recovery strategy 'Restart' applied to operation {}",
                        operation_id
                    );
                }
                RecoveryStrategy::SkipStep => {
                    op.mark_progress();
                    info!(
                        "Recovery strategy 'SkipStep' applied to operation {}",
                        operation_id
                    );
                }
                RecoveryStrategy::FallbackAlternative
                | RecoveryStrategy::SimplifyRequest
                | RecoveryStrategy::ReduceParameters => {
                    op.metadata
                        .insert("fallback".to_string(), "true".to_string());
                    op.mark_progress();
                    info!(
                        "Recovery strategy '{:?}' applied to operation {}",
                        strategy, operation_id
                    );
                }
                RecoveryStrategy::TimeoutFallback => {
                    op.metadata
                        .insert("timeout_reduced".to_string(), "true".to_string());
                    op.mark_progress();
                    info!(
                        "Recovery strategy 'TimeoutFallback' applied to operation {}",
                        operation_id
                    );
                }
                RecoveryStrategy::Custom(name) => {
                    op.metadata.insert("custom_recovery".to_string(), name);
                    op.mark_progress();
                }
            }

            true
        } else {
            warn!("Cannot recover: operation {} not found", operation_id);
            false
        }
    }

    pub async fn mark_recovery_completed(&self, operation_id: &str, success: bool) {
        let mut recoveries = self.active_recoveries.write().await;
        if let Some(task) = recoveries.get_mut(operation_id) {
            task.status = if success {
                RecoveryStatus::Completed
            } else {
                RecoveryStatus::Failed
            };
        }

        let mut ops = self.operations.write().await;
        if let Some(op) = ops.get_mut(operation_id) {
            op.state = if success {
                OperationState::Recovered
            } else {
                OperationState::Failed
            };
        }
    }

    pub async fn get_operation(&self, operation_id: &str) -> Option<Operation> {
        let ops = self.operations.read().await;
        ops.get(operation_id).cloned()
    }

    pub async fn get_active_operations(&self) -> Vec<Operation> {
        let ops = self.operations.read().await;
        ops.values()
            .filter(|op| {
                op.state == OperationState::Running || op.state == OperationState::WaitingForInput
            })
            .cloned()
            .collect()
    }

    pub async fn get_recovery_stats(&self) -> HashMap<String, u32> {
        let mut stats = HashMap::new();

        let ops = self.operations.read().await;
        for op in ops.values() {
            *stats.entry(format!("{:?}", op.state)).or_insert(0) += 1;
        }

        let recoveries = self.active_recoveries.read().await;
        stats.insert("active_recoveries".to_string(), recoveries.len() as u32);

        stats
    }

    pub async fn cleanup_completed(&self, older_than: Duration) -> usize {
        let mut ops = self.operations.write().await;
        let before = ops.len();

        ops.retain(|_, op| {
            op.state == OperationState::Running
                || op.state == OperationState::Pending
                || op.state == OperationState::WaitingForInput
                || op.started_at.elapsed() < older_than
        });

        before - ops.len()
    }

    pub async fn start_auto_recovery_loop(
        &self,
        shutdown_signal: Option<tokio::sync::oneshot::Receiver<()>>,
    ) {
        info!(
            "Starting auto-recovery loop with check interval {:?}",
            self.check_interval
        );

        let _shutdown_rx = shutdown_signal;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(self.check_interval) => {
                    let stuck = self.check_for_stuck_operations().await;

                    for detection in stuck {
                        info!("Auto-recovery: detected stuck operation {}", detection.operation_id);

                        let ops = self.operations.read().await;
                        if let Some(op) = ops.get(&detection.operation_id) {
                            let strategy = self.determine_recovery_strategy(op);
                            drop(ops);

                            let success = self.attempt_recovery(&detection.operation_id, strategy).await;
                            if success {
                                info!("Auto-recovery: successfully recovered operation {}", detection.operation_id);
                            } else {
                                warn!("Auto-recovery: failed to recover operation {}", detection.operation_id);
                            }
                        }
                    }
                }
                _ = std::future::pending::<()>() => {
                    info!("Auto-recovery loop received shutdown signal");
                    break;
                }
            }
        }
    }

    fn determine_recovery_strategy(&self, operation: &Operation) -> RecoveryStrategy {
        if let Some(ref strategy) = operation.recovery_strategy {
            return strategy.clone();
        }

        if operation.retry_count < operation.max_retries / 2 {
            RecoveryStrategy::Restart
        } else if operation.retry_count < operation.max_retries {
            RecoveryStrategy::SimplifyRequest
        } else {
            RecoveryStrategy::FallbackAlternative
        }
    }
}
