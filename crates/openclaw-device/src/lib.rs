//! OpenClaw Device - 设备节点模块
//!
//! 提供设备能力节点：相机、屏幕录制、定位、通知、系统命令等
//! 以及弹性计算和边缘计算的设备抽象层

use std::sync::Arc;

pub mod nodes;
pub mod camera;
pub mod screen;
pub mod location;
pub mod notification;
pub mod system;

// 弹性计算和边缘计算设备抽象
pub mod device;
pub mod platform;
pub mod capabilities;
pub mod adapter;
pub mod registry;
pub mod device_trait;

pub use nodes::*;
pub use camera::*;
pub use screen::*;
pub use location::*;
pub use notification::*;
pub use system::*;

pub use device::*;
pub use platform::*;
pub use capabilities::*;
pub use adapter::*;
pub use registry::*;
pub use device_trait::*;

static DEVICE_REGISTRY: std::sync::OnceLock<Arc<registry::DeviceRegistry>> = std::sync::OnceLock::new();

pub async fn init_device() -> anyhow::Result<()> {
    let registry = Arc::new(registry::DeviceRegistry::new());
    registry.init().await?;
    
    DEVICE_REGISTRY.set(registry).map_err(|_| anyhow::anyhow!("Device already initialized"))?;
    
    let reg = DEVICE_REGISTRY.get().unwrap();
    let info = reg.platform_info();
    let caps = reg.capabilities();
    
    println!();
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║                     OpenClaw Device Info                      ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Platform:   {:-56} ║", format!("{:?}", info.platform));
    println!("║  Category:   {:-56} ║", format!("{:?}", info.category));
    println!("║  OS:        {:-56} ║", info.os);
    println!("║  Arch:      {:-56} ║", info.arch);
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  CPU Cores: {:-56} ║", caps.cpu.cores);
    println!("║  CPU Arch:  {:-56} ║", caps.cpu.architecture);
    println!("║  CPU Freq:  {:-56} ║", format!("{} MHz", caps.cpu.frequency_mhz));
    println!("║  Memory:    {:-56} ║", format!("{:.1} GB", caps.memory.total_bytes as f64 / 1e9));
    println!("║  Available: {:-56} ║", format!("{:.1} GB", caps.memory.available_bytes as f64 / 1e9));
    println!("╠══════════════════════════════════════════════════════════════════╣");
    
    let mut net_str = String::new();
    let mut net_first = true;
    if caps.network.has_ethernet {
        net_str.push_str("Ethernet");
        net_first = false;
    }
    if caps.network.has_wifi {
        if !net_first { net_str.push_str(", "); }
        net_str.push_str("WiFi");
        net_first = false;
    }
    if caps.network.has_ble {
        if !net_first { net_str.push_str(", "); }
        net_str.push_str("BLE");
        net_first = false;
    }
    if caps.network.has_cellular {
        if !net_first { net_str.push_str(", "); }
        net_str.push_str("Cellular");
    }
    if net_str.is_empty() {
        net_str.push_str("None");
    }
    println!("║  Network:   {:-56} ║", &net_str);
    
    println!("╠══════════════════════════════════════════════════════════════════╣");
    let mut gpu_str = String::new();
    if caps.gpu.has_gpu {
        gpu_str.push_str(caps.gpu.gpu_name.as_deref().unwrap_or("Available"));
    } else if caps.gpu.has_npu {
        gpu_str.push_str("NPU Available");
    } else {
        gpu_str.push_str("None");
    }
    println!("║  GPU:       {:-56} ║", &gpu_str);
    
    println!("╠══════════════════════════════════════════════════════════════════╣");
    let mut feat_str = String::new();
    let mut feat_first = true;
    if caps.features.is_container {
        feat_str.push_str("Container");
        feat_first = false;
    }
    if caps.features.is_wasm {
        if !feat_first { feat_str.push_str(", "); }
        feat_str.push_str("WASM");
        feat_first = false;
    }
    if caps.features.is_virtualized {
        if !feat_first { feat_str.push_str(", "); }
        feat_str.push_str("Virtualized");
        feat_first = false;
    }
    if caps.features.has_sgx {
        if !feat_first { feat_str.push_str(", "); }
        feat_str.push_str("SGX");
        feat_first = false;
    }
    if caps.features.has_tpm {
        if !feat_first { feat_str.push_str(", "); }
        feat_str.push_str("TPM");
        feat_first = false;
    }
    if caps.features.has_npu {
        if !feat_first { feat_str.push_str(", "); }
        feat_str.push_str("NPU");
    }
    if feat_str.is_empty() {
        feat_str.push_str("None");
    }
    println!("║  Features:  {:-56} ║", &feat_str);
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();
    
    tracing::info!("Device initialized: platform={:?}, category={:?}", 
        info.platform, info.category);
    
    Ok(())
}

pub fn get_device_registry() -> Option<Arc<registry::DeviceRegistry>> {
    DEVICE_REGISTRY.get().cloned()
}

pub async fn get_adapter_config() -> anyhow::Result<adapter::AdapterConfig> {
    let registry = DEVICE_REGISTRY.get().ok_or_else(|| anyhow::anyhow!("Device not initialized"))?;
    
    adapter::Adapters::apply_all(
        &registry.platform_info().platform,
        registry.capabilities()
    ).await.map_err(|e| anyhow::anyhow!("Adapter error: {}", e))
}
