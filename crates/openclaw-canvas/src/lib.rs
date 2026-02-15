//! OpenClaw Canvas - 实时协作画布模块
//!
//! 提供 A2UI 可视化工作空间和实时协作画布功能

pub mod canvas;
pub mod collaboration;
pub mod draw;
pub mod types;

pub use canvas::*;
pub use collaboration::*;
pub use draw::*;
pub use types::*;
