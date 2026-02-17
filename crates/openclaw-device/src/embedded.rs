use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedDeviceConfig {
    pub id: String,
    pub name: String,
    pub device_type: EmbeddedDeviceType,
    pub endpoint: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub sensors: Vec<SensorDef>,
    #[serde(default)]
    pub actuators: Vec<ActuatorDef>,
    #[serde(default)]
    pub commands: Vec<CommandDef>,
}

fn default_timeout() -> u64 {
    5000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddedDeviceType {
    Esp32,
    Esp32S2,
    Esp32S3,
    Esp32C3,
    Esp32C6,
    Stm32F1,
    Stm32F4,
    Stm32H7,
    ArduinoUno,
    ArduinoNano,
    ArduinoMega,
    ArduinoDue,
    RpiPico,
    RpiPicoW,
    Nrf52,
    Generic,
}

impl EmbeddedDeviceType {
    pub fn to_platform_string(&self) -> &'static str {
        match self {
            Self::Esp32 | Self::Esp32S2 | Self::Esp32S3 | Self::Esp32C3 | Self::Esp32C6 => "esp32",
            Self::Stm32F1 | Self::Stm32F4 | Self::Stm32H7 => "stm32",
            Self::ArduinoUno | Self::ArduinoNano | Self::ArduinoMega | Self::ArduinoDue => {
                "arduino"
            }
            Self::RpiPico | Self::RpiPicoW => "rpi_pico",
            Self::Nrf52 => "nrf52",
            Self::Generic => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDef {
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub method: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceState {
    pub sensors: std::collections::HashMap<String, serde_json::Value>,
    pub actuators: std::collections::HashMap<String, serde_json::Value>,
    pub timestamp: i64,
}

pub struct HttpDevice {
    config: EmbeddedDeviceConfig,
    client: reqwest::Client,
}

impl HttpDevice {
    pub fn new(config: EmbeddedDeviceConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.config.endpoint);

        let mut request = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        request
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    pub async fn get_state(&self) -> Result<DeviceState, String> {
        let url = format!("{}/state", self.config.endpoint);

        let mut request = self.client.get(&url);
        if let Some(key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let state: DeviceState = response
            .json()
            .await
            .map_err(|e| format!("Parse failed: {}", e))?;

        Ok(state)
    }

    pub async fn send_command(
        &self,
        command: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (path, method) = self
            .config
            .commands
            .iter()
            .find(|c| c.name == command)
            .map(|c| (c.path.clone(), c.method.clone()))
            .unwrap_or_else(|| ("command".to_string(), "POST".to_string()));

        let full_path = if path.is_empty() {
            format!("command/{}", command)
        } else {
            path
        };

        let url = format!("{}/{}", self.config.endpoint, full_path);

        let mut request = match method.as_str() {
            "GET" => self.client.get(&url),
            _ => self.client.post(&url),
        };

        if let Some(key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        if !params.is_null() {
            request = request.json(&params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Parse failed: {}", e))?;

        Ok(result)
    }

    pub fn config(&self) -> &EmbeddedDeviceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_device_type_to_platform_string() {
        assert_eq!(EmbeddedDeviceType::Esp32.to_platform_string(), "esp32");
        assert_eq!(EmbeddedDeviceType::Stm32F4.to_platform_string(), "stm32");
        assert_eq!(
            EmbeddedDeviceType::ArduinoMega.to_platform_string(),
            "arduino"
        );
        assert_eq!(
            EmbeddedDeviceType::RpiPicoW.to_platform_string(),
            "rpi_pico"
        );
        assert_eq!(EmbeddedDeviceType::Nrf52.to_platform_string(), "nrf52");
    }

    #[test]
    fn test_device_state_default() {
        let state = DeviceState::default();
        assert!(state.sensors.is_empty());
        assert!(state.actuators.is_empty());
    }

    #[test]
    fn test_http_device_config() {
        let config = EmbeddedDeviceConfig {
            id: "test-device".to_string(),
            name: "Test Device".to_string(),
            device_type: EmbeddedDeviceType::Esp32,
            endpoint: "http://192.168.1.100:80".to_string(),
            api_key: Some("test-key".to_string()),
            timeout_ms: 5000,
            sensors: vec![SensorDef {
                id: "temp".to_string(),
                name: "Temperature".to_string(),
                unit: Some("℃".to_string()),
                path: "temperature".to_string(),
            }],
            actuators: vec![],
            commands: vec![CommandDef {
                name: "led_on".to_string(),
                path: "led".to_string(),
                method: "POST".to_string(),
            }],
        };

        let device = HttpDevice::new(config.clone());
        assert_eq!(device.config().id, "test-device");
    }

    #[test]
    fn test_serialize_deserialize() {
        let config = EmbeddedDeviceConfig {
            id: "test".to_string(),
            name: "Test".to_string(),
            device_type: EmbeddedDeviceType::Esp32,
            endpoint: "http://localhost".to_string(),
            api_key: None,
            timeout_ms: 5000,
            sensors: vec![],
            actuators: vec![],
            commands: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: EmbeddedDeviceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "test");
    }
}
