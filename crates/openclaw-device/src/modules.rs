//! 模块管理器
//!
//! 统一管理 HAL 硬件抽象层和框架集成层模块
//! 根据设备能力自动发现和初始化可用模块

pub mod manager;

pub use manager::ModuleManager;
