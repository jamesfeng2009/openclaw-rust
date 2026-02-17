//! SPI 接口
//!
//! SPI (Serial Peripheral Interface) 总线接口

use crate::hal::{HalConfig, HalModule, HalResult};
use crate::platform::Platform;
use serde::{Deserialize, Serialize};

pub type SpiResult<T> = crate::hal::HalResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum SpiError {
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Bus not found: {0}")]
    BusNotFound(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Chip select error: {0}")]
    ChipSelectError(u8),
    #[error("Mode not supported: {0}")]
    ModeNotSupported(u8),
    #[error("Timeout")]
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpiMode {
    Mode0,
    Mode1,
    Mode2,
    Mode3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpiBitOrder {
    LsbFirst,
    MsbFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiConfig {
    pub mode: SpiMode,
    pub bit_order: SpiBitOrder,
    pub max_speed_hz: u32,
    pub bits_per_word: u8,
    pub chip_select: u8,
}

impl Default for SpiConfig {
    fn default() -> Self {
        Self {
            mode: SpiMode::Mode0,
            bit_order: SpiBitOrder::MsbFirst,
            max_speed_hz: 1_000_000,
            bits_per_word: 8,
            chip_select: 0,
        }
    }
}

pub trait SpiDevice: Send + Sync {
    fn device_id(&self) -> &str;

    fn config(&self) -> &SpiConfig;

    fn transfer(&self, data: &[u8]) -> SpiResult<Vec<u8>>;

    fn write(&self, data: &[u8]) -> SpiResult<usize>;

    fn read(&self, buffer: &mut [u8]) -> SpiResult<usize>;
}

pub trait SpiBus: HalModule {
    fn bus_id(&self) -> &str;

    fn available_devices(&self) -> Vec<u8>;

    fn get_device(&self, chip_select: u8, config: SpiConfig) -> SpiResult<Box<dyn SpiDevice>>;
}
