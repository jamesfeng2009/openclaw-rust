//! 设备能力 API 路由

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use openclaw_device::{UnifiedDeviceManager, DeviceInfo};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize)]
pub struct CaptureResponse {
    pub success: bool,
    pub data: Option<String>,
    pub mime_type: String,
    pub timestamp: i64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DeviceListResponse {
    pub success: bool,
    pub devices: Vec<DeviceInfo>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct DeviceApiState {
    pub device_manager: Arc<UnifiedDeviceManager>,
}

async fn list_devices(State(state): State<DeviceApiState>) -> Json<DeviceListResponse> {
    info!("Listing all devices");
    
    let devices = state.device_manager.list_capabilities().await;
    Json(DeviceListResponse {
        success: true,
        devices,
        error: None,
    })
}

async fn capture_camera(
    Path(id): Path<String>,
    State(state): State<DeviceApiState>,
) -> Json<CaptureResponse> {
    info!("Capturing camera with id: {}", id);
    
    match state.device_manager.capture_camera(&id).await {
        Ok(result) => Json(CaptureResponse {
            success: result.success,
            data: result.data,
            mime_type: result.mime_type,
            timestamp: result.timestamp,
            error: result.error,
        }),
        Err(e) => Json(CaptureResponse {
            success: false,
            data: None,
            mime_type: "".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            error: Some(e.to_string()),
        }),
    }
}

async fn capture_screen(
    Path(id): Path<String>,
    State(state): State<DeviceApiState>,
) -> Json<CaptureResponse> {
    info!("Capturing screen with id: {}", id);
    
    match state.device_manager.capture_screen(&id).await {
        Ok(result) => Json(CaptureResponse {
            success: result.success,
            data: result.data,
            mime_type: result.mime_type,
            timestamp: result.timestamp,
            error: result.error,
        }),
        Err(e) => Json(CaptureResponse {
            success: false,
            data: None,
            mime_type: "".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            error: Some(e.to_string()),
        }),
    }
}

pub fn create_device_router(device_manager: Arc<UnifiedDeviceManager>) -> Router {
    let state = DeviceApiState { device_manager };
    
    Router::new()
        .route("/device/list", get(list_devices))
        .route("/device/camera/{id}/capture", post(capture_camera))
        .route("/device/screen/{id}/capture", post(capture_screen))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use openclaw_device::{CameraManager, ScreenManager};
    use tower::ServiceExt;
    
    #[tokio::test]
    async fn test_device_api_state_creation() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        let state = DeviceApiState { device_manager: manager };
        
        assert!(state.device_manager.get_camera("test").await.is_none());
    }
    
    #[tokio::test]
    async fn test_list_devices_empty() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        let devices = manager.list_capabilities().await;
        assert!(devices.is_empty());
    }
    
    #[tokio::test]
    async fn test_list_devices_with_camera() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        manager.register_camera("test_cam", CameraManager::new()).await;
        
        let devices = manager.list_capabilities().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "test_cam");
    }
    
    #[tokio::test]
    async fn test_list_devices_with_multiple_devices() {
        use openclaw_device::DeviceType;
        
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        manager.register_camera("cam1", CameraManager::new()).await;
        manager.register_camera("cam2", CameraManager::new()).await;
        manager.register_screen("screen1", ScreenManager::new()).await;
        
        let devices = manager.list_capabilities().await;
        assert_eq!(devices.len(), 3);
        
        let camera_count = devices.iter().filter(|d| d.device_type == DeviceType::Camera).count();
        let screen_count = devices.iter().filter(|d| d.device_type == DeviceType::Screen).count();
        assert_eq!(camera_count, 2);
        assert_eq!(screen_count, 1);
    }
    
    #[tokio::test]
    async fn test_get_camera_not_found() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        let camera = manager.get_camera("nonexistent").await;
        assert!(camera.is_none());
    }
    
    #[tokio::test]
    async fn test_get_screen_not_found() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        let screen = manager.get_screen("nonexistent").await;
        assert!(screen.is_none());
    }
    
    #[test]
    fn test_router_creation() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        let _router = create_device_router(manager);
    }
    
    #[tokio::test]
    async fn test_router_list_devices() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        let router = create_device_router(manager);
        
        let response = router
            .oneshot(Request::builder().uri("/device/list").body(Body::empty()).unwrap())
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_router_capture_camera_not_found() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        let router = create_device_router(manager);
        
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/device/camera/nonexistent/capture")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_router_capture_screen_not_found() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        let router = create_device_router(manager);
        
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/device/screen/nonexistent/capture")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
    
    #[tokio::test]
    async fn test_router_404_not_found() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        let router = create_device_router(manager);
        
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/nonexistent/route")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
