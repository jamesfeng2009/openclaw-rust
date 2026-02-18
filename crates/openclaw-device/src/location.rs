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

            if let Ok(output) = output
                && output.status.success()
            {
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
            #[cfg(target_os = "linux")]
            {
                if let Ok(output) = Command::new("gpspipe")
                    .args(["-w", "-n", "5"])
                    .output()
                {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    if let Some(lat) = output_str
                        .lines()
                        .find(|l| l.contains("\"lat\""))
                        .and_then(|l| l.split(":").nth(1))
                        .and_then(|s| s.trim().trim_matches(',').parse::<f64>().ok())
                    {
                        if let Some(lon) = output_str
                            .lines()
                            .find(|l| l.contains("\"lon\""))
                            .and_then(|l| l.split(":").nth(1))
                            .and_then(|s| s.trim().trim_matches(',').parse::<f64>().ok())
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

                if let Ok(output) = std::fs::read_to_string("/dev/ttyUSB0") {
                    let lines: Vec<&str> = output.lines().collect();
                    for line in lines {
                        if line.starts_with("$GPGGA") || line.starts_with("$GPRMC") {
                            return Ok(LocationResult {
                                success: true,
                                latitude: None,
                                longitude: None,
                                altitude: None,
                                accuracy: None,
                                timestamp,
                                error: Some("GPS 原始数据解析需要实现 NMEA 解析器".to_string()),
                            });
                        }
                    }
                }
            }

            #[cfg(target_os = "windows")]
            {
                let ps_script = r#"
                    Add-Type -AssemblyName System.Device;
                    $loc = New-Object System.Device.Location.GeoCoordinateWatcher;
                    $loc.Start();
                    Start-Sleep -Seconds 2;
                    if ($loc.Position.Location.IsValid) {
                        Write-Output "$($loc.Position.Location.Latitude),$($loc.Position.Location.Longitude)";
                    }
                "#;

                if let Ok(output) = Command::new("powershell")
                    .args(["-NoProfile", "-Command", ps_script])
                    .output()
                {
                    let output_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let parts: Vec<&str> = output_str.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(lat), Ok(lon)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
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
            }

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
            return Command::new("defaults")
                .args(["read", "com.apple.coreLocation", "LocationEnabled"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        }

        #[cfg(target_os = "linux")]
        {
            return Command::new("gpspipe")
                .args(["-V"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || std::path::Path::new("/dev/ttyUSB0").exists();
        }

        #[cfg(target_os = "windows")]
        {
            return Command::new("powershell")
                .args(["-NoProfile", "-Command", "Add-Type -AssemblyName System.Device; (New-Object System.Device.Location.GeoCoordinateWatcher).Status"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
        }

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
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
