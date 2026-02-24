//! RISC-V ADC Hardware Abstraction
//!
//! ADC trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait AdcRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn resolution_bits(&self) -> u8;
    fn read(&self, channel: u8) -> Result<u16, AdcError>;
    fn read_voltage(&self, channel: u8) -> Result<f32, AdcError>;
}

#[derive(Debug, Clone)]
pub enum AdcError {
    ChannelError,
    ReadError,
    CalibrationError,
    NotSupported,
}

impl std::fmt::Display for AdcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelError => write!(f, "Invalid ADC channel"),
            Self::ReadError => write!(f, "ADC read error"),
            Self::CalibrationError => write!(f, "ADC calibration error"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for AdcError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockAdcRiscv {
        plat: Platform,
        resolution: u8,
        vref: f32,
    }

    impl MockAdcRiscv {
        pub fn new(platform: Platform, resolution_bits: u8, vref: f32) -> Self {
            Self { plat: platform, resolution: resolution_bits, vref }
        }
    }

    impl AdcRiscv for MockAdcRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn resolution_bits(&self) -> u8 {
            self.resolution
        }

        fn read(&self, _channel: u8) -> Result<u16, AdcError> {
            let max = (1 << self.resolution) - 1;
            Ok(max / 2)
        }

        fn read_voltage(&self, channel: u8) -> Result<f32, AdcError> {
            let raw = self.read(channel)?;
            let max = (1 << self.resolution) as f32;
            Ok((raw as f32 / max) * self.vref)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_adc() {
        use mock::*;
        let adc = MockAdcRiscv::new(Platform::Esp32C3, 12, 3.3);
        assert_eq!(adc.platform(), Platform::Esp32C3);
        assert_eq!(adc.resolution_bits(), 12);
        
        let raw = adc.read(0).unwrap();
        assert!(raw > 0);
        
        let voltage = adc.read_voltage(0).unwrap();
        assert!(voltage > 0.0 && voltage < 3.3);
    }
}
