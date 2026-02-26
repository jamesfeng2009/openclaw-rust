//! Safety Module - 安全控制模块
//!
//! 提供运行时安全防护：
//! - Turn 计数限制
//! - 超时控制
//! - 任务中止

pub mod abort_control;
pub mod agent_wrapper;
pub mod timeout;
pub mod turn_limiter;

pub use abort_control::AbortableTask;
pub use agent_wrapper::{AgentSafetyConfig, AgentSafetyWrapper, SafetyAction, SafetyStats};
pub use timeout::{TimeoutConfig, TimeoutController, TimeoutState};
pub use turn_limiter::{LimitReason, TurnLimitConfig, TurnLimiter};
