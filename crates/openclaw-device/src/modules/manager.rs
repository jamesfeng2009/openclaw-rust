//! 模块管理器
//!
//! 统一管理 HAL 硬件抽象层和框架集成层模块
//! 根据设备能力自动发现和初始化可用模块

use crate::capabilities::{DeviceCapabilities, PeripheralType};
use crate::framework::{FrameworkConfig, FrameworkModule, FrameworkResult};
use crate::hal::{HalConfig, HalModule, HalResult};
use crate::platform::Platform;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum ModuleType {
    Hal(HalModuleType),
    Framework(FrameworkModuleType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HalModuleType {
    Gpio,
    I2c,
    Spi,
    Serial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkModuleType {
    Ros2,
    Mqtt,
    Can,
}

impl ModuleType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Hal(t) => match t {
                HalModuleType::Gpio => "gpio",
                HalModuleType::I2c => "i2c",
                HalModuleType::Spi => "spi",
                HalModuleType::Serial => "serial",
            },
            Self::Framework(t) => match t {
                FrameworkModuleType::Ros2 => "ros2",
                FrameworkModuleType::Mqtt => "mqtt",
                FrameworkModuleType::Can => "can",
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModuleInfo {
    pub name: String,
    pub module_type: ModuleType,
    pub platform: Platform,
    pub is_initialized: bool,
    pub is_available: bool,
}

pub struct ModuleRegistry {
    hal_modules: HashMap<HalModuleType, Arc<dyn HalModule>>,
    framework_modules: HashMap<FrameworkModuleType, Arc<dyn FrameworkModule>>,
    initialized: RwLock<HashMap<String, bool>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            hal_modules: HashMap::new(),
            framework_modules: HashMap::new(),
            initialized: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_hal(&mut self, module: Arc<dyn HalModule>) {
        let name = module.name().to_string();
        let hal_type = match name.as_str() {
            "gpio" => HalModuleType::Gpio,
            "i2c" => HalModuleType::I2c,
            "spi" => HalModuleType::Spi,
            "serial" => HalModuleType::Serial,
            _ => return,
        };
        self.hal_modules.insert(hal_type, module);
    }

    pub fn register_framework(&mut self, module: Arc<dyn FrameworkModule>) {
        let name = module.name().to_string();
        let fw_type = match name.as_str() {
            "ros2" => FrameworkModuleType::Ros2,
            "mqtt" => FrameworkModuleType::Mqtt,
            "can" => FrameworkModuleType::Can,
            _ => return,
        };
        self.framework_modules.insert(fw_type, module);
    }

    pub async fn init_hal(&self, hal_type: HalModuleType, config: &HalConfig) -> HalResult<()> {
        if let Some(module) = self.hal_modules.get(&hal_type) {
            module.as_ref().init(config).await
        } else {
            Err(crate::hal::HalError::PlatformNotSupported(format!(
                "HAL module not registered: {:?}",
                hal_type
            )))
        }
    }

    pub async fn init_framework(
        &self,
        fw_type: FrameworkModuleType,
        config: &FrameworkConfig,
    ) -> FrameworkResult<()> {
        if let Some(module) = self.framework_modules.get(&fw_type) {
            module.as_ref().connect(config).await
        } else {
            Err(crate::framework::FrameworkError::ConnectionFailed(format!(
                "Framework module not registered: {:?}",
                fw_type
            )))
        }
    }

    pub fn get_hal(&self, hal_type: HalModuleType) -> Option<Arc<dyn HalModule>> {
        self.hal_modules.get(&hal_type).cloned()
    }

    pub fn get_framework(&self, fw_type: FrameworkModuleType) -> Option<Arc<dyn FrameworkModule>> {
        self.framework_modules.get(&fw_type).cloned()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModuleManager {
    registry: ModuleRegistry,
    platform: Platform,
    capabilities: DeviceCapabilities,
}

impl ModuleManager {
    pub fn new(platform: Platform, capabilities: DeviceCapabilities) -> Self {
        Self {
            registry: ModuleRegistry::new(),
            platform,
            capabilities,
        }
    }

    pub fn with_registry(mut self, registry: ModuleRegistry) -> Self {
        self.registry = registry;
        self
    }

    pub fn platform(&self) -> Platform {
        self.platform
    }

    pub fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }

    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut ModuleRegistry {
        &mut self.registry
    }

    pub fn detect_available_modules(&self) -> Vec<ModuleInfo> {
        let mut modules = Vec::new();

        if self
            .capabilities
            .peripherals
            .contains(&PeripheralType::Gpio)
        {
            modules.push(ModuleInfo {
                name: "gpio".to_string(),
                module_type: ModuleType::Hal(HalModuleType::Gpio),
                platform: self.platform,
                is_initialized: false,
                is_available: true,
            });
        }

        if self.capabilities.peripherals.contains(&PeripheralType::I2c) {
            modules.push(ModuleInfo {
                name: "i2c".to_string(),
                module_type: ModuleType::Hal(HalModuleType::I2c),
                platform: self.platform,
                is_initialized: false,
                is_available: true,
            });
        }

        if self.capabilities.peripherals.contains(&PeripheralType::Spi) {
            modules.push(ModuleInfo {
                name: "spi".to_string(),
                module_type: ModuleType::Hal(HalModuleType::Spi),
                platform: self.platform,
                is_initialized: false,
                is_available: true,
            });
        }

        if self
            .capabilities
            .peripherals
            .contains(&PeripheralType::Uart)
        {
            modules.push(ModuleInfo {
                name: "serial".to_string(),
                module_type: ModuleType::Hal(HalModuleType::Serial),
                platform: self.platform,
                is_initialized: false,
                is_available: true,
            });
        }

        if self.capabilities.peripherals.contains(&PeripheralType::Can) {
            modules.push(ModuleInfo {
                name: "can".to_string(),
                module_type: ModuleType::Framework(FrameworkModuleType::Can),
                platform: self.platform,
                is_initialized: false,
                is_available: true,
            });
        }

        modules
    }

    pub fn can_run_ros2(&self) -> bool {
        matches!(
            self.platform,
            Platform::NvidiaJetsonNano
                | Platform::NvidiaJetsonXavier
                | Platform::NvidiaJetsonOrin
                | Platform::NvidiaJetsonOrinNano
                | Platform::RaspberryPi3
                | Platform::RaspberryPi4
                | Platform::RaspberryPi5
                | Platform::RockchipRk3588
                | Platform::LinuxServer
        ) && self.capabilities.memory.total_bytes >= 2 * 1024 * 1024 * 1024
    }

    pub fn can_run_mqtt(&self) -> bool {
        self.capabilities.network.has_ethernet || self.capabilities.network.has_wifi
    }

    pub async fn auto_init(&mut self, config: &ModuleConfig) -> Result<(), ModuleError> {
        let available = self.detect_available_modules();

        for module in available {
            if config.is_enabled(&module.name) {
                match module.module_type {
                    ModuleType::Hal(hal_type) => {
                        let hal_config = HalConfig {
                            enabled: true,
                            platform: Some(self.platform),
                            config: config.get_module_config(&module.name),
                        };
                        if let Err(e) = self.registry.init_hal(hal_type, &hal_config).await {
                            tracing::warn!("Failed to init HAL {}: {}", module.name, e);
                        }
                    }
                    ModuleType::Framework(fw_type) => {
                        let fw_config = FrameworkConfig {
                            enabled: true,
                            endpoint: None,
                            config: config.get_module_config(&module.name),
                        };
                        if let Err(e) = self.registry.init_framework(fw_type, &fw_config).await {
                            tracing::warn!("Failed to init framework {}: {}", module.name, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModuleConfig {
    modules: HashMap<String, ModuleModuleConfig>,
}

#[derive(Debug, Clone)]
struct ModuleModuleConfig {
    enabled: bool,
    config: std::collections::HashMap<String, String>,
}

impl ModuleConfig {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn enable_module(mut self, name: &str) -> Self {
        self.modules.insert(
            name.to_string(),
            ModuleModuleConfig {
                enabled: true,
                config: HashMap::new(),
            },
        );
        self
    }

    pub fn with_config(mut self, name: &str, key: &str, value: &str) -> Self {
        if let Some(module) = self.modules.get_mut(name) {
            module.config.insert(key.to_string(), value.to_string());
        }
        self
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.modules.get(name).map(|m| m.enabled).unwrap_or(false)
    }

    pub fn get_module_config(&self, name: &str) -> std::collections::HashMap<String, String> {
        self.modules
            .get(name)
            .map(|m| m.config.clone())
            .unwrap_or_default()
    }
}

impl Default for ModuleConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ModuleError {
    #[error("Module not found: {0}")]
    NotFound(String),

    #[error("Module already initialized: {0}")]
    AlreadyInitialized(String),

    #[error("Initialization failed: {0}")]
    InitFailed(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

pub type ModuleResult<T> = Result<T, ModuleError>;
