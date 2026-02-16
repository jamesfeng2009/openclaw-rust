use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::device_trait::{Device, DeviceConfig, DeviceCommand, DeviceResponse, DeviceResult, DeviceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Custom,
}

impl EmbeddedDeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Esp32 => "esp32",
            Self::Esp32S2 => "esp32s2",
            Self::Esp32S3 => "esp32s3",
            Self::Esp32C3 => "esp32c3",
            Self::Esp32C6 => "esp32c6",
            Self::Stm32F1 => "stm32f1",
            Self::Stm32F4 => "stm32f4",
            Self::Stm32H7 => "stm32h7",
            Self::ArduinoUno => "arduino_uno",
            Self::ArduinoNano => "arduino_nano",
            Self::ArduinoMega => "arduino_mega",
            Self::ArduinoDue => "arduino_due",
            Self::RpiPico => "rpi_pico",
            Self::RpiPicoW => "rpi_pico_w",
            Self::Nrf52 => "nrf52",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    pub device_type: EmbeddedDeviceType,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    pub sensors: Vec<SensorDefinition>,
    pub actuators: Vec<ActuatorDefinition>,
    pub commands: Vec<CommandDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDefinition {
    pub id: String,
    pub name: String,
    pub unit: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorDefinition {
    pub id: String,
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub params: Vec<CommandParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmbeddedState {
    pub sensors: std::collections::HashMap<String, f64>,
    pub timestamp: i64,
}

pub struct EmbeddedDevice {
    id: String,
    name: String,
    config: EmbeddedConfig,
    state: Arc<RwLock<EmbeddedState>>,
    client: reqwest::Client,
}

impl EmbeddedDevice {
    pub fn new(id: String, name: String, config: EmbeddedConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_default();

        Self {
            id,
            name,
            config,
            state: Arc::new(RwLock::new(EmbeddedState::default())),
            client,
        }
    }

    pub async fn fetch_state(&self) -> DeviceResult<EmbeddedState> {
        let url = format!("{}/state", self.config.endpoint);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await
            .map_err(|e| DeviceError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DeviceError::ExecutionFailed(format!("HTTP error: {}", response.status())));
        }

        let state: EmbeddedState = response.json().await
            .map_err(|e| DeviceError::ExecutionFailed(format!("Parse response failed: {}", e)))?;

        let mut current_state = self.state.write().await;
        *current_state = state.clone();

        Ok(state)
    }

    pub async fn send_command(&self, command: &str, params: std::collections::HashMap<String, serde_json::Value>) -> DeviceResult<serde_json::Value> {
        let url = format!("{}/command/{}", self.config.endpoint, command);
        
        let mut request = self.client.post(&url);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        request = request.json(&params);

        let response = request.send().await
            .map_err(|e| DeviceError::ExecutionFailed(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(DeviceError::ExecutionFailed(format!("HTTP error: {}", response.status())));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| DeviceError::ExecutionFailed(format!("Parse response failed: {}", e)))?;

        Ok(result)
    }

    pub async fn get_sensor_value(&self, sensor_id: &str) -> DeviceResult<f64> {
        let state = self.state.read().await;
        state.sensors
            .get(sensor_id)
            .copied()
            .ok_or_else(|| DeviceError::NotFound(format!("Sensor not found: {}", sensor_id)))
    }

    pub fn device_type(&self) -> EmbeddedDeviceType {
        self.config.device_type
    }
}

#[async_trait]
impl Device for EmbeddedDevice {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn platform(&self) -> crate::Platform {
        match self.config.device_type {
            EmbeddedDeviceType::Esp32 | EmbeddedDeviceType::Esp32S2 | 
            EmbeddedDeviceType::Esp32S3 | EmbeddedDeviceType::Esp32C3 |
            EmbeddedDeviceType::Esp32C6 => crate::Platform::Esp32,
            EmbeddedDeviceType::Stm32F1 | EmbeddedDeviceType::Stm32F4 | 
            EmbeddedDeviceType::Stm32H7 => crate::Platform::Stm32F4,
            EmbeddedDeviceType::ArduinoUno | EmbeddedDeviceType::ArduinoNano |
            EmbeddedDeviceType::ArduinoMega | EmbeddedDeviceType::ArduinoDue => crate::Platform::ArduinoUno,
            EmbeddedDeviceType::RpiPico | EmbeddedDeviceType::RpiPicoW => crate::Platform::RpiPico,
            EmbeddedDeviceType::Nrf52 => crate::Platform::Nrf52,
            EmbeddedDeviceType::Custom => crate::Platform::Unknown,
        }
    }

    fn category(&self) -> crate::ComputeCategory {
        crate::ComputeCategory::Embedded
    }

    fn capabilities(&self) -> &crate::DeviceCapabilities {
        static CAPS: std::sync::OnceLock<crate::DeviceCapabilities> = std::sync::OnceLock::new();
        CAPS.get_or_init(|| {
            let mut caps = crate::DeviceCapabilities::default();
            caps.cpu.cores = 1;
            caps.cpu.threads = 1;
            caps.memory.total_bytes = 512 * 1024 * 1024;
            caps.sensors = vec![
                crate::capabilities::SensorType::Temperature,
                crate::capabilities::SensorType::Humidity,
            ];
            caps.peripherals = vec![
                crate::capabilities::PeripheralType::Gpio,
                crate::capabilities::PeripheralType::Uart,
            ];
            caps
        })
    }

    async fn init(&self, _config: &DeviceConfig) -> DeviceResult<()> {
        self.fetch_state().await?;
        Ok(())
    }

    async fn health_check(&self) -> DeviceResult<bool> {
        let url = format!("{}/health", self.config.endpoint);
        
        let mut request = self.client.get(&url);
        
        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        match request.send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    async fn execute(&self, command: &DeviceCommand) -> DeviceResult<DeviceResponse> {
        match command.action.as_str() {
            "fetch_state" => {
                let state = self.fetch_state().await?;
                Ok(DeviceResponse {
                    success: true,
                    data: Some(serde_json::to_value(state).map_err(|e| DeviceError::Internal(anyhow::anyhow!(e)))?),
                    error: None,
                })
            }
            "get_sensor" => {
                let sensor_id = command.params.get("sensor_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DeviceError::Config("Missing sensor_id".to_string()))?;
                
                let value = self.get_sensor_value(sensor_id).await?;
                
                Ok(DeviceResponse {
                    success: true,
                    data: Some(serde_json::json!({ "sensor_id": sensor_id, "value": value })),
                    error: None,
                })
            }
            "command" => {
                let cmd_name = command.params.get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DeviceError::Config("Missing command name".to_string()))?;
                
                let mut params = std::collections::HashMap::new();
                if let Some(params_obj) = command.params.get("params") {
                    if let Some(obj) = params_obj.as_object() {
                        for (k, v) in obj {
                            params.insert(k.clone(), v.clone());
                        }
                    }
                }
                
                let result = self.send_command(cmd_name, params).await?;
                
                Ok(DeviceResponse {
                    success: true,
                    data: Some(result),
                    error: None,
                })
            }
            _ => Err(DeviceError::Unsupported(format!("Action not supported: {}", command.action))),
        }
    }
}

pub struct EmbeddedDeviceBuilder {
    id: String,
    name: String,
    config: Option<EmbeddedConfig>,
}

impl EmbeddedDeviceBuilder {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            config: None,
        }
    }

    pub fn config(mut self, config: EmbeddedConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn esp32(mut self, endpoint: impl Into<String>) -> Self {
        self.config = Some(EmbeddedConfig {
            device_type: EmbeddedDeviceType::Esp32,
            endpoint: endpoint.into(),
            api_key: None,
            timeout_ms: 5000,
            sensors: vec![],
            actuators: vec![],
            commands: vec![],
        });
        self
    }

    pub fn stm32(mut self, endpoint: impl Into<String>) -> Self {
        self.config = Some(EmbeddedConfig {
            device_type: EmbeddedDeviceType::Stm32F4,
            endpoint: endpoint.into(),
            api_key: None,
            timeout_ms: 3000,
            sensors: vec![],
            actuators: vec![],
            commands: vec![],
        });
        self
    }

    pub fn arduino(mut self, endpoint: impl Into<String>) -> Self {
        self.config = Some(EmbeddedConfig {
            device_type: EmbeddedDeviceType::ArduinoMega,
            endpoint: endpoint.into(),
            api_key: None,
            timeout_ms: 5000,
            sensors: vec![],
            actuators: vec![],
            commands: vec![],
        });
        self
    }

    pub fn build(self) -> EmbeddedDevice {
        EmbeddedDevice::new(
            self.id,
            self.name,
            self.config.unwrap_or_else(|| EmbeddedConfig {
                device_type: EmbeddedDeviceType::Custom,
                endpoint: "http://localhost:8080".to_string(),
                api_key: None,
                timeout_ms: 5000,
                sensors: vec![],
                actuators: vec![],
                commands: vec![],
            }),
        )
    }
}
