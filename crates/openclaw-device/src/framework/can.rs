//! CAN 接口
//!
//! CAN (Controller Area Network) 总线接口

use crate::framework::FrameworkModule;
use serde::{Deserialize, Serialize};

pub type CanResult<T> = crate::framework::FrameworkResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum CanError {
    #[error("Interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("Bus off")]
    BusOff,
    #[error("Error passive")]
    ErrorPassive,
    #[error("Error warning")]
    ErrorWarning,
    #[error("TX queue full")]
    TxQueueFull,
    #[error("RX queue empty")]
    RxQueueEmpty,
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Filter error: {0}")]
    FilterError(String),
    #[error("Frame too long: {0}")]
    FrameTooLong(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanId(u32);

impl CanId {
    pub fn standard(id: u16) -> Self {
        Self(id as u32)
    }

    pub fn extended(id: u32) -> Self {
        Self(id | 0x80000000)
    }

    pub fn is_extended(&self) -> bool {
        (self.0 & 0x80000000) != 0
    }

    pub fn id(&self) -> u32 {
        self.0 & 0x7FFFFFFF
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFrame {
    pub id: CanId,
    pub data: Vec<u8>,
    pub is_remote_transmission_request: bool,
    pub is_error_frame: bool,
    pub is_overload_frame: bool,
    pub timestamp_ns: u64,
}

impl CanFrame {
    pub fn new(id: CanId, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            is_remote_transmission_request: false,
            is_error_frame: false,
            is_overload_frame: false,
            timestamp_ns: 0,
        }
    }

    pub fn data_len(&self) -> usize {
        self.data.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFilter {
    pub id: CanId,
    pub mask: CanId,
}

impl CanFilter {
    pub fn standard(id: u16, mask: u16) -> Self {
        Self {
            id: CanId::standard(id),
            mask: CanId::standard(mask),
        }
    }

    pub fn extended(id: u32, mask: u32) -> Self {
        Self {
            id: CanId::extended(id),
            mask: CanId::extended(mask),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanState {
    ErrorActive,
    ErrorWarning,
    ErrorPassive,
    BusOff,
    Sleeping,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanBusInfo {
    pub name: String,
    pub state: CanState,
    pub bit_rate: u32,
    pub tx_count: u64,
    pub rx_count: u64,
    pub error_count: u64,
}

pub trait CanBus: FrameworkModule {
    fn interface_name(&self) -> &str;

    fn state(&self) -> CanState;

    fn bit_rate(&self) -> u32;

    fn send(&self, frame: &CanFrame) -> CanResult<()>;

    fn receive(&self) -> CanResult<CanFrame>;

    fn set_filters(&self, filters: &[CanFilter]) -> CanResult<()>;

    fn get_stats(&self) -> CanResult<CanBusInfo>;

    fn is_available(&self) -> bool;
}
