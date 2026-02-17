//! 硬件抽象层 (HAL) 模块接口
//!
//! 提供统一的硬件接口抽象，支持多种平台

pub mod gpio;
pub mod i2c;
pub mod serial;
pub mod spi;

pub use gpio::{GpioError, GpioMode, GpioPin, GpioResult, GpioState};
pub use i2c::{I2cBus, I2cDevice, I2cError, I2cResult};
pub use serial::{SerialConfig, SerialError, SerialPort, SerialResult};
pub use spi::{SpiBus, SpiDevice, SpiError, SpiMode, SpiResult};

use crate::platform::Platform;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalConfig {
    pub enabled: bool,
    pub platform: Option<Platform>,
    pub config: std::collections::HashMap<String, String>,
}

#[async_trait]
pub trait HalModule: Send + Sync {
    fn name(&self) -> &str;

    fn supported_platforms(&self) -> &[Platform];

    fn is_available(&self) -> bool;

    async fn init(&self, config: &HalConfig) -> HalResult<()>;

    async fn health_check(&self) -> HalResult<bool>;
}

pub type HalResult<T> = Result<T, HalError>;

#[derive(Debug, thiserror::Error)]
pub enum HalError {
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Not initialized: {0}")]
    NotInitialized(String),

    #[error("Already initialized")]
    AlreadyInitialized,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
