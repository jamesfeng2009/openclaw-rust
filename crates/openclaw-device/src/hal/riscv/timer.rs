//! RISC-V Timer Hardware Abstraction
//!
//! Timer trait definitions for RISC-V platforms.

use crate::platform::Platform;

pub trait TimerRiscv: Send + Sync {
    fn platform(&self) -> Platform;
    fn frequency(&self) -> u32;
    fn get_count(&self) -> Result<u64, TimerError>;
    fn set_alarm(&self, period_us: u64) -> Result<(), TimerError>;
    fn clear_alarm(&self) -> Result<(), TimerError>;
}

#[derive(Debug, Clone)]
pub enum TimerError {
    NotRunning,
    AlarmError,
    NotSupported,
}

impl std::fmt::Display for TimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning => write!(f, "Timer not running"),
            Self::AlarmError => write!(f, "Timer alarm error"),
            Self::NotSupported => write!(f, "Operation not supported on this platform"),
        }
    }
}

impl std::error::Error for TimerError {}

#[cfg(feature = "riscv")]
pub mod mock {
    use super::*;

    pub struct MockTimerRiscv {
        plat: Platform,
        freq: u32,
        count: std::sync::atomic::AtomicU64,
    }

    impl MockTimerRiscv {
        pub fn new(platform: Platform, frequency: u32) -> Self {
            Self {
                plat: platform,
                freq: frequency,
                count: std::sync::atomic::AtomicU64::new(0),
            }
        }
    }

    impl TimerRiscv for MockTimerRiscv {
        fn platform(&self) -> Platform {
            self.plat
        }

        fn frequency(&self) -> u32 {
            self.freq
        }

        fn get_count(&self) -> Result<u64, TimerError> {
            Ok(self.count.load(std::sync::atomic::Ordering::Relaxed))
        }

        fn set_alarm(&self, _period_us: u64) -> Result<(), TimerError> {
            Ok(())
        }

        fn clear_alarm(&self) -> Result<(), TimerError> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "riscv")]
    #[test]
    fn test_mock_timer() {
        use mock::*;
        let timer = MockTimerRiscv::new(Platform::KendryteK210, 1_000_000);
        assert_eq!(timer.platform(), Platform::KendryteK210);
        
        let count = timer.get_count().unwrap();
        assert_eq!(count, 0);
    }
}
