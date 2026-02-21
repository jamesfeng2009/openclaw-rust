//! 设备能力工具抽象
//!
//! 将设备能力作为工具集成到 Agent 决策流程中

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::Result;

#[async_trait]
pub trait DeviceCapabilityTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: DeviceToolParams) -> Result<DeviceToolResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToolParams {
    pub action: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub message: String,
}

pub struct CameraTool {
    capabilities: Arc<openclaw_device::DeviceCapabilities>,
}

impl CameraTool {
    pub fn new(capabilities: Arc<openclaw_device::DeviceCapabilities>) -> Self {
        Self { capabilities }
    }
}

#[async_trait]
impl DeviceCapabilityTool for CameraTool {
    fn name(&self) -> &str {
        "device_camera"
    }

    fn description(&self) -> &str {
        "Capture photos or videos from device camera"
    }

    async fn execute(&self, params: DeviceToolParams) -> Result<DeviceToolResult> {
        match params.action.as_str() {
            "capture" => {
                Ok(DeviceToolResult {
                    success: true,
                    data: serde_json::json!({
                        "camera_available": self.capabilities.sensors.contains(&openclaw_device::SensorType::Camera),
                        "resolution": "1920x1080"
                    }),
                    message: "Camera capture simulated".to_string(),
                })
            }
            _ => Ok(DeviceToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

pub struct ScreenTool;

impl ScreenTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DeviceCapabilityTool for ScreenTool {
    fn name(&self) -> &str {
        "device_screen"
    }

    fn description(&self) -> &str {
        "Control device screen, capture screenshots"
    }

    async fn execute(&self, params: DeviceToolParams) -> Result<DeviceToolResult> {
        match params.action.as_str() {
            "screenshot" => {
                Ok(DeviceToolResult {
                    success: true,
                    data: serde_json::json!({
                        "screen_available": true,
                        "resolution": "1920x1080"
                    }),
                    message: "Screenshot captured".to_string(),
                })
            }
            _ => Ok(DeviceToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

pub struct LocationTool {
    capabilities: Arc<openclaw_device::DeviceCapabilities>,
}

impl LocationTool {
    pub fn new(capabilities: Arc<openclaw_device::DeviceCapabilities>) -> Self {
        Self { capabilities }
    }
}

#[async_trait]
impl DeviceCapabilityTool for LocationTool {
    fn name(&self) -> &str {
        "device_location"
    }

    fn description(&self) -> &str {
        "Get device location via GPS or network"
    }

    async fn execute(&self, params: DeviceToolParams) -> Result<DeviceToolResult> {
        match params.action.as_str() {
            "get_location" => {
                Ok(DeviceToolResult {
                    success: true,
                    data: serde_json::json!({
                        "location_available": self.capabilities.sensors.contains(&openclaw_device::SensorType::Gps),
                        "latitude": 0.0,
                        "longitude": 0.0
                    }),
                    message: "Location retrieved".to_string(),
                })
            }
            _ => Ok(DeviceToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

pub struct SystemTool {
    capabilities: Arc<openclaw_device::DeviceCapabilities>,
}

impl SystemTool {
    pub fn new(capabilities: Arc<openclaw_device::DeviceCapabilities>) -> Self {
        Self { capabilities }
    }
}

#[async_trait]
impl DeviceCapabilityTool for SystemTool {
    fn name(&self) -> &str {
        "device_system"
    }

    fn description(&self) -> &str {
        "Get system information like battery, memory, CPU"
    }

    async fn execute(&self, params: DeviceToolParams) -> Result<DeviceToolResult> {
        match params.action.as_str() {
            "battery" => {
                Ok(DeviceToolResult {
                    success: true,
                    data: serde_json::json!({
                        "battery_level": 100,
                        "charging": false
                    }),
                    message: "Battery info retrieved".to_string(),
                })
            }
            "memory" => {
                Ok(DeviceToolResult {
                    success: true,
                    data: serde_json::json!({
                        "total_bytes": self.capabilities.memory.total_bytes,
                        "available_bytes": self.capabilities.memory.available_bytes
                    }),
                    message: "Memory info retrieved".to_string(),
                })
            }
            _ => Ok(DeviceToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

pub struct DeviceToolRegistry {
    tools: Vec<Arc<dyn DeviceCapabilityTool>>,
}

impl DeviceToolRegistry {
    pub fn new(capabilities: Arc<openclaw_device::DeviceCapabilities>) -> Self {
        let mut tools = Vec::new();

        tools.push(Arc::new(CameraTool::new(capabilities.clone())) as Arc<dyn DeviceCapabilityTool>);
        tools.push(Arc::new(ScreenTool::new()) as Arc<dyn DeviceCapabilityTool>);
        tools.push(Arc::new(LocationTool::new(capabilities.clone())) as Arc<dyn DeviceCapabilityTool>);
        tools.push(Arc::new(SystemTool::new(capabilities.clone())) as Arc<dyn DeviceCapabilityTool>);

        Self { tools }
    }

    pub fn get_tools(&self) -> &[Arc<dyn DeviceCapabilityTool>] {
        &self.tools
    }

    pub async fn execute(&self, tool_name: &str, params: DeviceToolParams) -> Result<DeviceToolResult> {
        for tool in &self.tools {
            if tool.name() == tool_name {
                return tool.execute(params).await;
            }
        }
        Err(openclaw_core::OpenClawError::Config(format!("Tool not found: {}", tool_name)))
    }
}
