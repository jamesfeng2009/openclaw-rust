//! GPIO 接口
//!
//! 通用 GPIO (通用输入输出) 接口

use crate::platform::Platform;
use crate::hal::{HalModule, HalConfig, HalResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type GpioResult<T> = crate::hal::HalResult<T>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpioMode {
    Input,
    Output,
    InputPullUp,
    InputPullDown,
    InputPullUpDown,
    Alternate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpioState {
    Low,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpioPinInfo {
    pub number: u32,
    pub mode: GpioMode,
    pub state: GpioState,
    pub name: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum GpioError {
    #[error("Pin not found: {0}")]
    PinNotFound(u32),
    #[error("Pin already in use: {0}")]
    PinInUse(u32),
    #[error("Invalid mode for pin: {0}")]
    InvalidMode(u32),
    #[error("Permission denied for pin: {0}")]
    PermissionDenied(u32),
    #[error("IO error: {0}")]
    IoError(String),
}

#[async_trait]
pub trait GpioPin: Send + Sync {
    fn pin_number(&self) -> u32;
    
    fn mode(&self) -> GpioMode;
    
    fn read(&self) -> GpioResult<GpioState>;
    
    fn write(&self, state: GpioState) -> GpioResult<()>;
    
    async fn toggle(&self) -> GpioResult<()> {
        let current = self.read()?;
        let new_state = match current {
            GpioState::Low => GpioState::High,
            GpioState::High => GpioState::Low,
        };
        self.write(new_state)
    }
}

#[async_trait]
pub trait GpioBus: Send + Sync {
    async fn init(&self, config: &HalConfig) -> HalResult<()>;
    
    async fn health_check(&self) -> HalResult<bool>;
    
    fn name(&self) -> &str;
    
    fn supported_platforms(&self) -> &[Platform];
    
    fn is_available(&self) -> bool;
    
    fn pin_count(&self) -> usize;
    
    fn get_pin(&self, number: u32) -> GpioResult<Box<dyn GpioPin>>;
    
    fn list_pins(&self) -> Vec<GpioPinInfo>;
    
    fn supports_interrupt(&self) -> bool;
}
