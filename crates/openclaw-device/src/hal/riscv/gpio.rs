//! RISC-V GPIO Hardware Abstraction
//!
//! GPIO trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait GpioRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn pin_count(&self) -> usize;
    fn set_direction(&mut self, pin: u8, direction: GpioDirection) -> Result<(), GpioError>;
    fn write(&mut self, pin: u8, value: bool) -> Result<(), GpioError>;
    fn read(&self, pin: u8) -> Result<bool, GpioError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpioDirection {
    Input,
    Output,
    InputPullUp,
    InputPullDown,
}

#[derive(Debug, Clone)]
pub enum GpioError {
    InvalidPin(u8),
    DirectionError,
    WriteError,
    ReadError,
    NotSupported,
}

impl std::fmt::Display for GpioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPin(pin) => write!(f, "Invalid GPIO pin: {}", pin),
            Self::DirectionError => write!(f, "Failed to set GPIO direction"),
            Self::WriteError => write!(f, "Failed to write GPIO"),
            Self::ReadError => write!(f, "Failed to read GPIO"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for GpioError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockGpioRiscv {
        plat: Platform,
        pins: std::collections::HashMap<u8, bool>,
    }

    impl MockGpioRiscv {
        pub fn new(platform: Platform) -> Self {
            Self {
                plat: platform,
                pins: std::collections::HashMap::new(),
            }
        }
    }

    impl GpioRiscv for MockGpioRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn pin_count(&self) -> usize {
            32
        }

        fn set_direction(&mut self, _pin: u8, _direction: GpioDirection) -> Result<(), GpioError> {
            Ok(())
        }

        fn write(&mut self, pin: u8, value: bool) -> Result<(), GpioError> {
            self.pins.insert(pin, value);
            Ok(())
        }

        fn read(&self, pin: u8) -> Result<bool, GpioError> {
            Ok(self.pins.get(&pin).copied().unwrap_or(false))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_gpio_direction() {
        let dir = GpioDirection::Input;
        assert_eq!(dir, GpioDirection::Input);
    }

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_gpio() {
        use mock::*;
        let mut gpio = MockGpioRiscv::new(Platform::KendryteK210);
        assert_eq!(gpio.platform(), Platform::KendryteK210);
        assert_eq!(gpio.pin_count(), 32);
        gpio.write(0, true).unwrap();
        assert_eq!(gpio.read(0).unwrap(), true);
    }
}
