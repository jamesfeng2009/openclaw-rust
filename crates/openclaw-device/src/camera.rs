use crate::nodes::{CaptureResult, DeviceError};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn path_to_str(path: &PathBuf) -> Result<&str, DeviceError> {
    path.to_str().ok_or_else(|| {
        DeviceError::OperationFailed("路径包含无效 Unicode 字符".to_string())
    })
}

pub struct CameraManager;

impl CameraManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn capture_photo(
        &self,
        device_index: Option<u32>,
    ) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("photo_{}.jpg", timestamp));

            let output = Command::new("imagesnap")
                .args([
                    "-w",
                    "5",
                    "-d",
                    &device.to_string(),
                    path_to_str(&output_path)?,
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

        #[cfg(target_os = "linux")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("photo_{}.jpg", timestamp));

            let device_path = format!("/dev/video{}", device);

            let output = Command::new("fswebcam")
                .args([
                    "-r", "1280x720",
                    "--no-banner",
                    "-d", &device_path,
                    path_to_str(&output_path)?,
                ])
                .output();

            match output {
                Ok(result) if result.status.success() => {
                    if output_path.exists() {
                        let data = fs::read(&output_path)
                            .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                        let base64_data =
                            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                        fs::remove_file(&output_path).ok();

                        Ok(CaptureResult {
                            success: true,
                            data: Some(base64_data),
                            mime_type: "image/jpeg".to_string(),
                            timestamp,
                            error: None,
                        })
                    } else {
                        Ok(CaptureResult {
                            success: false,
                            data: None,
                            mime_type: "image/jpeg".to_string(),
                            timestamp,
                            error: Some("摄像头文件未生成".to_string()),
                        })
                    }
                }
                Ok(result) => {
                    let error = String::from_utf8_lossy(&result.stderr).to_string();
                    Ok(CaptureResult {
                        success: false,
                        data: None,
                        mime_type: "image/jpeg".to_string(),
                        timestamp,
                        error: Some(error),
                    })
                }
                Err(e) => Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "image/jpeg".to_string(),
                    timestamp,
                    error: Some(format!("请安装 fswebcam: {}", e)),
                }),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output_dir = PathBuf::from(std::env::temp_dir()).join("openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let output_path = output_dir.join(format!("photo_{}.jpg", timestamp));

            let ps_script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $capture = New-Object System.Windows.Forms.WebCamCapture; $capture.Start(); Start-Sleep -Seconds 2; $capture.Stop(); $bitmap = $capture.GetCurrentFrame(); $bitmap.Save('{}', [System.Drawing.Imaging.ImageFormat]::Jpeg); $bitmap.Dispose()"#,
                path_to_str(&output_path)?.replace('\\', "\\\\")
            );

            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_script])
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            if output.status.success() && output_path.exists() {
                let data = fs::read(&output_path)
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                let base64_data =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
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
                    error: Some(format!("Windows 摄像头需要额外库支持: {}", error)),
                })
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
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

    pub async fn start_recording(
        &self,
        device_index: Option<u32>,
        duration_secs: Option<u32>,
    ) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("video_{}.mov", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output = Command::new("imagesnap")
                .args([
                    "-v",
                    "-d",
                    &device.to_string(),
                    "-t",
                    &duration.to_string(),
                    path_to_str(&output_path)?,
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

        #[cfg(target_os = "linux")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let device = device_index.unwrap_or(0);
            let output_path = output_dir.join(format!("video_{}.mp4", timestamp));
            let duration = duration_secs.unwrap_or(10);
            let device_path = format!("/dev/video{}", device);

            let output_path_clone = output_path.clone();
            let path_str = path_to_str(&output_path_clone)?;

            let result = tokio::task::spawn_blocking(move || {
                Command::new("ffmpeg")
                    .args([
                        "-f", "v4l2",
                        "-framerate", "30",
                        "-video_size", "1280x720",
                        "-i", &device_path,
                        "-t", &duration.to_string(),
                        "-c:v", "libx264",
                        "-preset", "ultrafast",
                        path_str,
                    ])
                    .output()
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))
            })
            .await
            .map_err(|e| DeviceError::Internal(anyhow::anyhow!(e.to_string())))??;

            if result.status.success() && output_path.exists() {
                let data = fs::read(&output_path)
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;
                let base64_data =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data);
                fs::remove_file(&output_path).ok();

                Ok(CaptureResult {
                    success: true,
                    data: Some(base64_data),
                    mime_type: "video/mp4".to_string(),
                    timestamp,
                    error: None,
                })
            } else {
                let error = String::from_utf8_lossy(&result.stderr).to_string();
                Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "video/mp4".to_string(),
                    timestamp,
                    error: Some(error),
                })
            }
        }

        #[cfg(target_os = "windows")]
        {
            Ok(CaptureResult {
                success: false,
                data: None,
                mime_type: "video/mp4".to_string(),
                timestamp,
                error: Some("Windows 平台录像功能开发中".to_string()),
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
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
            if let Ok(output) = Command::new("system_profiler")
                .args(["SPCameraDataType"])
                .output()
            {
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

        #[cfg(target_os = "linux")]
        {
            let mut cameras = Vec::new();
            if let Ok(entries) = fs::read_dir("/dev") {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("video") {
                        cameras.push(name);
                    }
                }
            }
            if !cameras.is_empty() {
                return cameras;
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-WmiObject Win32_PnPEntity | Where-Object {$_.Name -match 'Camera|Webcam'} | Select-Object -ExpandProperty Name"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let cameras: Vec<String> = output_str
                    .lines()
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect();
                if !cameras.is_empty() {
                    return cameras;
                }
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
