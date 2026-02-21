//! OpenClaw Agent - 多智能体系统
//!
//! 实现 Agent Teams 架构：
//! - 不同类型的 Agent 处理不同任务
//! - Agent Orchestrator 协调多个 Agent
//! - 任务路由和分配机制

pub mod agent;
pub mod aieos;
pub mod channels;
pub mod decision;
pub mod dependencies;
pub mod device_tools;
pub mod graph;
pub mod integration;
pub mod memory_pipeline;
pub mod orchestrator;
pub mod presence;
pub mod router;
pub mod sessions;
pub mod sub_agent;
pub mod task;
pub mod team;
pub mod types;
pub mod ui_tools;
pub mod voice;

pub use agent::*;
pub use aieos::AIEOS;
pub use dependencies::*;
pub use device_tools::*;
pub use graph::*;
pub use integration::*;
pub use memory_pipeline::*;
pub use orchestrator::*;
pub use presence::*;
pub use router::*;
pub use sessions::*;
pub use sub_agent::*;
pub use task::*;
pub use team::*;
pub use types::*;
pub use ui_tools::*;
pub use voice::*;

pub use openclaw_core::{OpenClawError, Result};
