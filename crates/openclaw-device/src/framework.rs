//! 框架集成层模块接口
//!
//! 提供与外部框架和协议的集成接口

pub mod ros2;
pub mod mqtt;
pub mod can;

pub use ros2::{Ros2Client, Ros2Topic, Ros2Service, Ros2Action, Ros2Error, Ros2Result, Ros2TopicInfo, Ros2ServiceInfo};
pub use mqtt::{MqttClient, MqttConfig, MqttMessage, MqttError, MqttResult, MqttSubscription, MqttQos};
pub use can::{CanBus, CanFrame, CanFilter, CanError, CanResult, CanId, CanState, CanBusInfo};

use crate::platform::Platform;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    pub enabled: bool,
    pub endpoint: Option<String>,
    pub config: std::collections::HashMap<String, String>,
}

#[async_trait]
pub trait FrameworkModule: Send + Sync {
    fn name(&self) -> &str;
    
    fn supported_platforms(&self) -> &[Platform];
    
    fn is_connected(&self) -> bool;
    
    async fn connect(&self, config: &FrameworkConfig) -> FrameworkResult<()>;
    
    async fn disconnect(&self) -> FrameworkResult<()>;
    
    async fn health_check(&self) -> FrameworkResult<bool>;
}

pub type FrameworkResult<T> = Result<T, FrameworkError>;

#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Not connected")]
    NotConnected,
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("Message error: {0}")]
    MessageError(String),
    
    #[error("Service error: {0}")]
    ServiceError(String),
    
    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}
