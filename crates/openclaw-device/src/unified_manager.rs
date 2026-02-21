use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::camera::CameraManager;
use crate::screen::ScreenManager;
use crate::location::LocationManager;
use crate::registry::DeviceRegistry;
use crate::nodes::DeviceError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub device_type: DeviceType,
    pub name: String,
    pub status: String,
    pub can_capture_photo: bool,
    pub can_capture_screen: bool,
    pub can_get_location: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Camera,
    Screen,
    Location,
    Custom,
}

impl DeviceType {
    pub fn as_str(&self) -> &str {
        match self {
            DeviceType::Camera => "camera",
            DeviceType::Screen => "screen",
            DeviceType::Location => "location",
            DeviceType::Custom => "custom",
        }
    }
}

pub enum DeviceCapabilityRef {
    Camera(Arc<CameraManager>),
    Screen(Arc<ScreenManager>),
    Location(Arc<LocationManager>),
}

pub struct UnifiedDeviceManager {
    capabilities: Arc<RwLock<HashMap<String, DeviceCapabilityRef>>>,
    registry: Arc<DeviceRegistry>,
}

impl UnifiedDeviceManager {
    pub fn new(registry: Arc<DeviceRegistry>) -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
            registry,
        }
    }
    
    pub async fn register_camera(&self, id: impl Into<String>, manager: CameraManager) {
        let id = id.into();
        let mut caps = self.capabilities.write().await;
        caps.insert(id, DeviceCapabilityRef::Camera(Arc::new(manager)));
    }
    
    pub async fn register_screen(&self, id: impl Into<String>, manager: ScreenManager) {
        let id = id.into();
        let mut caps = self.capabilities.write().await;
        caps.insert(id, DeviceCapabilityRef::Screen(Arc::new(manager)));
    }
    
    pub async fn register_location(&self, id: impl Into<String>, manager: LocationManager) {
        let id = id.into();
        let mut caps = self.capabilities.write().await;
        caps.insert(id, DeviceCapabilityRef::Location(Arc::new(manager)));
    }
    
    pub async fn get_camera(&self, id: &str) -> Option<Arc<CameraManager>> {
        let caps = self.capabilities.read().await;
        match caps.get(id) {
            Some(DeviceCapabilityRef::Camera(mgr)) => Some(mgr.clone()),
            _ => None,
        }
    }
    
    pub async fn get_screen(&self, id: &str) -> Option<Arc<ScreenManager>> {
        let caps = self.capabilities.read().await;
        match caps.get(id) {
            Some(DeviceCapabilityRef::Screen(mgr)) => Some(mgr.clone()),
            _ => None,
        }
    }
    
    pub async fn get_location(&self, id: &str) -> Option<Arc<LocationManager>> {
        let caps = self.capabilities.read().await;
        match caps.get(id) {
            Some(DeviceCapabilityRef::Location(mgr)) => Some(mgr.clone()),
            _ => None,
        }
    }
    
    pub async fn capture_camera(&self, id: &str) -> Result<crate::nodes::CaptureResult, DeviceError> {
        let camera = self.get_camera(id).await
            .ok_or_else(|| DeviceError::DeviceNotFound(format!("Camera '{}' not found", id)))?;
        camera.capture_photo(None).await
    }
    
    pub async fn capture_screen(&self, id: &str) -> Result<crate::nodes::CaptureResult, DeviceError> {
        let screen = self.get_screen(id).await
            .ok_or_else(|| DeviceError::DeviceNotFound(format!("Screen '{}' not found", id)))?;
        screen.screenshot(None).await
    }
    
    pub async fn get_location_data(&self, id: &str) -> Result<crate::nodes::LocationResult, DeviceError> {
        let location = self.get_location(id).await
            .ok_or_else(|| DeviceError::DeviceNotFound(format!("Location '{}' not found", id)))?;
        location.get_location().await
    }
    
    pub async fn list_capabilities(&self) -> Vec<DeviceInfo> {
        let caps = self.capabilities.read().await;
        let mut infos = Vec::new();
        
        for (id, cap) in caps.iter() {
            let device_type = match cap {
                DeviceCapabilityRef::Camera(_) => DeviceType::Camera,
                DeviceCapabilityRef::Screen(_) => DeviceType::Screen,
                DeviceCapabilityRef::Location(_) => DeviceType::Location,
            };
            
            let (can_capture_photo, can_capture_screen, can_get_location) = match cap {
                DeviceCapabilityRef::Camera(_) => (true, false, false),
                DeviceCapabilityRef::Screen(_) => (false, true, false),
                DeviceCapabilityRef::Location(_) => (false, false, true),
            };
            
            infos.push(DeviceInfo {
                id: id.clone(),
                device_type,
                name: id.clone(),
                status: "online".to_string(),
                can_capture_photo,
                can_capture_screen,
                can_get_location,
            });
        }
        
        infos
    }
    
    pub fn registry(&self) -> Arc<DeviceRegistry> {
        self.registry.clone()
    }
}

impl Default for UnifiedDeviceManager {
    fn default() -> Self {
        Self::new(Arc::new(DeviceRegistry::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_unified_manager_creation() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        let infos = manager.list_capabilities().await;
        assert!(infos.is_empty());
    }
    
    #[tokio::test]
    async fn test_register_and_get_camera() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_camera("test_cam", CameraManager::new()).await;
        
        let camera = manager.get_camera("test_cam").await;
        assert!(camera.is_some());
        
        let nonexistent = manager.get_camera("nonexistent").await;
        assert!(nonexistent.is_none());
    }
    
    #[tokio::test]
    async fn test_register_and_get_screen() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_screen("test_screen", ScreenManager::new()).await;
        
        let screen = manager.get_screen("test_screen").await;
        assert!(screen.is_some());
    }
    
    #[tokio::test]
    async fn test_register_and_get_location() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_location("test_loc", LocationManager::new()).await;
        
        let location = manager.get_location("test_loc").await;
        assert!(location.is_some());
    }
    
    #[tokio::test]
    async fn test_list_capabilities() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_camera("cam1", CameraManager::new()).await;
        manager.register_screen("screen1", ScreenManager::new()).await;
        manager.register_location("loc1", LocationManager::new()).await;
        
        let infos = manager.list_capabilities().await;
        assert_eq!(infos.len(), 3);
        
        let types: Vec<_> = infos.iter().map(|i| i.device_type).collect();
        assert!(types.contains(&DeviceType::Camera));
        assert!(types.contains(&DeviceType::Screen));
        assert!(types.contains(&DeviceType::Location));
    }
    
    #[tokio::test]
    async fn test_capture_camera_not_found() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        let result = manager.capture_camera("nonexistent").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_capture_screen_not_found() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        let result = manager.capture_screen("nonexistent").await;
        assert!(result.is_err());
    }
    
    #[test]
    fn test_device_type_as_str() {
        assert_eq!(DeviceType::Camera.as_str(), "camera");
        assert_eq!(DeviceType::Screen.as_str(), "screen");
        assert_eq!(DeviceType::Location.as_str(), "location");
        assert_eq!(DeviceType::Custom.as_str(), "custom");
    }
    
    #[tokio::test]
    async fn test_default_manager() {
        let manager = UnifiedDeviceManager::default();
        let infos = manager.list_capabilities().await;
        assert!(infos.is_empty());
    }
    
    #[tokio::test]
    async fn test_device_info_fields() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_camera("test_cam", CameraManager::new()).await;
        
        let infos = manager.list_capabilities().await;
        assert_eq!(infos.len(), 1);
        
        let info = &infos[0];
        assert_eq!(info.id, "test_cam");
        assert_eq!(info.name, "test_cam");
        assert_eq!(info.status, "online");
        assert!(info.can_capture_photo);
        assert!(!info.can_capture_screen);
        assert!(!info.can_get_location);
    }
    
    #[tokio::test]
    async fn test_multiple_devices_same_type() {
        let registry = Arc::new(DeviceRegistry::new());
        let manager = UnifiedDeviceManager::new(registry);
        
        manager.register_camera("cam1", CameraManager::new()).await;
        manager.register_camera("cam2", CameraManager::new()).await;
        
        let infos = manager.list_capabilities().await;
        assert_eq!(infos.len(), 2);
        
        let camera_count = infos.iter().filter(|i| i.device_type == DeviceType::Camera).count();
        assert_eq!(camera_count, 2);
    }
}
