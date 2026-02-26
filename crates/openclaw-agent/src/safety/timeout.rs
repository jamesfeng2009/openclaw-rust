//! Timeout Controller - 超时控制器
//!
//! 提供运行时超时控制：
//! - 单次操作超时
//! - 总体运行超时
//! - 滑动窗口超时

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeoutConfig {
    pub operation_timeout_ms: u64,
    pub total_timeout_ms: u64,
    pub idle_timeout_ms: u64,
    pub warn_threshold_ms: u64,
}

impl TimeoutConfig {
    pub fn operation_timeout(&self) -> Duration {
        Duration::from_millis(self.operation_timeout_ms)
    }

    pub fn total_timeout(&self) -> Duration {
        Duration::from_millis(self.total_timeout_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutState {
    #[default]
    Idle,
    Running,
    Warned,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimeoutStats {
    pub start_time: Option<DateTime<Utc>>,
    pub last_activity: Option<DateTime<Utc>>,
    pub warn_count: u64,
    pub timeout_count: u64,
    pub total_runtime_ms: u64,
}

pub struct TimeoutController {
    config: TimeoutConfig,
    state: Arc<RwLock<TimeoutState>>,
    start_time: Arc<RwLock<Option<Instant>>>,
    last_activity: Arc<RwLock<Option<Instant>>>,
    warn_issued: Arc<AtomicBool>,
    stats: Arc<RwLock<TimeoutStats>>,
}

impl TimeoutController {
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(TimeoutState::Idle)),
            start_time: Arc::new(RwLock::new(None)),
            last_activity: Arc::new(RwLock::new(None)),
            warn_issued: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(TimeoutStats::default())),
        }
    }

    pub fn with_default() -> Self {
        Self::new(TimeoutConfig::default())
    }

    #[inline]
    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }

    pub async fn state(&self) -> TimeoutState {
        *self.state.read().await
    }

    pub async fn start(&self) {
        let mut state = self.state.write().await;
        if *state == TimeoutState::Idle {
            *state = TimeoutState::Running;
            *self.start_time.write().await = Some(Instant::now());
            *self.last_activity.write().await = Some(Instant::now());
            
            let mut stats = self.stats.write().await;
            stats.start_time = Some(Utc::now());
            stats.last_activity = Some(Utc::now());
        }
    }

    pub async fn stop(&self) {
        let mut state = self.state.write().await;
        *state = TimeoutState::Idle;
        self.warn_issued.store(false, Ordering::SeqCst);
        
        if let Some(start) = *self.start_time.read().await {
            let elapsed = start.elapsed().as_millis() as u64;
            let mut stats = self.stats.write().await;
            stats.total_runtime_ms += elapsed;
        }
        
        *self.start_time.write().await = None;
    }

    pub async fn record_activity(&self) {
        *self.last_activity.write().await = Some(Instant::now());
        
        let mut stats = self.stats.write().await;
        stats.last_activity = Some(Utc::now());
        
        if self.warn_issued.load(Ordering::SeqCst) {
            *self.state.write().await = TimeoutState::Running;
            self.warn_issued.store(false, Ordering::SeqCst);
        }
    }

    pub async fn check_timeout(&self) -> bool {
        let state = self.state.read().await;
        if *state != TimeoutState::Running {
            return false;
        }
        drop(state);

        if let Some(start) = *self.start_time.read().await {
            let elapsed = start.elapsed().as_millis() as u64;
            
            if elapsed >= self.config.total_timeout_ms {
                let mut state = self.state.write().await;
                *state = TimeoutState::TimedOut;
                
                let mut stats = self.stats.write().await;
                stats.timeout_count += 1;
                
                return true;
            }
            
            if elapsed >= self.config.warn_threshold_ms 
                && !self.warn_issued.load(Ordering::SeqCst) 
                && *self.state.read().await == TimeoutState::Running
            {
                *self.state.write().await = TimeoutState::Warned;
                self.warn_issued.store(true, Ordering::SeqCst);
                
                let mut stats = self.stats.write().await;
                stats.warn_count += 1;
            }
        }

        if let Some(last) = *self.last_activity.read().await {
            let idle = last.elapsed().as_millis() as u64;
            
            if idle >= self.config.idle_timeout_ms {
                let mut state = self.state.write().await;
                *state = TimeoutState::TimedOut;
                
                let mut stats = self.stats.write().await;
                stats.timeout_count += 1;
                
                return true;
            }
        }

        false
    }

    pub async fn is_timeout(&self) -> bool {
        *self.state.read().await == TimeoutState::TimedOut
    }

    pub async fn is_warned(&self) -> bool {
        *self.state.read().await == TimeoutState::Warned
    }

    pub async fn is_running(&self) -> bool {
        let state = *self.state.read().await;
        state == TimeoutState::Running || state == TimeoutState::Warned
    }

    pub async fn elapsed_ms(&self) -> u64 {
        if let Some(start) = *self.start_time.read().await {
            start.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    pub async fn idle_ms(&self) -> u64 {
        if let Some(last) = *self.last_activity.read().await {
            last.elapsed().as_millis() as u64
        } else {
            0
        }
    }

    pub async fn remaining_ms(&self) -> u64 {
        if let Some(start) = *self.start_time.read().await {
            let elapsed = start.elapsed().as_millis() as u64;
            self.config.total_timeout_ms.saturating_sub(elapsed)
        } else {
            self.config.total_timeout_ms
        }
    }

    pub async fn stats(&self) -> TimeoutStats {
        self.stats.read().await.clone()
    }

    pub async fn reset(&self) {
        *self.state.write().await = TimeoutState::Idle;
        *self.start_time.write().await = None;
        *self.last_activity.write().await = None;
        self.warn_issued.store(false, Ordering::SeqCst);
    }

    pub async fn cancel(&self) {
        let mut state = self.state.write().await;
        if *state != TimeoutState::TimedOut {
            *state = TimeoutState::Cancelled;
        }
    }

    pub fn operation_timeout(&self) -> Duration {
        self.config.operation_timeout()
    }

    pub fn total_timeout(&self) -> Duration {
        self.config.total_timeout()
    }

    pub async fn run_with_timeout<F, T, E>(&self, future: F) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        self.start().await;
        
        let timeout = self.operation_timeout();
        match tokio::time::timeout(timeout, future).await {
            Ok(result) => {
                self.record_activity().await;
                result.map_err(|e| TimeoutError::OperationError(format!("{:?}", e)))
            }
            Err(_) => {
                self.cancel().await;
                Err(TimeoutError::OperationTimeout)
            }
        }
    }
}

impl Default for TimeoutController {
    fn default() -> Self {
        Self::with_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeoutError {
    OperationTimeout,
    TotalTimeout,
    IdleTimeout,
    Cancelled,
    OperationError(String),
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::OperationTimeout => write!(f, "operation timeout"),
            TimeoutError::TotalTimeout => write!(f, "total timeout"),
            TimeoutError::IdleTimeout => write!(f, "idle timeout"),
            TimeoutError::Cancelled => write!(f, "cancelled"),
            TimeoutError::OperationError(e) => write!(f, "operation error: {}", e),
        }
    }
}

impl std::error::Error for TimeoutError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_controller() -> TimeoutController {
        TimeoutController::new(TimeoutConfig {
            operation_timeout_ms: 1000,
            total_timeout_ms: 5000,
            idle_timeout_ms: 2000,
            warn_threshold_ms: 800,
        })
    }

    #[tokio::test]
    async fn test_start_stop() {
        let controller = create_test_controller();
        
        assert_eq!(controller.state().await, TimeoutState::Idle);
        
        controller.start().await;
        assert_eq!(controller.state().await, TimeoutState::Running);
        
        controller.stop().await;
        assert_eq!(controller.state().await, TimeoutState::Idle);
    }

    #[tokio::test]
    async fn test_record_activity() {
        let controller = create_test_controller();
        
        controller.start().await;
        controller.record_activity().await;
        
        assert!(controller.idle_ms().await < 100);
    }

    #[tokio::test]
    async fn test_check_timeout_total() {
        let controller = TimeoutController::new(TimeoutConfig {
            operation_timeout_ms: 100,
            total_timeout_ms: 100,
            idle_timeout_ms: 10000,
            warn_threshold_ms: 50,
        });
        
        controller.start().await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        let timed_out = controller.check_timeout().await;
        assert!(timed_out);
        assert_eq!(controller.state().await, TimeoutState::TimedOut);
    }

    #[tokio::test]
    async fn test_warn_threshold() {
        let controller = TimeoutController::new(TimeoutConfig {
            operation_timeout_ms: 1000,
            total_timeout_ms: 5000,
            idle_timeout_ms: 10000,
            warn_threshold_ms: 50,
        });
        
        controller.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let _ = controller.check_timeout().await;
        assert_eq!(controller.state().await, TimeoutState::Warned);
    }

    #[tokio::test]
    async fn test_remaining() {
        let controller = TimeoutController::new(TimeoutConfig {
            operation_timeout_ms: 1000,
            total_timeout_ms: 5000,
            idle_timeout_ms: 10000,
            warn_threshold_ms: 800,
        });
        
        controller.start().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let remaining = controller.remaining_ms().await;
        assert!(remaining < 5000);
        assert!(remaining > 4000);
    }

    #[tokio::test]
    async fn test_reset() {
        let controller = create_test_controller();
        
        controller.start().await;
        controller.reset().await;
        
        assert_eq!(controller.state().await, TimeoutState::Idle);
        assert_eq!(controller.elapsed_ms().await, 0);
    }

    #[tokio::test]
    async fn test_cancel() {
        let controller = create_test_controller();
        
        controller.start().await;
        controller.cancel().await;
        
        assert_eq!(controller.state().await, TimeoutState::Cancelled);
    }

    #[tokio::test]
    async fn test_stats() {
        let controller = create_test_controller();
        
        controller.start().await;
        controller.record_activity().await;
        
        let stats = controller.stats().await;
        assert!(stats.start_time.is_some());
        assert!(stats.last_activity.is_some());
    }

    #[tokio::test]
    async fn test_run_with_timeout_success() {
        let controller = create_test_controller();
        
        let result = controller
            .run_with_timeout(async { 
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok::<_, ()>(42)
            })
            .await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_run_with_timeout_err() {
        let controller = TimeoutController::new(TimeoutConfig {
            operation_timeout_ms: 50,
            total_timeout_ms: 5000,
            idle_timeout_ms: 10000,
            warn_threshold_ms: 40,
        });
        
        let result = controller
            .run_with_timeout(async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<_, ()>(42)
            })
            .await;
        
        assert!(result.is_err());
    }
}
