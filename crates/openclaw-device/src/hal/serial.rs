//! Serial 接口
//!
//! 串口 (UART/RS232/RS485) 通信接口

use crate::hal::{HalConfig, HalModule, HalResult};
use crate::platform::Platform;
use serde::{Deserialize, Serialize};

pub type SerialResult<T> = crate::hal::HalResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum SerialError {
    #[error("Port not found: {0}")]
    PortNotFound(String),
    #[error("Port already in use")]
    PortInUse,
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Port not open")]
    PortNotOpen,
    #[error("Timeout")]
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaudRate {
    B300,
    B1200,
    B2400,
    B4800,
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
    B230400,
    B460800,
    B921600,
    Custom(u32),
}

impl Default for BaudRate {
    fn default() -> Self {
        Self::B115200
    }
}

impl BaudRate {
    pub fn to_speed(&self) -> u32 {
        match self {
            Self::B300 => 300,
            Self::B1200 => 1200,
            Self::B2400 => 2400,
            Self::B4800 => 4800,
            Self::B9600 => 9600,
            Self::B19200 => 19200,
            Self::B38400 => 38400,
            Self::B57600 => 57600,
            Self::B115200 => 115200,
            Self::B230400 => 230400,
            Self::B460800 => 460800,
            Self::B921600 => 921600,
            Self::Custom(speed) => *speed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
}

impl Default for DataBits {
    fn default() -> Self {
        Self::Eight
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl Default for Parity {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopBits {
    One,
    Two,
}

impl Default for StopBits {
    fn default() -> Self {
        Self::One
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: BaudRate,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: bool,
    pub timeout_ms: u64,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".to_string(),
            baud_rate: BaudRate::default(),
            data_bits: DataBits::default(),
            parity: Parity::default(),
            stop_bits: StopBits::default(),
            flow_control: false,
            timeout_ms: 1000,
        }
    }
}

pub trait SerialPort: Send + Sync {
    fn port_name(&self) -> &str;

    fn config(&self) -> &SerialConfig;

    fn write(&self, data: &[u8]) -> SerialResult<usize>;

    fn read(&self, buffer: &mut [u8]) -> SerialResult<usize>;

    fn flush(&self) -> SerialResult<()>;

    fn is_connected(&self) -> bool;

    fn set_config(&self, config: SerialConfig) -> SerialResult<()>;
}
