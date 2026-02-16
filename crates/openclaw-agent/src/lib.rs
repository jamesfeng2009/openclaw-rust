//! OpenClaw Agent - 多智能体系统
//!
//! 实现 Agent Teams 架构：
//! - 不同类型的 Agent 处理不同任务
//! - Agent Orchestrator 协调多个 Agent
//! - 任务路由和分配机制

pub mod agent;
pub mod orchestrator;
pub mod task;
pub mod team;
pub mod types;
pub mod integration;
pub mod sub_agent;
pub mod router;
pub mod presence;
pub mod sessions;
pub mod decision;
pub mod voice;
pub mod channels;
pub mod aieos;

pub use agent::*;
pub use orchestrator::*;
pub use task::*;
pub use team::*;
pub use types::*;
pub use router::*;
pub use presence::*;
pub use sessions::*;
pub use integration::*;
pub use aieos::*;
pub use sub_agent::*;

pub use openclaw_core::{Result, OpenClawError};
