use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevicesConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub compute_categories: Vec<ComputeCategoryConfig>,
    #[serde(default)]
    pub platforms: Vec<PlatformConfig>,
    #[serde(default)]
    pub nodes: Vec<NodeConfig>,
    #[serde(default)]
    pub adapters: Vec<AdapterConfig>,
    #[serde(default)]
    pub custom_devices: Vec<CustomDeviceConfig>,
    #[serde(default)]
    pub plugins: Vec<PluginConfig>,
    #[serde(default)]
    pub embedded_devices: Vec<EmbeddedDeviceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomDeviceConfig {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub category: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub capabilities: Option<DeviceCapabilitiesConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceCapabilitiesConfig {
    #[serde(default)]
    pub min_cpu_cores: Option<u32>,
    #[serde(default)]
    pub min_memory_mb: Option<u32>,
    pub has_gpu: bool,
    pub has_npu: bool,
    #[serde(default)]
    pub has_wifi: bool,
    #[serde(default)]
    pub has_ethernet: bool,
    #[serde(default)]
    pub has_ble: bool,
    #[serde(default)]
    pub has_cellular: bool,
    #[serde(default)]
    pub peripherals: Vec<String>,
    #[serde(default)]
    pub sensors: Vec<String>,
    #[serde(default)]
    pub network_protocols: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub enabled: bool,
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCategoryConfig {
    pub category: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub platform: String,
    pub enabled: bool,
    #[serde(default)]
    pub capabilities: PlatformCapabilities,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlatformCapabilities {
    #[serde(default)]
    pub min_cpu_cores: Option<u32>,
    #[serde(default)]
    pub min_memory_mb: Option<u32>,
    #[serde(default)]
    pub has_gpu: bool,
    #[serde(default)]
    pub has_npu: bool,
    #[serde(default)]
    pub supported_peripherals: Vec<String>,
    #[serde(default)]
    pub supported_sensors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_type: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: String,
    pub enabled: bool,
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedDeviceConfig {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub endpoint: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub sensors: Vec<SensorDef>,
    #[serde(default)]
    pub actuators: Vec<ActuatorDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDef {
    pub id: String,
    pub name: String,
    pub sensor_type: String,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorDef {
    pub id: String,
    pub name: String,
    pub actuator_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devices_config_default() {
        let config = DevicesConfig::default();
        assert!(!config.enabled);
        assert!(config.custom_devices.is_empty());
        assert!(config.embedded_devices.is_empty());
    }

    #[test]
    fn test_custom_device_config() {
        let device = CustomDeviceConfig {
            id: "esp32-001".to_string(),
            name: "ESP32 Controller".to_string(),
            platform: "esp32".to_string(),
            category: "microcontroller".to_string(),
            enabled: true,
            endpoint: Some("http://192.168.1.100".to_string()),
            api_key: None,
            timeout_ms: Some(5000),
            metadata: HashMap::new(),
            capabilities: None,
        };
        assert_eq!(device.id, "esp32-001");
        assert!(device.enabled);
    }

    #[test]
    fn test_embedded_device_config() {
        let device = EmbeddedDeviceConfig {
            id: "esp32-test".to_string(),
            name: "Test ESP32".to_string(),
            device_type: "esp32".to_string(),
            endpoint: "http://192.168.1.50".to_string(),
            api_key: None,
            timeout_ms: 30000,
            sensors: vec![SensorDef {
                id: "temp-1".to_string(),
                name: "Temperature".to_string(),
                sensor_type: "temperature".to_string(),
                unit: Some("°C".to_string()),
            }],
            actuators: vec![],
        };
        assert_eq!(device.sensors.len(), 1);
        assert_eq!(device.timeout_ms, 30000);
    }

    #[test]
    fn test_platform_config() {
        let platform = PlatformConfig {
            platform: "esp32".to_string(),
            enabled: true,
            capabilities: PlatformCapabilities {
                min_cpu_cores: Some(1),
                min_memory_mb: Some(512),
                has_gpu: false,
                has_npu: false,
                supported_peripherals: vec!["GPIO".to_string(), "I2C".to_string()],
                supported_sensors: vec!["temperature".to_string()],
            },
            description: Some("ESP32 microcontroller".to_string()),
        };
        assert!(platform.enabled);
        assert_eq!(platform.capabilities.supported_peripherals.len(), 2);
    }

    #[test]
    fn test_devices_config_serialize_deserialize() {
        let config = DevicesConfig {
            enabled: true,
            compute_categories: vec![],
            platforms: vec![],
            nodes: vec![],
            adapters: vec![],
            custom_devices: vec![CustomDeviceConfig {
                id: "device1".to_string(),
                name: "Device 1".to_string(),
                platform: "esp32".to_string(),
                category: "iot".to_string(),
                enabled: true,
                endpoint: None,
                api_key: None,
                timeout_ms: None,
                metadata: HashMap::new(),
                capabilities: None,
            }],
            plugins: vec![],
            embedded_devices: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: DevicesConfig = serde_json::from_str(&json).unwrap();

        assert!(parsed.enabled);
        assert_eq!(parsed.custom_devices.len(), 1);
    }
}
