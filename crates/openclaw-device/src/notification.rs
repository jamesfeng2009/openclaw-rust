use crate::nodes::{DeviceError, NotificationResult};
use std::process::Command;

pub struct NotificationManager;

impl NotificationManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn send_notification(
        &self,
        title: &str,
        body: &str,
        icon: Option<&str>,
    ) -> Result<NotificationResult, DeviceError> {
        let notification_id = uuid::Uuid::new_v4().to_string();

        #[cfg(target_os = "macos")]
        {
            let _icon_arg = icon.unwrap_or("SF Symbols:bell.fill");

            let output = Command::new("osascript")
                .args([
                    "-e",
                    &format!(
                        r#"display notification "{}" with title "{}" subtitle "" sound name "Glass""#,
                        body, title
                    ),
                ])
                .output()
                .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

            if output.status.success() {
                Ok(NotificationResult {
                    success: true,
                    notification_id: Some(notification_id),
                    error: None,
                })
            } else {
                let error = String::from_utf8_lossy(&output.stderr).to_string();
                Ok(NotificationResult {
                    success: false,
                    notification_id: None,
                    error: Some(error),
                })
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(target_os = "linux")]
            {
                let output = Command::new("notify-send")
                    .args([title, body])
                    .output()
                    .map_err(|e| DeviceError::OperationFailed(e.to_string()))?;

                if output.status.success() {
                    return Ok(NotificationResult {
                        success: true,
                        notification_id: Some(notification_id),
                        error: None,
                    });
                }
            }

            Ok(NotificationResult {
                success: false,
                notification_id: None,
                error: Some("不支持的平台或通知系统".to_string()),
            })
        }
    }

    pub async fn send_system_notification(
        &self,
        title: &str,
        message: &str,
    ) -> Result<NotificationResult, DeviceError> {
        self.send_notification(title, message, None).await
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}
