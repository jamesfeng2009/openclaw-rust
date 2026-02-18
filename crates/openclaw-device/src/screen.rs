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

fn path_to_string(path: &PathBuf) -> Result<String, DeviceError> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            DeviceError::OperationFailed("路径包含无效 Unicode 字符".to_string())
        })
}

pub struct ScreenManager;

impl ScreenManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn screenshot(&self, display_id: Option<u32>) -> Result<CaptureResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
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

        #[cfg(target_os = "linux")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let output_path = output_dir.join(format!("screen_{}.png", timestamp));

            let screenshot_cmd = if Command::new("gnome-screenshot").arg("--version").output().is_ok() {
                "gnome-screenshot"
            } else if Command::new("scrot").arg("--version").output().is_ok() {
                "scrot"
            } else if Command::new("import").arg("-version").output().is_ok() {
                "import"
            } else {
                return Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "image/png".to_string(),
                    timestamp,
                    error: Some("无可用截图工具，请安装 gnome-screenshot, scrot 或 imagemagick".to_string()),
                });
            };

            let path_str = path_to_str(&output_path)?;

            let result = if screenshot_cmd == "import" {
                Command::new("import")
                    .arg("-window")
                    .arg("root")
                    .arg(path_str)
                    .output()
            } else if screenshot_cmd == "gnome-screenshot" {
                Command::new("gnome-screenshot")
                    .arg("-f")
                    .arg(path_str)
                    .output()
            } else {
                Command::new("scrot")
                    .arg(path_str)
                    .output()
            };

            match result {
                Ok(output) if output.status.success() => {
                    if output_path.exists() {
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
                        Ok(CaptureResult {
                            success: false,
                            data: None,
                            mime_type: "image/png".to_string(),
                            timestamp,
                            error: Some("截图文件未生成".to_string()),
                        })
                    }
                }
                Ok(output) => {
                    let error = String::from_utf8_lossy(&output.stderr).to_string();
                    Ok(CaptureResult {
                        success: false,
                        data: None,
                        mime_type: "image/png".to_string(),
                        timestamp,
                        error: Some(error),
                    })
                }
                Err(e) => Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "image/png".to_string(),
                    timestamp,
                    error: Some(e.to_string()),
                }),
            }
        }

        #[cfg(target_os = "windows")]
        {
            let output_dir = PathBuf::from(std::env::temp_dir()).join("openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let output_path = output_dir.join(format!("screen_{}.png", timestamp));
            let path_str = path_to_string(&output_path)?;

            let ps_script = format!(
                r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Screen]::PrimaryScreen.Bounds | ForEach-Object {{ $bmp = New-Object System.Drawing.Bitmap($_.Width, $_.Height); $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($_.Location, [System.Drawing.Point]::Empty, $_.Size); $bmp.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose() }}"#,
                path_str.replace('\\', "\\\\")
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

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
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
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let display = display_id.unwrap_or(0);
            let output_path = output_dir.join(format!("recording_{}.mov", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output_path_clone = output_path.clone();
            let path_str = path_to_string(&output_path_clone)?;
            let display_clone = display;

            let result = tokio::task::spawn_blocking(move || {
                let output = Command::new("screencapture")
                    .args([
                        "-v",
                        "-D",
                        &display_clone.to_string(),
                        "-t",
                        &duration.to_string(),
                        &path_str,
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

        #[cfg(target_os = "linux")]
        {
            let output_dir = PathBuf::from("/tmp/openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let output_path = output_dir.join(format!("recording_{}.mp4", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output_path_clone = output_path.clone();
            let path_str = path_to_string(&output_path_clone)?;

            let result = tokio::task::spawn_blocking(move || {
                let output = Command::new("ffmpeg")
                    .args([
                        "-f", "x11grab",
                        "-framerate", "30",
                        "-video_size", "1920x1080",
                        "-i", ":0.0",
                        "-t", &duration.to_string(),
                        "-c:v", "libx264",
                        "-preset", "ultrafast",
                        &path_str,
                    ])
                    .output()
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

                Ok::<_, DeviceError>(output)
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
            let output_dir = PathBuf::from(std::env::temp_dir()).join("openclaw");
            fs::create_dir_all(&output_dir)
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            let output_path = output_dir.join(format!("recording_{}.mp4", timestamp));
            let duration = duration_secs.unwrap_or(10);

            let output_path_clone = output_path.clone();
            let path_str = path_to_string(&output_path_clone)?;

            let result = tokio::task::spawn_blocking(move || {
                let ps_script = format!(
                    r#"Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $duration = {}; $fps = 30; $width = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Width; $height = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds.Height; $bmp = New-Object System.Drawing.Bitmap($width, $height); $g = [System.Drawing.Graphics]::FromImage($bmp); $frames = @(); for($i = 0; $i -lt $duration * $fps; $i++) {{ $g.CopyFromScreen([System.Drawing.Point]::Zero, [System.Drawing.Point]::Zero, [System.Drawing.Size]::new($width, $height)); $frames += $bmp.Clone(); Start-Sleep -Milliseconds (1000 / $fps) }}; $g.Dispose(); $bmp.Dispose(); $frames[0].Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); $frames | ForEach-Object {{ $_.Dispose() }}"#,
                    duration,
                    path_str.replace('\\', "\\\\")
                );
                Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_script])
                    .output()
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))
            })
            .await
            .map_err(|e| DeviceError::Internal(anyhow::anyhow!(e.to_string())))??;

            if output_path.exists() {
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
                Ok(CaptureResult {
                    success: false,
                    data: None,
                    mime_type: "video/mp4".to_string(),
                    timestamp,
                    error: Some("录制功能需要 ffmpeg".to_string()),
                })
            }
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
