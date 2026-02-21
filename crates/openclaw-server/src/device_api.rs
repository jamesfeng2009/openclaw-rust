//! 设备能力 API 路由

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use openclaw_device::{CameraManager, ScreenManager, UnifiedDeviceManager, DeviceInfo};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct CapturePhotoRequest {
    pub device_index: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct CaptureResponse {
    pub success: bool,
    pub data: Option<String>,
    pub mime_type: String,
    pub timestamp: i64,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScreenshotRequest {
    pub display: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StartRecordingRequest {
    pub device_index: Option<u32>,
    pub display: Option<u32>,
    pub duration_secs: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RecordingResponse {
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

#[derive(Debug, Deserialize)]
pub struct SmartCaptureRequest {
    pub device_id: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SmartCaptureResponse {
    pub success: bool,
    pub image_data: Option<String>,
    pub analysis: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct DeviceApiState {
    pub device_manager: Arc<UnifiedDeviceManager>,
}

async fn capture_photo(Json(req): Json<CapturePhotoRequest>) -> Json<CaptureResponse> {
    info!("Capturing photo with device_index: {:?}", req.device_index);
    
    let camera_manager = CameraManager::new();
    match camera_manager.capture_photo(req.device_index).await {
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

async fn capture_screenshot(Json(req): Json<ScreenshotRequest>) -> Json<CaptureResponse> {
    info!("Capturing screenshot with display: {:?}", req.display);
    
    let screen_manager = ScreenManager::new();
    match screen_manager.screenshot(req.display).await {
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

async fn start_video_recording(Json(req): Json<StartRecordingRequest>) -> Json<RecordingResponse> {
    info!("Starting video recording: device_index={:?}, duration={:?}", 
          req.device_index, req.duration_secs);
    
    let camera_manager = CameraManager::new();
    match camera_manager.start_recording(req.device_index, req.duration_secs).await {
        Ok(result) => Json(RecordingResponse {
            success: result.success,
            data: result.data,
            mime_type: result.mime_type,
            timestamp: result.timestamp,
            error: result.error,
        }),
        Err(e) => Json(RecordingResponse {
            success: false,
            data: None,
            mime_type: "".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            error: Some(e.to_string()),
        }),
    }
}

async fn start_screen_recording(Json(req): Json<StartRecordingRequest>) -> Json<RecordingResponse> {
    info!("Starting screen recording: duration={:?}", req.duration_secs);
    
    let screen_manager = ScreenManager::new();
    match screen_manager.record_screen(req.display, req.duration_secs).await {
        Ok(result) => Json(RecordingResponse {
            success: result.success,
            data: result.data,
            mime_type: result.mime_type,
            timestamp: result.timestamp,
            error: result.error,
        }),
        Err(e) => Json(RecordingResponse {
            success: false,
            data: None,
            mime_type: "".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            error: Some(e.to_string()),
        }),
    }
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
        .route("/device/camera/capture", post(capture_photo))
        .route("/device/camera/record", post(start_video_recording))
        .route("/device/camera/{id}/capture", post(capture_camera))
        .route("/device/screen/screenshot", post(capture_screenshot))
        .route("/device/screen/record", post(start_screen_recording))
        .route("/device/screen/{id}/capture", post(capture_screen))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
    
    #[test]
    fn test_router_creation() {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        let manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        let _router = create_device_router(manager);
    }
}
