//! 设备注册表模块
//! 
//! 支持设备动态注册、热插拔、以及按能力查询设备

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

use crate::capabilities::DeviceCapabilities;
use crate::platform::{ComputeCategory, Platform, PlatformInfo};

pub type RegistryResult<T> = Result<T, RegistryError>;

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Device not found: {0}")]
    NotFound(String),
    #[error("Device already exists: {0}")]
    AlreadyExists(String),
    #[error("Registration failed: {0}")]
    RegistrationFailed(String),
    #[error("Discovery failed: {0}")]
    DiscoveryFailed(String),
}

/// 设备句柄
#[derive(Debug, Clone)]
pub struct DeviceHandle {
    pub id: String,
    pub name: String,
    pub platform: Platform,
    pub capabilities: DeviceCapabilities,
    pub status: DeviceStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Standby,
    Error,
}

/// 设备注册表
pub struct DeviceRegistry {
    devices: Arc<RwLock<HashMap<String, DeviceHandle>>>,
    platform_info: PlatformInfo,
    capabilities: DeviceCapabilities,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        let platform_info = PlatformInfo::detect();
        let capabilities = DeviceCapabilities::detect();
        
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            platform_info,
            capabilities,
        }
    }
    
    pub async fn init(&self) -> RegistryResult<()> {
        // 注册当前平台设备
        let local_device = DeviceHandle {
            id: "local".to_string(),
            name: Self::generate_device_name(&self.platform_info.platform),
            platform: self.platform_info.platform,
            capabilities: self.capabilities.clone(),
            status: DeviceStatus::Online,
        };
        
        let mut devices = self.devices.write().await;
        devices.insert(local_device.id.clone(), local_device);
        
        Ok(())
    }
    
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.platform_info
    }
    
    pub fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }
    
    pub async fn register(&self, device: DeviceHandle) -> RegistryResult<()> {
        let mut devices = self.devices.write().await;
        
        if devices.contains_key(&device.id) {
            return Err(RegistryError::AlreadyExists(device.id));
        }
        
        devices.insert(device.id.clone(), device);
        
        Ok(())
    }
    
    pub async fn unregister(&self, id: &str) -> RegistryResult<DeviceHandle> {
        let mut devices = self.devices.write().await;
        
        devices
            .remove(id)
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))
    }
    
    pub async fn get(&self, id: &str) -> RegistryResult<DeviceHandle> {
        let devices = self.devices.read().await;
        
        devices
            .get(id)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(id.to_string()))
    }
    
    pub async fn list(&self) -> Vec<DeviceHandle> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }
    
    pub async fn query(&self, requirements: &DeviceQuery) -> Vec<DeviceHandle> {
        let devices = self.devices.read().await;
        
        devices
            .values()
            .filter(|d| requirements.matches(d))
            .cloned()
            .collect()
    }
    
    pub async fn find_best(&self, requirements: &DeviceQuery) -> RegistryResult<DeviceHandle> {
        let candidates = self.query(requirements).await;
        
        candidates
            .into_iter()
            .max_by_key(|d| Self::score_device(d))
            .ok_or_else(|| RegistryError::NotFound("No matching device".to_string()))
    }
    
    fn generate_device_name(platform: &Platform) -> String {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "unknown".to_string());
        
        format!("{}@{}", platform.name(), hostname)
    }
    
    fn score_device(device: &DeviceHandle) -> u32 {
        let mut score = 0u32;
        
        // 在线状态优先
        if device.status == DeviceStatus::Online {
            score += 100;
        }
        
        // 根据平台类型加分
        match device.platform {
            Platform::CloudServer => score += 50,
            Platform::LinuxServer => score += 40,
            Platform::LinuxDesktop => score += 30,
            Platform::MacOSAppleSilicon => score += 25,
            Platform::MacOSIntel => score += 20,
            Platform::Windows => score += 15,
            Platform::Docker => score += 10,
            Platform::Kubernetes => score += 10,
            Platform::WasmRuntime | Platform::WasmBrowser => score += 5,
            _ => {}
        }
        
        // GPU 能力加分
        if device.capabilities.gpu.has_gpu {
            score += 20;
        }
        
        // NPU 能力加分
        if device.capabilities.gpu.has_npu {
            score += 25;
        }
        
        score
    }
}

/// 设备查询条件
#[derive(Debug, Clone, Default)]
pub struct DeviceQuery {
    pub platform: Option<Platform>,
    pub category: Option<ComputeCategory>,
    pub min_cores: Option<u32>,
    pub min_memory_bytes: Option<u64>,
    pub has_gpu: bool,
    pub has_wifi: bool,
    pub has_ethernet: bool,
    pub is_container: Option<bool>,
    pub status: Option<DeviceStatus>,
}

impl DeviceQuery {
    pub fn matches(&self, device: &DeviceHandle) -> bool {
        if let Some(platform) = &self.platform {
            if &device.platform != platform {
                return false;
            }
        }
        
        if let Some(category) = &self.category {
            if device.platform.category() != *category {
                return false;
            }
        }
        
        if let Some(min_cores) = self.min_cores {
            if device.capabilities.cpu.cores < min_cores {
                return false;
            }
        }
        
        if let Some(min_memory) = self.min_memory_bytes {
            if device.capabilities.memory.total_bytes < min_memory {
                return false;
            }
        }
        
        if self.has_gpu && !device.capabilities.gpu.has_gpu {
            return false;
        }
        
        if self.has_wifi && !device.capabilities.network.has_wifi {
            return false;
        }
        
        if self.has_ethernet && !device.capabilities.network.has_ethernet {
            return false;
        }
        
        if let Some(is_container) = self.is_container {
            if device.capabilities.features.is_container != is_container {
                return false;
            }
        }
        
        if let Some(status) = &self.status {
            if &device.status != status {
                return false;
            }
        }
        
        true
    }
}

/// 设备事件
#[derive(Debug, Clone)]
pub enum DeviceEvent {
    Connected(DeviceHandle),
    Disconnected(String),
    Updated(DeviceHandle),
    StatusChanged { id: String, status: DeviceStatus },
}

/// 设备发现器 Trait
#[async_trait]
pub trait DeviceDiscoverer: Send + Sync {
    /// 发现器名称
    fn name(&self) -> &'static str;
    
    /// 发现设备
    async fn discover(&self) -> RegistryResult<Vec<DeviceHandle>>;
}

/// 本地设备发现器
pub struct LocalDeviceDiscoverer;

#[async_trait]
impl DeviceDiscoverer for LocalDeviceDiscoverer {
    fn name(&self) -> &'static str {
        "local"
    }
    
    async fn discover(&self) -> RegistryResult<Vec<DeviceHandle>> {
        let registry = DeviceRegistry::new();
        registry.init().await?;
        
        Ok(vec![registry
            .devices
            .read()
            .await
            .get("local")
            .cloned()
            .unwrap()])
    }
}

/// 预设查询构建器
pub struct DeviceQueryBuilder(DeviceQuery);

impl DeviceQueryBuilder {
    pub fn new() -> Self {
        Self(DeviceQuery::default())
    }
    
    pub fn platform(mut self, platform: Platform) -> Self {
        self.0.platform = Some(platform);
        self
    }
    
    pub fn category(mut self, category: ComputeCategory) -> Self {
        self.0.category = Some(category);
        self
    }
    
    pub fn min_cores(mut self, cores: u32) -> Self {
        self.0.min_cores = Some(cores);
        self
    }
    
    pub fn min_memory_gb(mut self, gb: u32) -> Self {
        self.0.min_memory_bytes = Some((gb as u64) * 1024 * 1024 * 1024);
        self
    }
    
    pub fn with_gpu(mut self) -> Self {
        self.0.has_gpu = true;
        self
    }
    
    pub fn with_wifi(mut self) -> Self {
        self.0.has_wifi = true;
        self
    }
    
    pub fn in_container(mut self) -> Self {
        self.0.is_container = Some(true);
        self
    }
    
    pub fn online_only(mut self) -> Self {
        self.0.status = Some(DeviceStatus::Online);
        self
    }
    
    pub fn build(self) -> DeviceQuery {
        self.0
    }
}

impl Default for DeviceQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_registry() {
        let registry = DeviceRegistry::new();
        registry.init().await.unwrap();
        
        let devices = registry.list().await;
        assert!(!devices.is_empty());
        
        let local = registry.get("local").await.unwrap();
        println!("Local device: {} ({:?})", local.name, local.platform);
        println!("Capabilities: {:?}", local.capabilities);
    }
    
    #[tokio::test]
    async fn test_query() {
        let registry = DeviceRegistry::new();
        registry.init().await.unwrap();
        
        let query = DeviceQueryBuilder::new()
            .category(ComputeCategory::Elastic)
            .min_cores(1)
            .build();
        
        let devices = registry.query(&query).await;
        println!("Elastic devices: {:?}", devices.len());
    }
}
