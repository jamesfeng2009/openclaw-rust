//! RISC-V UART Hardware Abstraction
//!
//! UART trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait UartRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn baud_rate(&self) -> u32;
    fn write(&mut self, data: &[u8]) -> Result<usize, UartError>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, UartError>;
    fn flush(&mut self) -> Result<(), UartError>;
}

#[derive(Debug, Clone)]
pub enum UartError {
    WriteError,
    ReadError,
    Timeout,
    NotSupported,
}

impl std::fmt::Display for UartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WriteError => write!(f, "UART write error"),
            Self::ReadError => write!(f, "UART read error"),
            Self::Timeout => write!(f, "UART operation timeout"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for UartError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockUartRiscv {
        plat: Platform,
        baud: u32,
        buffer: std::collections::VecDeque<u8>,
    }

    impl MockUartRiscv {
        pub fn new(platform: Platform, baud_rate: u32) -> Self {
            Self {
                plat: platform,
                baud: baud_rate,
                buffer: std::collections::VecDeque::new(),
            }
        }
    }

    impl UartRiscv for MockUartRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn baud_rate(&self) -> u32 {
            self.baud
        }

        fn write(&mut self, data: &[u8]) -> Result<usize, UartError> {
            for &b in data {
                self.buffer.push_back(b);
            }
            Ok(data.len())
        }

        fn read(&mut self, buffer: &mut [u8]) -> Result<usize, UartError> {
            let mut count = 0;
            for b in buffer.iter_mut() {
                if let Some(byte) = self.buffer.pop_front() {
                    *b = byte;
                    count += 1;
                } else {
                    break;
                }
            }
            Ok(count)
        }

        fn flush(&mut self) -> Result<(), UartError> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_uart() {
        use mock::*;
        let mut uart = MockUartRiscv::new(Platform::Esp32C3, 115200);
        assert_eq!(uart.platform(), Platform::Esp32C3);
        
        uart.write(b"Hello").unwrap();
        let mut buf = [0u8; 10];
        let len = uart.read(&mut buf).unwrap();
        assert_eq!(len, 5);
    }
}
