use crate::nodes::{CaptureResult, DeviceError};
use chrono::Utc;
use std::fs;
use std::process::Command;

pub struct ScreenManager;

impl ScreenManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn screenshot(&self, display_id: Option<u32>) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
            use std::path::PathBuf;

            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let display = display_id.unwrap_or(0);
            let output_path = output_dir.join(format!("screen_{}.png", timestamp));

            let output = Command::new("screencapture")
                .args([
                    "-x",
                    "-D",
                    &display.to_string(),
                    output_path.to_str().unwrap(),
                ])
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            if output.status.success() {
                let data = fs::read(&output_path)
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                let base64_data =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

                fs::remove_file(&output_path).ok();

                Ok(CaptureResult {
                    success: true,
                    data: Some(base64_data),
                    mime_type: "image/png".to_string(),
                    timestamp,
                    error: None,
                })
            } else {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "image/png".to_string(),
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
                mime_type: "image/png".to_string(),
                timestamp,
                error: Some("不支持的平台".to_string()),
            })
        }
    }

    pub async fn record_screen(
        &self,
        display_id: Option<u32>,
        duration_secs: Option<u32>,
    ) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
            use std::path::PathBuf;
            
            

            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let display = display_id.unwrap_or(0);
            let output_path = output_dir.join(format!("recording_{}.mov", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output_path_clone = output_path.clone();
            let display_clone = display;

            let result = tokio::task::spawn_blocking(move || {
                let output = Command::new("screencapture")
                    .args([
                        "-v",
                        "-D",
                        &display_clone.to_string(),
                        "-t",
                        &duration.to_string(),
                        output_path_clone.to_str().unwrap(),
                    ])
                    .output()
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

                Ok::<_, DeviceError>(output)
            })
            .await
            .map_err(|e| DeviceError::Internal(anyhow::anyhow!(e.to_string())))??;

            if result.status.success() {
                if output_path.exists() {
                    let data = fs::read(&output_path)
                        .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                    let base64_data =
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);

                    fs::remove_file(&output_path).ok();

                    Ok(CaptureResult {
                        success: true,
                        data: Some(base64_data),
                        mime_type: "video/quicktime".to_string(),
                        timestamp,
                        error: None,
                    })
                } else {
                    Ok(CaptureResult {
                        success: false,
                        data: None,
                        mime_type: "video/quicktime".to_string(),
                        timestamp,
                        error: Some("录制文件未生成".to_string()),
                    })
                }
            } else {
                let error = String::from_utf8_lossy(&result.stderr).to_string();
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

    pub fn list_displays(&self) -> Vec<(u32, String)> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("system_profiler")
                .args(["SPDisplaysDataType"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let mut displays = Vec::new();
                let mut display_id = 0u32;

                for line in output_str.lines() {
                    if line.contains("Display") {
                        display_id += 1;
                        let name = line.trim().to_string();
                        if !name.is_empty() {
                            displays.push((display_id, name));
                        }
                    }
                }

                if displays.is_empty() {
                    displays.push((0, "主显示器".to_string()));
                }
                return displays;
            }
        }
        vec![(0, "主显示器".to_string())]
    }
}

impl Default for ScreenManager {
    fn default() -> Self {
        Self::new()
    }
}
