use async_trait::async_trait;
use std::sync::Arc;
use openclaw_core::Result as OpenClawResult;
use openclaw_device::{CameraManager, DeviceCapabilities, ScreenManager, SensorType};

#[derive(Debug)]
pub enum HardwareError {
    DeviceError(String),
    NotAvailable(String),
    NotInitialized(String),
}

impl std::fmt::Display for HardwareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HardwareError::DeviceError(msg) => write!(f, "Device error: {}", msg),
            HardwareError::NotAvailable(msg) => write!(f, "Not available: {}", msg),
            HardwareError::NotInitialized(msg) => write!(f, "Not initialized: {}", msg),
        }
    }
}

impl std::error::Error for HardwareError {}

#[async_trait]
pub trait CameraCapture: Send + Sync {
    async fn capture_photo(&self, device_index: Option<u32>) -> Result<CaptureResult, HardwareError>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CaptureResult {
    pub success: bool,
    pub data: Option<String>,
    pub mime_type: String,
    pub timestamp: i64,
    pub error: Option<String>,
}

#[async_trait]
pub trait ScreenCapture: Send + Sync {
    async fn screenshot(&self, display_id: Option<u32>) -> Result<ScreenshotResult, HardwareError>;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScreenshotResult {
    pub success: bool,
    pub data: Option<String>,
    pub mime_type: String,
    pub timestamp: i64,
    pub error: Option<String>,
}

pub struct CameraTool {
    camera_manager: Option<Arc<CameraManager>>,
    capabilities: DeviceCapabilities,
}

impl CameraTool {
    pub fn new(
        camera_manager: Option<Arc<CameraManager>>,
        capabilities: DeviceCapabilities,
    ) -> Self {
        Self {
            camera_manager,
            capabilities,
        }
    }
}

#[async_trait]
impl openclaw_tools::Tool for CameraTool {
    fn name(&self) -> &str {
        "hardware_camera"
    }

    fn description(&self) -> &str {
        "Capture photos from device camera. Actions: capture (default)"
    }

    async fn execute(&self, args: serde_json::Value) -> OpenClawResult<serde_json::Value> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("capture");

        match action {
            "capture" => {
                let device_index = args.get("device_index")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let available = self.capabilities.sensors.contains(&SensorType::Camera);

                if !available {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": "Camera not available on this device"
                    }));
                }

                if let Some(ref camera) = self.camera_manager {
                    match camera.capture_photo(device_index).await {
                        Ok(result) => return Ok(serde_json::json!({
                            "success": result.success,
                            "data": result.data,
                            "mime_type": result.mime_type,
                            "timestamp": result.timestamp,
                            "error": result.error
                        })),
                        Err(e) => return Ok(serde_json::json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    }
                }

                Ok(serde_json::json!({
                    "success": false,
                    "error": "Camera not initialized"
                }))
            }
            _ => Ok(serde_json::json!({
                "success": false,
                "error": format!("Unknown action: {}", action)
            })),
        }
    }
}

pub struct ScreenTool {
    screen_manager: Option<Arc<ScreenManager>>,
    capabilities: DeviceCapabilities,
}

impl ScreenTool {
    pub fn new(
        screen_manager: Option<Arc<ScreenManager>>,
        capabilities: DeviceCapabilities,
    ) -> Self {
        Self {
            screen_manager,
            capabilities,
        }
    }
}

#[async_trait]
impl openclaw_tools::Tool for ScreenTool {
    fn name(&self) -> &str {
        "hardware_screenshot"
    }

    fn description(&self) -> &str {
        "Capture screenshots of the device screen. Actions: capture (default)"
    }

    async fn execute(&self, args: serde_json::Value) -> OpenClawResult<serde_json::Value> {
        let action = args.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("capture");

        match action {
            "capture" => {
                let display_id = args.get("display")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                if let Some(ref screen) = self.screen_manager {
                    match screen.screenshot(display_id).await {
                        Ok(result) => return Ok(serde_json::json!({
                            "success": result.success,
                            "data": result.data,
                            "mime_type": result.mime_type,
                            "timestamp": result.timestamp,
                            "error": result.error
                        })),
                        Err(e) => return Ok(serde_json::json!({
                            "success": false,
                            "error": e.to_string()
                        })),
                    }
                }

                Ok(serde_json::json!({
                    "success": false,
                    "error": "Screen capture not initialized"
                }))
            }
            _ => Ok(serde_json::json!({
                "success": false,
                "error": format!("Unknown action: {}", action)
            })),
        }
    }
}
