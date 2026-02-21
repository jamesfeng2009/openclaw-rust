//! 设备能力 API 路由

use axum::{
    routing::post,
    Json, Router,
};
use openclaw_device::{CameraManager, ScreenManager};
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

pub fn create_device_router() -> Router {
    Router::new()
        .route("/device/camera/capture", post(capture_photo))
        .route("/device/camera/record", post(start_video_recording))
        .route("/device/screen/screenshot", post(capture_screenshot))
        .route("/device/screen/record", post(start_screen_recording))
}
