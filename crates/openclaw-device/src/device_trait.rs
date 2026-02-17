use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::capabilities::DeviceCapabilities;
use crate::platform::{ComputeCategory, Platform};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDevice {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub category: ComputeCategory,
    pub capabilities: DeviceCapabilities,
    pub config: DeviceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceConfig {
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: Option<u64>,
    pub metadata: std::collections::HashMap<String, String>,
}

#[async_trait]
pub trait Device: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn platform(&self) -> Platform;
    fn category(&self) -> ComputeCategory;
    fn capabilities(&self) -> &DeviceCapabilities;

    async fn init(&self, config: &DeviceConfig) -> DeviceResult<()>;
    async fn health_check(&self) -> DeviceResult<bool>;
    async fn execute(&self, command: &DeviceCommand) -> DeviceResult<DeviceResponse>;
}

pub type DeviceResult<T> = Result<T, DeviceError>;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("Device not found: {0}")]
    NotFound(String),

    #[error("Device offline: {0}")]
    Offline(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCommand {
    pub action: String,
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub struct DeviceBuilder {
    id: String,
    name: String,
    platform: Platform,
    category: ComputeCategory,
    capabilities: DeviceCapabilities,
    config: DeviceConfig,
}

impl DeviceBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            platform: Platform::Unknown,
            category: ComputeCategory::Edge,
            capabilities: DeviceCapabilities::default(),
            config: DeviceConfig::default(),
        }
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = platform;
        self
    }

    pub fn category(mut self, category: ComputeCategory) -> Self {
        self.category = category;
        self
    }

    pub fn capabilities(mut self, capabilities: DeviceCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn config(mut self, config: DeviceConfig) -> Self {
        self.config = config;
        self
    }

    pub fn status(mut self, status: super::DeviceStatus) -> Self {
        self
    }

    pub fn build(self) -> CustomDevice {
        CustomDevice {
            id: self.id,
            name: self.name,
            platform: self.platform,
            category: self.category,
            capabilities: self.capabilities,
            config: self.config,
        }
    }
}

impl CustomDevice {
    pub fn to_handle(&self, status: super::DeviceStatus) -> super::DeviceHandle {
        super::DeviceHandle {
            id: self.id.clone(),
            name: self.name.clone(),
            platform: self.platform,
            capabilities: self.capabilities.clone(),
            status,
        }
    }
}
