//! OpenClaw Memory - 分层记忆系统
//!
//! 实现三层记忆架构：
//! - 工作记忆 (Working Memory): 最近消息，高优先级
//! - 短期记忆 (Short-term Memory): 压缩摘要，中优先级
//! - 长期记忆 (Long-term Memory): 向量存储，低优先级
//!
//! 同时支持 OpenClaw 风格的 Markdown 记忆系统：
//! - AGENTS.md: 智能体操作指南
//! - SOUL.md: 个性设定
//! - USER.md: 用户信息
//! - memory/YYYY-MM-DD.md: 每日记忆
//! - MEMORY.md: 长期记忆汇总

pub mod ai_adapter;
pub mod bm25;
pub mod chunk;
pub mod compressor;
pub mod config;
pub mod conflict_resolver;
pub mod embedding;
pub mod fact_extractor;
pub mod file_tracker;
pub mod hybrid_search;
pub mod knowledge_graph;
pub mod manager;
pub mod maintenance_scheduler;
pub mod pruning;
pub mod recall;
pub mod recall_strategy;
pub mod scorer;
pub mod store;
pub mod types;
pub mod unified_search;
pub mod checkpoint;
pub mod checkpoint_store;
pub mod file_watcher;

pub use checkpoint::*;
pub use checkpoint_store::*;
pub use file_watcher::*;
pub mod working;
pub mod workspace;

pub use ai_adapter::*;
pub use bm25::*;
pub use chunk::*;
pub use compressor::*;
pub use config::*;
pub use conflict_resolver::*;
pub use embedding::*;
pub use fact_extractor::*;
pub use file_tracker::*;
pub use hybrid_search::*;
pub use knowledge_graph::*;
pub use manager::*;
pub use maintenance_scheduler::*;
pub use pruning::*;
pub use recall::*;
pub use recall_strategy::*;
pub use scorer::*;
pub use store::*;
pub use types::*;
pub use working::*;
pub use workspace::*;
