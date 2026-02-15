//! OpenClaw Device - 设备节点模块
//!
//! 提供设备能力节点：相机、屏幕录制、定位、通知、系统命令等

pub mod nodes;
pub mod camera;
pub mod screen;
pub mod location;
pub mod notification;
pub mod system;

pub use nodes::*;
pub use camera::*;
pub use screen::*;
pub use location::*;
pub use notification::*;
pub use system::*;
