//! RISC-V I2C Hardware Abstraction
//!
//! I2C trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait I2cRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn frequency(&self) -> u32;
    fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), I2cError>;
    fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<usize, I2cError>;
    fn write_read(&mut self, addr: u8, write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, I2cError>;
}

#[derive(Debug, Clone)]
pub enum I2cError {
    AddressNack,
    BusError,
    Timeout,
    InvalidAddress,
    WriteError,
    ReadError,
    NotSupported,
}

impl std::fmt::Display for I2cError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AddressNack => write!(f, "I2C address not acknowledged"),
            Self::BusError => write!(f, "I2C bus error"),
            Self::Timeout => write!(f, "I2C operation timeout"),
            Self::InvalidAddress => write!(f, "Invalid I2C address"),
            Self::WriteError => write!(f, "I2C write error"),
            Self::ReadError => write!(f, "I2C read error"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for I2cError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockI2cRiscv {
        plat: Platform,
        freq: u32,
        memory: std::collections::HashMap<u8, Vec<u8>>,
    }

    impl MockI2cRiscv {
        pub fn new(platform: Platform, frequency: u32) -> Self {
            Self {
                plat: platform,
                freq: frequency,
                memory: std::collections::HashMap::new(),
            }
        }
    }

    impl I2cRiscv for MockI2cRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn frequency(&self) -> u32 {
            self.freq
        }

        fn write(&mut self, addr: u8, data: &[u8]) -> Result<(), I2cError> {
            if addr == 0 || addr > 127 {
                return Err(I2cError::InvalidAddress);
            }
            self.memory.insert(addr, data.to_vec());
            Ok(())
        }

        fn read(&mut self, addr: u8, buffer: &mut [u8]) -> Result<usize, I2cError> {
            if addr == 0 || addr > 127 {
                return Err(I2cError::InvalidAddress);
            }
            if let Some(data) = self.memory.get(&addr) {
                let len = buffer.len().min(data.len());
                buffer[..len].copy_from_slice(&data[..len]);
                Ok(len)
            } else {
                Ok(0)
            }
        }

        fn write_read(&mut self, addr: u8, write_data: &[u8], read_buffer: &mut [u8]) -> Result<usize, I2cError> {
            self.write(addr, write_data)?;
            self.read(addr, read_buffer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_i2c_write_read() {
        use mock::*;
        let mut i2c = MockI2cRiscv::new(Platform::Esp32C3, 100_000);
        assert_eq!(i2c.platform(), Platform::Esp32C3);
        
        i2c.write(0x76, &[0x00, 0x01]).unwrap();
        let mut buf = [0u8; 4];
        let len = i2c.read(0x76, &mut buf).unwrap();
        assert!(len > 0);
    }
}
