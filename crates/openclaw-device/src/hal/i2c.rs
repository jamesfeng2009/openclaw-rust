//! I2C 接口
//!
//! I2C (Inter-Integrated Circuit) 总线接口

use crate::hal::{HalConfig, HalModule, HalResult};
use crate::platform::Platform;
use serde::{Deserialize, Serialize};

pub type I2cResult<T> = crate::hal::HalResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum I2cError {
    #[error("Device not found at address: {0}")]
    DeviceNotFound(u8),
    #[error("Bus not found: {0}")]
    BusNotFound(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Bus busy")]
    BusBusy,
    #[error("NACK received")]
    Nack,
    #[error("Timeout")]
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum I2cSpeed {
    Standard,
    Fast,
    FastModePlus,
    HighSpeed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I2cDeviceAddress {
    pub address: u8,
    pub name: Option<String>,
}

pub trait I2cDevice: Send + Sync {
    fn address(&self) -> u8;

    fn write(&self, data: &[u8]) -> I2cResult<usize>;

    fn read(&self, buffer: &mut [u8]) -> I2cResult<usize>;

    fn write_read(&self, write_data: &[u8], read_buffer: &mut [u8]) -> I2cResult<usize>;
}

pub trait I2cBus: HalModule {
    fn bus_id(&self) -> &str;

    fn speed(&self) -> I2cSpeed;

    fn scan(&self) -> Vec<I2cDeviceAddress>;

    fn get_device(&self, address: u8) -> I2cResult<Box<dyn I2cDevice>>;

    fn is_busy(&self) -> bool;
}
