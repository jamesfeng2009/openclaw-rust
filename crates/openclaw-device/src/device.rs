//! 平台检测与设备抽象层
//!
//! 支持弹性计算和边缘计算设备的统一抽象

// 重新导出主要类型
pub use crate::adapter::{AdapterConfig, AdapterResult, Adapters, DeviceAdapter};
pub use crate::capabilities::DeviceCapabilities;
pub use crate::platform::{ComputeCategory, Platform, PlatformInfo};
pub use crate::registry::{DeviceHandle, DeviceQuery, DeviceRegistry, DeviceStatus};
