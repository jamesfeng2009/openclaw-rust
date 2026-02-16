use std::sync::Arc;
use tokio::sync::RwLock;
use openclaw_core::Config;

pub struct DeviceManager {
    registry: Arc<openclaw_device::DeviceRegistry>,
    config: Arc<Config>,
}

impl DeviceManager {
    pub fn new(config: Config) -> Self {
        let registry = Arc::new(openclaw_device::DeviceRegistry::new());
        Self {
            registry,
            config: Arc::new(config),
        }
    }

    pub async fn init(&self) -> openclaw_core::Result<()> {
        if !self.config.devices.enabled {
            tracing::info!("Devices disabled in config");
            return Ok(());
        }

        self.registry.init().await
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("Device registry init failed: {}", e)))?;

        self.load_custom_devices().await?;
        self.load_plugins().await?;

        tracing::info!("Device manager initialized");
        Ok(())
    }

    async fn load_custom_devices(&self) -> openclaw_core::Result<()> {
        for device_config in &self.config.devices.custom_devices {
            if !device_config.enabled {
                continue;
            }

            let platform = self.parse_platform(&device_config.platform)?;
            let category = self.parse_category(&device_config.category)?;

            let capabilities = if let Some(caps_config) = &device_config.capabilities {
                openclaw_device::DeviceCapabilities::default()
            } else {
                openclaw_device::DeviceCapabilities::default()
            };

            let device = openclaw_device::DeviceBuilder::new(
                device_config.id.clone(),
                device_config.name.clone(),
            )
            .platform(platform)
            .category(category)
            .capabilities(capabilities)
            .config(openclaw_device::DeviceConfig {
                endpoint: device_config.endpoint.clone(),
                api_key: device_config.api_key.clone(),
                timeout_ms: device_config.timeout_ms,
                metadata: device_config.metadata.clone(),
            })
            .build();

            let handle = device.to_handle(openclaw_device::DeviceStatus::Online);
            self.registry.register(handle).await
                .map_err(|e| openclaw_core::OpenClawError::Config(format!("Register device failed: {}", e)))?;

            tracing::info!("Registered custom device: {}", device_config.id);
        }

        Ok(())
    }

    async fn load_plugins(&self) -> openclaw_core::Result<()> {
        for plugin_config in &self.config.devices.plugins {
            if !plugin_config.enabled {
                continue;
            }

            if let Some(path) = &plugin_config.path {
                tracing::info!("Loading plugin: {} from {:?}", plugin_config.name, path);
            } else {
                tracing::debug!("Plugin {} configured but no path specified", plugin_config.name);
            }
        }

        Ok(())
    }

    fn parse_platform(&self, s: &str) -> openclaw_core::Result<openclaw_device::Platform> {
        use openclaw_device::Platform;
        
        let platform = match s.to_lowercase().as_str() {
            "cloud_server" => Platform::CloudServer,
            "docker" => Platform::Docker,
            "kubernetes" => Platform::Kubernetes,
            "linux_server" => Platform::LinuxServer,
            "linux_desktop" => Platform::LinuxDesktop,
            "linux_embedded" => Platform::LinuxEmbedded,
            "windows" => Platform::Windows,
            "macos_intel" => Platform::MacOSIntel,
            "macos_apple_silicon" => Platform::MacOSAppleSilicon,
            "android" => Platform::Android,
            "ios" => Platform::iOS,
            "raspberry_pi" | "rpi" => Platform::RaspberryPi,
            "raspberry_pi_4" | "rpi4" => Platform::RaspberryPi4,
            "orange_pi" => Platform::OrangePi,
            "nvidia_jetson_nano" => Platform::NvidiaJetsonNano,
            "nvidia_jetson_xavier" => Platform::NvidiaJetsonXavier,
            "nvidia_jetson_orin" => Platform::NvidiaJetsonOrin,
            "esp32" => Platform::Esp32,
            "stm32" => Platform::Stm32F4,
            _ => {
                tracing::warn!("Unknown platform: {}, using Unknown", s);
                Platform::Unknown
            }
        };

        Ok(platform)
    }

    fn parse_category(&self, s: &str) -> openclaw_core::Result<openclaw_device::ComputeCategory> {
        use openclaw_device::ComputeCategory;
        
        let category = match s.to_lowercase().as_str() {
            "elastic" => ComputeCategory::Elastic,
            "edge" => ComputeCategory::Edge,
            "embedded" => ComputeCategory::Embedded,
            _ => {
                tracing::warn!("Unknown category: {}, using Edge", s);
                ComputeCategory::Edge
            }
        };

        Ok(category)
    }

    pub fn registry(&self) -> &Arc<openclaw_device::DeviceRegistry> {
        &self.registry
    }
}
