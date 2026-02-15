//! OpenClaw Browser - 浏览器控制模块
//!
//! 提供 Puppeteer 风格的 Chrome/Chromium 实例控制
//! 支持截图、页面操作、文件上传等功能

pub mod browser;
pub mod page;
pub mod screenshot;
pub mod types;

pub use browser::*;
pub use page::*;
pub use screenshot::*;
pub use types::*;
