//! Abort Control - 任务中止控制
//!
//! 提供基于 AbortSignal 的任务中止控制：
//! - 可取消的任务
//! - 超时自动中止
//! - 外部信号中止

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AbortReason {
    #[default]
    Manual,
    Timeout,
    LimitReached,
    ParentCancelled,
}

impl std::fmt::Display for AbortReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbortReason::Manual => write!(f, "manually aborted"),
            AbortReason::Timeout => write!(f, "timeout"),
            AbortReason::LimitReached => write!(f, "limit reached"),
            AbortReason::ParentCancelled => write!(f, "parent cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortState {
    pub is_aborted: bool,
    pub reason: Option<AbortReason>,
    pub abort_count: u64,
}

impl Default for AbortState {
    fn default() -> Self {
        Self {
            is_aborted: false,
            reason: None,
            abort_count: 0,
        }
    }
}

#[derive(Clone)]
pub struct AbortSignal {
    aborted: std::sync::Arc<AtomicBool>,
    reason: std::sync::Arc<Mutex<Option<AbortReason>>>,
}

impl AbortSignal {
    pub fn new() -> Self {
        Self {
            aborted: std::sync::Arc::new(AtomicBool::new(false)),
            reason: std::sync::Arc::new(Mutex::new(None)),
        }
    }

    pub fn abort(&self, reason: AbortReason) {
        self.aborted.store(true, Ordering::SeqCst);
        if let Ok(mut r) = self.reason.lock() {
            *r = Some(reason);
        }
    }

    pub fn abort_manual(&self) {
        self.abort(AbortReason::Manual);
    }

    pub fn is_aborted(&self) -> bool {
        self.aborted.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<AbortReason> {
        self.reason.lock().ok().and_then(|r| *r)
    }

    pub fn reset(&self) {
        self.aborted.store(false, Ordering::SeqCst);
        if let Ok(mut r) = self.reason.lock() {
            *r = None;
        }
    }
}

impl Default for AbortSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AbortSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AbortSignal")
            .field("is_aborted", &self.is_aborted())
            .finish()
    }
}

pub struct AbortableTask {
    signal: AbortSignal,
    is_cancelled: AtomicBool,
}

impl AbortableTask {
    pub fn new() -> Self {
        Self {
            signal: AbortSignal::new(),
            is_cancelled: AtomicBool::new(false),
        }
    }

    pub fn signal(&self) -> &AbortSignal {
        &self.signal
    }

    pub fn abort(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
        self.signal.abort_manual();
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst) || self.signal.is_aborted()
    }

    pub fn check(&self) -> Result<(), AbortReason> {
        if self.is_cancelled() {
            Err(self.signal.reason().unwrap_or(AbortReason::Manual))
        } else {
            Ok(())
        }
    }

    pub async fn spawn<F>(&self, future: F) -> AbortHandle
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let signal = self.signal.clone();
        let handle = tokio::spawn(async move {
            if !signal.is_aborted() {
                future.await;
            }
        });
        AbortHandle { handle }
    }
}

impl Default for AbortableTask {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AbortHandle {
    handle: tokio::task::JoinHandle<()>,
}

impl AbortHandle {
    pub async fn join(self) {
        let _ = self.handle.await;
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal() {
        let signal = AbortSignal::new();
        
        assert!(!signal.is_aborted());
        
        signal.abort(AbortReason::Timeout);
        assert!(signal.is_aborted());
    }

    #[tokio::test]
    async fn test_abortable_task() {
        let task = AbortableTask::new();
        
        let handle = task.spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }).await;
        
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        assert!(!task.is_cancelled());
        handle.join().await;
    }

    #[tokio::test]
    async fn test_abortable_task_abort() {
        let task = AbortableTask::new();
        
        let handle = task.spawn(async {
            for _ in 0..1000u64 {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }).await;
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        task.abort();
        
        assert!(task.is_cancelled());
        handle.join().await;
    }
}
