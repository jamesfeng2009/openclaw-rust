use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_device::unified_manager::UnifiedDeviceManager;
use super::device_tool_registry::{DeviceTool, DeviceToolResult};

pub struct CameraCaptureTool {
    device_manager: Arc<UnifiedDeviceManager>,
}

impl CameraCaptureTool {
    pub fn new(device_manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { device_manager }
    }
}

#[async_trait]
impl DeviceTool for CameraCaptureTool {
    fn name(&self) -> &str {
        "camera_capture"
    }
    
    fn description(&self) -> &str {
        "Capture a photo from the device camera"
    }
    
    async fn execute(&self, params: serde_json::Value) -> DeviceToolResult {
        let device_id = params.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        match self.device_manager.capture_camera(device_id).await {
            Ok(result) => {
                if result.success {
                    DeviceToolResult::success(serde_json::json!({
                        "image_data": result.data,
                        "mime_type": result.mime_type,
                        "timestamp": result.timestamp,
                        "device_id": device_id,
                    }))
                } else {
                    DeviceToolResult::failure(result.error.unwrap_or_else(|| "Capture failed".to_string()))
                }
            }
            Err(e) => DeviceToolResult::failure(e.to_string()),
        }
    }
}

pub struct ScreenCaptureTool {
    device_manager: Arc<UnifiedDeviceManager>,
}

impl ScreenCaptureTool {
    pub fn new(device_manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { device_manager }
    }
}

#[async_trait]
impl DeviceTool for ScreenCaptureTool {
    fn name(&self) -> &str {
        "screen_capture"
    }
    
    fn description(&self) -> &str {
        "Capture a screenshot from the device screen"
    }
    
    async fn execute(&self, params: serde_json::Value) -> DeviceToolResult {
        let device_id = params.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        match self.device_manager.capture_screen(device_id).await {
            Ok(result) => {
                if result.success {
                    DeviceToolResult::success(serde_json::json!({
                        "image_data": result.data,
                        "mime_type": result.mime_type,
                        "timestamp": result.timestamp,
                        "device_id": device_id,
                    }))
                } else {
                    DeviceToolResult::failure(result.error.unwrap_or_else(|| "Screenshot failed".to_string()))
                }
            }
            Err(e) => DeviceToolResult::failure(e.to_string()),
        }
    }
}

pub struct LocationTool {
    device_manager: Arc<UnifiedDeviceManager>,
}

impl LocationTool {
    pub fn new(device_manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { device_manager }
    }
}

#[async_trait]
impl DeviceTool for LocationTool {
    fn name(&self) -> &str {
        "location_get"
    }
    
    fn description(&self) -> &str {
        "Get the device location via GPS or network"
    }
    
    async fn execute(&self, params: serde_json::Value) -> DeviceToolResult {
        let device_id = params.get("device_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        
        match self.device_manager.get_location_data(device_id).await {
            Ok(result) => {
                if result.success {
                    DeviceToolResult::success(serde_json::json!({
                        "latitude": result.latitude,
                        "longitude": result.longitude,
                        "altitude": result.altitude,
                        "accuracy": result.accuracy,
                        "timestamp": result.timestamp,
                        "device_id": device_id,
                    }))
                } else {
                    DeviceToolResult::failure(result.error.unwrap_or_else(|| "Location failed".to_string()))
                }
            }
            Err(e) => DeviceToolResult::failure(e.to_string()),
        }
    }
}

pub struct DeviceListTool {
    device_manager: Arc<UnifiedDeviceManager>,
}

impl DeviceListTool {
    pub fn new(device_manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { device_manager }
    }
}

#[async_trait]
impl DeviceTool for DeviceListTool {
    fn name(&self) -> &str {
        "device_list"
    }
    
    fn description(&self) -> &str {
        "List all available device capabilities"
    }
    
    async fn execute(&self, _params: serde_json::Value) -> DeviceToolResult {
        let devices = self.device_manager.list_capabilities().await;
        DeviceToolResult::success(serde_json::json!({
            "devices": devices,
        }))
    }
}

pub struct DeviceManagerInitializer {
    device_manager: Arc<UnifiedDeviceManager>,
}

impl DeviceManagerInitializer {
    pub fn new(device_manager: Arc<UnifiedDeviceManager>) -> Self {
        Self { device_manager }
    }
    
    pub fn register_default_tools(
        &self,
        registry: &mut super::device_tool_registry::DeviceToolRegistry,
    ) {
        registry.register(CameraCaptureTool::new(self.device_manager.clone()));
        registry.register(ScreenCaptureTool::new(self.device_manager.clone()));
        registry.register(LocationTool::new(self.device_manager.clone()));
        registry.register(DeviceListTool::new(self.device_manager.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_device::unified_manager::UnifiedDeviceManager;
    use openclaw_device::registry::DeviceRegistry;
    use crate::DeviceToolRegistry;
    
    fn create_test_manager() -> Arc<UnifiedDeviceManager> {
        let registry = Arc::new(DeviceRegistry::new());
        Arc::new(UnifiedDeviceManager::new(registry))
    }
    
    #[tokio::test]
    async fn test_camera_tool_creation() {
        let manager = create_test_manager();
        let tool = CameraCaptureTool::new(manager);
        
        assert_eq!(tool.name(), "camera_capture");
        assert!(!tool.description().is_empty());
    }
    
    #[tokio::test]
    async fn test_screen_tool_creation() {
        let manager = create_test_manager();
        let tool = ScreenCaptureTool::new(manager);
        
        assert_eq!(tool.name(), "screen_capture");
        assert!(!tool.description().is_empty());
    }
    
    #[tokio::test]
    async fn test_location_tool_creation() {
        let manager = create_test_manager();
        let tool = LocationTool::new(manager);
        
        assert_eq!(tool.name(), "location_get");
        assert!(!tool.description().is_empty());
    }
    
    #[tokio::test]
    async fn test_device_list_tool_creation() {
        let manager = create_test_manager();
        let tool = DeviceListTool::new(manager);
        
        assert_eq!(tool.name(), "device_list");
        assert!(!tool.description().is_empty());
    }
    
    #[tokio::test]
    async fn test_camera_tool_not_found() {
        let manager = create_test_manager();
        let tool = CameraCaptureTool::new(manager);
        
        let result = tool.execute(serde_json::json!({"device_id": "nonexistent"})).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
    
    #[tokio::test]
    async fn test_screen_tool_not_found() {
        let manager = create_test_manager();
        let tool = ScreenCaptureTool::new(manager);
        
        let result = tool.execute(serde_json::json!({"device_id": "nonexistent"})).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
    
    #[tokio::test]
    async fn test_location_tool_not_found() {
        let manager = create_test_manager();
        let tool = LocationTool::new(manager);
        
        let result = tool.execute(serde_json::json!({"device_id": "nonexistent"})).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
    
    #[tokio::test]
    async fn test_device_list_tool() {
        let manager = create_test_manager();
        let tool = DeviceListTool::new(manager);
        
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.success);
        assert!(result.data.is_some());
    }
    
    #[tokio::test]
    async fn test_device_manager_initializer() {
        let manager = create_test_manager();
        let initializer = DeviceManagerInitializer::new(manager);
        
        let mut registry = DeviceToolRegistry::new();
        initializer.register_default_tools(&mut registry);
        
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 4);
        
        let tool_names: Vec<_> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"camera_capture".to_string()));
        assert!(tool_names.contains(&"screen_capture".to_string()));
        assert!(tool_names.contains(&"location_get".to_string()));
        assert!(tool_names.contains(&"device_list".to_string()));
    }
    
    #[tokio::test]
    async fn test_tool_with_default_device_id() {
        let manager = create_test_manager();
        let tool = CameraCaptureTool::new(manager);
        
        let result = tool.execute(serde_json::json!({})).await;
        assert!(!result.success);
    }
}
