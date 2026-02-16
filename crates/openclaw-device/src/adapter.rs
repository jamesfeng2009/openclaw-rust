//! 设备适配器模块
//! 
//! 根据设备能力自动选择合适的驱动适配器

use async_trait::async_trait;
use std::sync::Arc;

use super::capabilities::DeviceCapabilities;
use super::platform::Platform;

pub type AdapterResult<T> = Result<T, AdapterError>;

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Adapter not supported for this platform")]
    NotSupported,
    #[error("Adapter initialization failed: {0}")]
    InitFailed(String),
    #[error("Adapter operation failed: {0}")]
    OperationFailed(String),
}

/// 设备适配器 Trait
#[async_trait]
pub trait DeviceAdapter: Send + Sync {
    /// 适配器名称
    fn name(&self) -> &'static str;
    
    /// 支持的平台
    fn supports_platform(&self, platform: &Platform) -> bool;
    
    /// 检查是否需要此适配器 (默认: 如果支持平台则应用)
    fn should_apply(&self, _platform: &Platform, _capabilities: &DeviceCapabilities) -> bool {
        self.supports_platform(_platform)
    }
    
    /// 初始化适配器
    async fn init(&self) -> AdapterResult<()>;
    
    /// 适配器配置回调
    fn configure(&self, config: &mut AdapterConfig) {
        // 默认空实现
    }
}

/// 适配器配置
#[derive(Debug, Clone)]
pub struct AdapterConfig {
    pub memory_limit: Option<u64>,
    pub thread_count: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub cache_dir: Option<String>,
    pub data_dir: Option<String>,
    pub enable_logging: bool,
    pub custom: std::collections::HashMap<String, String>,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            memory_limit: None,
            thread_count: None,
            timeout_ms: Some(30000),
            cache_dir: None,
            data_dir: None,
            enable_logging: true,
            custom: std::collections::HashMap::new(),
        }
    }
}

/// 预定义适配器
pub struct Adapters;

impl Adapters {
    /// 根据平台和能力自动选择适配器
    pub fn auto_select(platform: &Platform, capabilities: &DeviceCapabilities) -> Vec<Arc<dyn DeviceAdapter>> {
        let mut adapters: Vec<Arc<dyn DeviceAdapter>> = Vec::new();
        
        // 基础适配器 (所有平台)
        adapters.push(Arc::new(BaseAdapter));
        
        // 网络适配器
        if capabilities.network.has_wifi {
            adapters.push(Arc::new(WifiAdapter));
        }
        if capabilities.network.has_ble {
            adapters.push(Arc::new(BleAdapter));
        }
        if capabilities.network.has_ethernet {
            adapters.push(Arc::new(EthernetAdapter));
        }
        
        // 存储适配器
        if capabilities.storage.has_flash {
            adapters.push(Arc::new(FlashStorageAdapter));
        }
        if capabilities.storage.has_sdcard {
            adapters.push(Arc::new(SdCardAdapter));
        }
        
        // GPU/NPU 适配器
        if capabilities.gpu.has_gpu {
            adapters.push(Arc::new(GpuAdapter));
        }
        if capabilities.gpu.has_npu {
            adapters.push(Arc::new(NpuAdapter));
        }
        
        // 容器适配器
        if capabilities.features.is_container {
            adapters.push(Arc::new(ContainerAdapter));
        }
        
        // Wasm 适配器
        if capabilities.features.is_wasm {
            adapters.push(Arc::new(WasmAdapter));
        }
        
        // 嵌入式适配器
        if capabilities.peripherals.iter().any(|p| p.name() == "gpio") {
            adapters.push(Arc::new(GpioAdapter));
        }
        
        adapters
    }
    
    /// 应用所有适配器
    pub async fn apply_all(
        platform: &Platform,
        capabilities: &DeviceCapabilities,
    ) -> AdapterResult<AdapterConfig> {
        let adapters = Self::auto_select(platform, capabilities);
        let mut config = AdapterConfig::default();
        
        for adapter in adapters {
            if adapter.should_apply(platform, capabilities) {
                adapter.configure(&mut config);
                adapter.init().await?;
            }
        }
        
        Ok(config)
    }
}

// ============== 基础适配器 ==============

struct BaseAdapter;

#[async_trait]
impl DeviceAdapter for BaseAdapter {
    fn name(&self) -> &'static str {
        "base"
    }
    
    fn supports_platform(&self, _platform: &Platform) -> bool {
        true
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

// ============== 网络适配器 ==============

struct WifiAdapter;

#[async_trait]
impl DeviceAdapter for WifiAdapter {
    fn name(&self) -> &'static str {
        "wifi"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::Esp32 | Platform::Esp32S2 | Platform::Esp32S3 | Platform::Esp32C3
            | Platform::LinuxEmbedded | Platform::LinuxDesktop
            | Platform::Android | Platform::MacOSIntel | Platform::MacOSAppleSilicon
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

struct BleAdapter;

#[async_trait]
impl DeviceAdapter for BleAdapter {
    fn name(&self) -> &'static str {
        "ble"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::Esp32 | Platform::Esp32C3 | Platform::Nrf52
            | Platform::LinuxDesktop | Platform::MacOSIntel | Platform::MacOSAppleSilicon
            | Platform::Android | Platform::iOS
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

struct EthernetAdapter;

#[async_trait]
impl DeviceAdapter for EthernetAdapter {
    fn name(&self) -> &'static str {
        "ethernet"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::LinuxServer | Platform::LinuxDesktop | Platform::LinuxEmbedded
            | Platform::Stm32H7 | Platform::Esp32S3
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

// ============== 存储适配器 ==============

struct FlashStorageAdapter;

#[async_trait]
impl DeviceAdapter for FlashStorageAdapter {
    fn name(&self) -> &'static str {
        "flash_storage"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::Esp32 | Platform::Esp32S2 | Platform::Esp32S3 | Platform::Esp32C3
            | Platform::Stm32F1 | Platform::Stm32F4 | Platform::Stm32H7
            | Platform::RpiPico | Platform::Nrf52 | Platform::RiscV
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

struct SdCardAdapter;

#[async_trait]
impl DeviceAdapter for SdCardAdapter {
    fn name(&self) -> &'static str {
        "sdcard"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::LinuxEmbedded | Platform::Esp32S3 | Platform::Stm32H7
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

// ============== GPU/NPU 适配器 ==============

struct GpuAdapter;

#[async_trait]
impl DeviceAdapter for GpuAdapter {
    fn name(&self) -> &'static str {
        "gpu"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::LinuxDesktop | Platform::LinuxServer | Platform::CloudServer
            | Platform::MacOSIntel | Platform::MacOSAppleSilicon | Platform::Windows
        )
    }
    
    fn configure(&self, config: &mut AdapterConfig) {
        config.memory_limit = Some(4 * 1024 * 1024 * 1024); // 4GB 推荐
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

struct NpuAdapter;

#[async_trait]
impl DeviceAdapter for NpuAdapter {
    fn name(&self) -> &'static str {
        "npu"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::LinuxEmbedded | Platform::LinuxDesktop
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

// ============== 容器适配器 ==============

struct ContainerAdapter;

#[async_trait]
impl DeviceAdapter for ContainerAdapter {
    fn name(&self) -> &'static str {
        "container"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform, Platform::Docker | Platform::Kubernetes)
    }
    
    fn configure(&self, config: &mut AdapterConfig) {
        config.memory_limit = detect_container_limits().ok();
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

fn detect_container_limits() -> Result<u64, std::io::Error> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(limit) = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes") {
            if let Ok(limit) = limit.trim().parse::<u64>() {
                if limit != u64::MAX {
                    return Ok(limit);
                }
            }
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No limit found"))
}

// ============== Wasm 适配器 ==============

struct WasmAdapter;

#[async_trait]
impl DeviceAdapter for WasmAdapter {
    fn name(&self) -> &'static str {
        "wasm"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(platform, Platform::WasmBrowser | Platform::WasmRuntime)
    }
    
    fn configure(&self, config: &mut AdapterConfig) {
        config.memory_limit = Some(2 * 1024 * 1024 * 1024); // 2GB 沙箱限制
        config.thread_count = Some(1);
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}

// ============== 嵌入式适配器 ==============

struct GpioAdapter;

#[async_trait]
impl DeviceAdapter for GpioAdapter {
    fn name(&self) -> &'static str {
        "gpio"
    }
    
    fn supports_platform(&self, platform: &Platform) -> bool {
        matches!(
            platform,
            Platform::Esp32 | Platform::Esp32S2 | Platform::Esp32S3 | Platform::Esp32C3
            | Platform::Stm32F1 | Platform::Stm32F4 | Platform::Stm32H7
            | Platform::RpiPico | Platform::LinuxEmbedded
        )
    }
    
    async fn init(&self) -> AdapterResult<()> {
        Ok(())
    }
}
