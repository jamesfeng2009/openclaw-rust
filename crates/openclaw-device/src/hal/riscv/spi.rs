//! RISC-V SPI Hardware Abstraction
//!
//! SPI trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait SpiRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn frequency(&self) -> u32;
    fn transfer(&self, data: &[u8]) -> Result<Vec<u8>, SpiError>;
    fn transfer_full(&self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError>;
}

#[derive(Debug, Clone)]
pub enum SpiError {
    TransferError,
    ChipSelectError,
    ClockError,
    NotSupported,
}

impl std::fmt::Display for SpiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TransferError => write!(f, "SPI transfer error"),
            Self::ChipSelectError => write!(f, "SPI chip select error"),
            Self::ClockError => write!(f, "SPI clock error"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for SpiError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockSpiRiscv {
        plat: Platform,
        freq: u32,
    }

    impl MockSpiRiscv {
        pub fn new(platform: Platform, frequency: u32) -> Self {
            Self { plat: platform, freq: frequency }
        }
    }

    impl SpiRiscv for MockSpiRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn frequency(&self) -> u32 {
            self.freq
        }

        fn transfer(&self, data: &[u8]) -> Result<Vec<u8>, SpiError> {
            Ok(data.to_vec())
        }

        fn transfer_full(&self, tx: &[u8], rx: &mut [u8]) -> Result<(), SpiError> {
            rx.copy_from_slice(tx);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_spi() {
        use mock::*;
        let spi = MockSpiRiscv::new(Platform::KendryteK210, 1_000_000);
        assert_eq!(spi.platform(), Platform::KendryteK210);
        
        let result = spi.transfer(&[0xAA, 0x55]).unwrap();
        assert_eq!(result, vec![0xAA, 0x55]);
    }
}
