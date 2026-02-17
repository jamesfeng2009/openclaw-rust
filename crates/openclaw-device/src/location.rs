use crate::nodes::{DeviceError, LocationResult};
use chrono::Utc;
use std::process::Command;

pub struct LocationManager;

impl LocationManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_location(&self) -> Result<LocationResult, DeviceError> {
        let timestamp = Utc::now().timestamp_millis();

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("coreutilelocation")
                .arg("OI")
                .output()
                .or_else(|_| {
                    Command::new("defaults")
                        .args(["read", "/var/db/locationd/clients.plist"])
                        .output()
                });

            if let Ok(output) = output {
                if output.status.success() {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if let (Some(lat), Some(lon)) =
                        (self.parse_lat(&output_str), self.parse_lon(&output_str))
                    {
                        return Ok(LocationResult {
                            success: true,
                            latitude: Some(lat),
                            longitude: Some(lon),
                            altitude: None,
                            accuracy: Some(10.0),
                            timestamp,
                            error: None,
                        });
                    }
                }
            }

            Ok(LocationResult {
                success: false,
                latitude: None,
                longitude: None,
                altitude: None,
                accuracy: None,
                timestamp,
                error: Some("无法获取位置信息，请确保位置服务已启用".to_string()),
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(LocationResult {
                success: false,
                latitude: None,
                longitude: None,
                altitude: None,
                accuracy: None,
                timestamp,
                error: Some("不支持的平台".to_string()),
            })
        }
    }

    fn parse_lat(&self, output: &str) -> Option<f64> {
        output
            .lines()
            .find(|line| line.contains("latitude") || line.contains("lat"))
            .and_then(|line| line.split(':').nth(1)?.trim().parse::<f64>().ok())
    }

    fn parse_lon(&self, output: &str) -> Option<f64> {
        output
            .lines()
            .find(|line| line.contains("longitude") || line.contains("lon"))
            .and_then(|line| line.split(':').nth(1)?.trim().parse::<f64>().ok())
    }

    pub fn is_available(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            Command::new("defaults")
                .args(["read", "com.apple.coreLocation", "LocationEnabled"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        }

        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }
}

impl Default for LocationManager {
    fn default() -> Self {
        Self::new()
    }
}
