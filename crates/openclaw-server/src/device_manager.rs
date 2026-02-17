use openclaw_core::Config;
use std::sync::Arc;
use tokio::sync::RwLock;

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

        self.registry.init().await.map_err(|e| {
            openclaw_core::OpenClawError::Config(format!("Device registry init failed: {}", e))
        })?;

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
            self.registry.register(handle).await.map_err(|e| {
                openclaw_core::OpenClawError::Config(format!("Register device failed: {}", e))
            })?;

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
                tracing::debug!(
                    "Plugin {} configured but no path specified",
                    plugin_config.name
                );
            }
        }

        Ok(())
    }

    fn parse_platform(&self, s: &str) -> openclaw_core::Result<openclaw_device::Platform> {
        use openclaw_device::Platform;

        let platform = match s.to_lowercase().as_str() {
            // 弹性计算
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

            // ARM 开发板
            "raspberry_pi" | "rpi" => Platform::RaspberryPi,
            "raspberry_pi_2" | "rpi2" => Platform::RaspberryPi2,
            "raspberry_pi_3" | "rpi3" => Platform::RaspberryPi3,
            "raspberry_pi_4" | "rpi4" => Platform::RaspberryPi4,
            "raspberry_pi_5" | "rpi5" => Platform::RaspberryPi5,
            "orange_pi" => Platform::OrangePi,
            "banana_pi" => Platform::BananaPi,
            "rockchip_rk3588" | "rk3588" => Platform::RockchipRk3588,
            "nvidia_jetson_nano" => Platform::NvidiaJetsonNano,
            "nvidia_jetson_xavier" => Platform::NvidiaJetsonXavier,
            "nvidia_jetson_orin" => Platform::NvidiaJetsonOrin,
            "nvidia_jetson_orin_nano" => Platform::NvidiaJetsonOrinNano,
            "google_coral" => Platform::GoogleCoral,

            // Arduino
            "arduino_uno" => Platform::ArduinoUno,
            "arduino_nano" => Platform::ArduinoNano,
            "arduino_mega" => Platform::ArduinoMega,
            "arduino_due" => Platform::ArduinoDue,

            // ESP32
            "esp32" => Platform::Esp32,
            "esp32s2" | "esp32_s2" => Platform::Esp32S2,
            "esp32s3" | "esp32_s3" => Platform::Esp32S3,
            "esp32c3" | "esp32_c3" => Platform::Esp32C3,
            "esp32c6" | "esp32_c6" => Platform::Esp32C6,
            "esp32p4" | "esp32_p4" => Platform::Esp32P4,

            // STM32
            "stm32f1" => Platform::Stm32F1,
            "stm32f4" => Platform::Stm32F4,
            "stm32h7" => Platform::Stm32H7,

            // 其他嵌入式
            "rpi_pico" | "pico" => Platform::RpiPico,
            "rpi_pico_w" | "pico_w" => Platform::RpiPicoW,
            "nrf52" => Platform::Nrf52,
            "risc_v" | "riscv" => Platform::RiscV,

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
