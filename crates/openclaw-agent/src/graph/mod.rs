//! Agent Graph Module - 多 Agent 协作执行引擎
//!
//! 提供真正的多 Agent 并行协作能力，支持多种协作模式：
//! - 顺序执行 (Sequential)
//! - 并行执行 (Parallel)
//! - 广播 (Broadcast)
//! - 树状结构 (Tree)
//! - Map-Reduce
//! - 专家混合 (MixtureOfExperts)
//!
//! # 核心特性
//!
//! - **真并行**: 使用 tokio::spawn() 实现多节点同时运行
//! - **动态调度**: 自动发现就绪节点并并行执行
//! - **依赖管理**: 自动追踪节点依赖关系，保证执行顺序正确
//! - **灵活模式**: 支持自定义 Graph 定义和预定义模式

pub mod definition;
pub mod context;
pub mod executor;
pub mod patterns;

pub use definition::*;
pub use context::*;
pub use executor::*;
pub use patterns::*;

use async_trait::async_trait;

#[async_trait]
pub trait GraphExecutorTrait: Send + Sync {
    async fn execute(&self, request_id: String, input: serde_json::Value) -> Result<GraphResponse, String>;
}
