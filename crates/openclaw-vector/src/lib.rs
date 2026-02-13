//! OpenClaw Vector - 向量存储抽象层
//!
//! 提供统一的向量存储接口，支持多种后端：
//! - LanceDB (嵌入式，零依赖)
//! - Qdrant (高性能独立服务)
//! - pgvector (PostgreSQL 扩展)
//! - SQLite-vec (轻量级嵌入)

pub mod store;
pub mod types;

pub use store::*;
pub use types::*;
