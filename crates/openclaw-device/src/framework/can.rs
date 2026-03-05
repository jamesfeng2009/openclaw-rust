//! CAN 接口
//!
//! CAN (Controller Area Network) 总线接口

use serde::{Deserialize, Serialize};

pub type CanResult<T> = Result<T, CanError>;

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

#[cfg(all(feature = "can_socketcan", not(target_os = "macos")))]
pub mod socketcan_impl {
    use super::*;
    use std::sync::Mutex;

    #[cfg(not(target_os = "macos"))]
    use socketcan::{CANFrame, SocketCan};

    #[cfg(not(target_os = "macos"))]
    pub struct SocketCanBus {
        socket: Mutex<Option<SocketCan>>,
        interface: String,
        state: Mutex<CanState>,
        bit_rate: Mutex<u32>,
    }

    #[cfg(not(target_os = "macos"))]
    impl SocketCanBus {
        pub fn new(interface: &str) -> Result<Self, CanError> {
            let socket = SocketCan::open(interface)
                .map_err(|e| CanError::IoError(e.to_string()))?;

            Ok(Self {
                socket: Mutex::new(Some(socket)),
                interface: interface.to_string(),
                state: Mutex::new(CanState::ErrorActive),
                bit_rate: Mutex::new(500_000),
            })
        }

        pub fn set_bit_rate(&self, bit_rate: u32) {
            let mut rate = self.bit_rate.lock().unwrap();
            *rate = bit_rate;
        }
    }

    #[cfg(not(target_os = "macos"))]
    impl CanBus for SocketCanBus {
        fn interface_name(&self) -> &str {
            &self.interface
        }

        fn state(&self) -> CanState {
            *self.state.lock().unwrap()
        }

        fn bit_rate(&self) -> u32 {
            *self.bit_rate.lock().unwrap()
        }

        fn send(&self, frame: &CanFrame) -> CanResult<()> {
            let mut socket_guard = self.socket.lock().unwrap();
            if let Some(ref socket) = *socket_guard {
                let can_frame = CANFrame::new(
                    frame.id.id(),
                    &frame.data,
                    frame.is_remote_transmission_request,
                    frame.is_error_frame,
                )
                .map_err(|e| CanError::IoError(e.to_string()))?;

                socket
                    .write_frame(&can_frame)
                    .map_err(|e| CanError::IoError(e.to_string()))?;
                Ok(())
            } else {
                Err(CanError::InterfaceNotFound(self.interface.clone()))
            }
        }

        fn receive(&self) -> CanResult<CanFrame> {
            let socket_guard = self.socket.lock().unwrap();
            if let Some(ref mut socket) = *socket_guard {
                let frame = socket
                    .read_frame()
                    .map_err(|e| CanError::IoError(e.to_string()))?;

                Ok(CanFrame {
                    id: CanId::standard(frame.id() as u16),
                    data: frame.data().to_vec(),
                    is_remote_transmission_request: frame.is_rtr(),
                    is_error_frame: frame.is_error_frame(),
                    is_overload_frame: false,
                    timestamp_ns: 0,
                })
            } else {
                Err(CanError::InterfaceNotFound(self.interface.clone()))
            }
        }

        fn set_filters(&self, _filters: &[CanFilter]) -> CanResult<()> {
            Ok(())
        }

        fn get_stats(&self) -> CanResult<CanBusInfo> {
            Ok(CanBusInfo {
                name: self.interface.clone(),
                state: self.state(),
                bit_rate: self.bit_rate(),
                tx_count: 0,
                rx_count: 0,
                error_count: 0,
            })
        }

        fn is_available(&self) -> bool {
            self.state() != CanState::BusOff
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "requires CAN interface"]
        fn test_socketcan_creation() {
            let result = SocketCanBus::new("vcan0");
            if result.is_ok() {
                let bus = result.unwrap();
                assert_eq!(bus.interface_name(), "vcan0");
            }
        }
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

pub trait CanBus: Send + Sync {
    fn interface_name(&self) -> &str;

    fn state(&self) -> CanState;

    fn bit_rate(&self) -> u32;

    fn send(&self, frame: &CanFrame) -> CanResult<()>;

    fn receive(&self) -> CanResult<CanFrame>;

    fn set_filters(&self, filters: &[CanFilter]) -> CanResult<()>;

    fn get_stats(&self) -> CanResult<CanBusInfo>;

    fn is_available(&self) -> bool;
}

#[cfg(feature = "can_mock")]
pub mod mock_impl {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    

    #[derive(Clone)]
    pub struct MockCanBus {
        interface: String,
        state: Arc<Mutex<CanState>>,
        bit_rate: u32,
        tx_queue: Arc<Mutex<VecDeque<CanFrame>>>,
        rx_queue: Arc<Mutex<VecDeque<CanFrame>>>,
        stats: Arc<Mutex<CanBusInfo>>,
    }

    impl MockCanBus {
        pub fn new(interface: &str, bit_rate: u32) -> Self {
            Self {
                interface: interface.to_string(),
                state: Arc::new(Mutex::new(CanState::ErrorActive)),
                bit_rate,
                tx_queue: Arc::new(Mutex::new(VecDeque::new())),
                rx_queue: Arc::new(Mutex::new(VecDeque::new())),
                stats: Arc::new(Mutex::new(CanBusInfo {
                    name: interface.to_string(),
                    state: CanState::ErrorActive,
                    bit_rate,
                    tx_count: 0,
                    rx_count: 0,
                    error_count: 0,
                })),
            }
        }

        pub fn send_frame(&self, frame: CanFrame) -> Result<(), CanError> {
            let mut queue = self.tx_queue.lock().unwrap();
            queue.push_back(frame);
            let mut stats = self.stats.lock().unwrap();
            stats.tx_count += 1;
            Ok(())
        }

        pub fn receive_frame(&self) -> Result<CanFrame, CanError> {
            let mut queue = self.rx_queue.lock().unwrap();
            queue.pop_front().ok_or(CanError::RxQueueEmpty)
        }

        pub fn inject_frame(&self, frame: CanFrame) {
            let mut queue = self.rx_queue.lock().unwrap();
            queue.push_back(frame);
            let mut stats = self.stats.lock().unwrap();
            stats.rx_count += 1;
        }

        pub fn set_state(&self, state: CanState) {
            let mut s = self.state.lock().unwrap();
            *s = state;
            let mut stats = self.stats.lock().unwrap();
            stats.state = state;
        }
    }

    impl CanBus for MockCanBus {
        fn interface_name(&self) -> &str {
            &self.interface
        }

        fn state(&self) -> CanState {
            *self.state.lock().unwrap()
        }

        fn bit_rate(&self) -> u32 {
            self.bit_rate
        }

        fn send(&self, frame: &CanFrame) -> CanResult<()> {
            self.send_frame(frame.clone())
                .map_err(|e| CanError::IoError(e.to_string()))
        }

        fn receive(&self) -> CanResult<CanFrame> {
            self.receive_frame()
                .map_err(|e| CanError::IoError(e.to_string()))
        }

        fn set_filters(&self, _filters: &[CanFilter]) -> CanResult<()> {
            Ok(())
        }

        fn get_stats(&self) -> CanResult<CanBusInfo> {
            Ok(self.stats.lock().unwrap().clone())
        }

        fn is_available(&self) -> bool {
            *self.state.lock().unwrap() == CanState::ErrorActive
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_can_bus_creation() {
            let bus = MockCanBus::new("vcan0", 500_000);
            assert_eq!(bus.interface_name(), "vcan0");
            assert_eq!(bus.bit_rate(), 500_000);
            assert!(bus.is_available());
        }

        #[test]
        fn test_mock_can_send_receive() {
            let bus = MockCanBus::new("vcan0", 500_000);
            let frame = CanFrame::new(CanId::standard(0x123), vec![0x01, 0x02, 0x03]);
            
            bus.inject_frame(frame.clone());
            let received = bus.receive_frame().unwrap();
            
            assert_eq!(received.id.id(), 0x123);
            assert_eq!(received.data, vec![0x01, 0x02, 0x03]);
        }

        #[test]
        fn test_mock_can_bus_stats() {
            let bus = MockCanBus::new("vcan0", 500_000);
            let frame = CanFrame::new(CanId::standard(0x100), vec![0xAA]);
            
            bus.send_frame(frame.clone()).unwrap();
            let stats = bus.get_stats().unwrap();
            
            assert_eq!(stats.tx_count, 1);
        }
    }
}
