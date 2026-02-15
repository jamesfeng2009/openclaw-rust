use crate::nodes::{CaptureResult, DeviceError};
use chrono::Utc;
use std::process::Command;

pub struct CameraManager;

impl CameraManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn capture_photo(&self, device_index: Option<u32>) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();
        
        #[cfg(target_os = "macos")]
        {
            use std::fs;
            use std::path::PathBuf;

            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir).map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
            
            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("photo_{}.jpg", timestamp));

            let output = Command::new("imagesnap")
                .args([
                    "-w", "5",
                    "-d", &device.to_string(),
                    output_path.to_str().unwrap(),
                ])
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            if output.status.success() {
                let data = fs::read(&output_path)
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                
                fs::remove_file(&output_path).ok();
                
                Ok(CaptureResult {
                    success: true,
                    data: Some(base64_data),
                    mime_type: "image/jpeg".to_string(),
                    timestamp,
                    error: None,
                })
            } else {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "image/jpeg".to_string(),
                    timestamp,
                    error: Some(error),
                })
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(CaptureResult {
                success: false,
                data: None,
                mime_type: "image/jpeg".to_string(),
                timestamp,
                error: Some("不支持的平台".to_string()),
            })
        }
    }

    pub async fn start_recording(&self, device_index: Option<u32>, duration_secs: Option<u32>) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();
        
        #[cfg(target_os = "macos")]
        {
            use std::fs;
            use std::path::PathBuf;

            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir).map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
            
            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("video_{}.mov", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output = Command::new("imagesnap")
                .args([
                    "-v",
                    "-d", &device.to_string(),
                    "-t", &duration.to_string(),
                    output_path.to_str().unwrap(),
                ])
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            if output.status.success() {
                let data = fs::read(&output_path)
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                let base64_data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                
                fs::remove_file(&output_path).ok();
                
                Ok(CaptureResult {
                    success: true,
                    data: Some(base64_data),
                    mime_type: "video/quicktime".to_string(),
                    timestamp,
                    error: None,
                })
            } else {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "video/quicktime".to_string(),
                    timestamp,
                    error: Some(error),
                })
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(CaptureResult {
                success: false,
                data: None,
                mime_type: "video/quicktime".to_string(),
                timestamp,
                error: Some("不支持的平台".to_string()),
            })
        }
    }

    pub fn list_cameras(&self) -> Vec<String> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("system_profiler").args(["SPCameraDataType"]).output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let cameras: Vec<String> = output_str
                    .lines()
                    .filter(|line| line.contains(":") && !line.contains("SPCamera"))
                    .filter_map(|line| line.split(':').next().map(|s| s.trim().to_string()))
                    .filter(|s| !s.is_empty())
                    .collect();
                return cameras;
            }
        }
        vec!["默认相机".to_string()]
    }
}

impl Default for CameraManager {
    fn default() -> Self {
        Self::new()
    }
}
