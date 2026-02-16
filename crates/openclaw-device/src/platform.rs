//! 平台检测模块
//! 
//! 检测当前运行环境：弹性计算 (Cloud/Wasm/Docker) 和 边缘计算 (Edge/Embedded)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeCategory {
    Elastic,
    Edge,
    Embedded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    // 弹性计算
    CloudServer,
    Docker,
    Kubernetes,
    WasmBrowser,
    WasmRuntime,
    Serverless,
    
    // 边缘计算
    LinuxDesktop,
    LinuxServer,
    LinuxEmbedded,
    Windows,
    MacOSIntel,
    MacOSAppleSilicon,
    Android,
    iOS,
    
    // 嵌入式
    Esp32,
    Esp32S2,
    Esp32S3,
    Esp32C3,
    Stm32F1,
    Stm32F4,
    Stm32H7,
    RpiPico,
    Nrf52,
    RiscV,
    
    Unknown,
}

impl Platform {
    pub fn category(&self) -> ComputeCategory {
        match self {
            // 弹性计算
            Self::CloudServer | Self::Docker | Self::Kubernetes 
            | Self::WasmBrowser | Self::WasmRuntime | Self::Serverless => ComputeCategory::Elastic,
            
            // 边缘计算
            Self::LinuxDesktop | Self::LinuxServer | Self::LinuxEmbedded
            | Self::Windows | Self::MacOSIntel | Self::MacOSAppleSilicon
            | Self::Android | Self::iOS => ComputeCategory::Edge,
            
            // 嵌入式
            Self::Esp32 | Self::Esp32S2 | Self::Esp32S3 | Self::Esp32C3
            | Self::Stm32F1 | Self::Stm32F4 | Self::Stm32H7
            | Self::RpiPico | Self::Nrf52 | Self::RiscV => ComputeCategory::Embedded,
            
            Self::Unknown => ComputeCategory::Edge,
        }
    }
    
    pub fn name(&self) -> &'static str {
        match self {
            Self::CloudServer => "cloud_server",
            Self::Docker => "docker",
            Self::Kubernetes => "kubernetes",
            Self::WasmBrowser => "wasm_browser",
            Self::WasmRuntime => "wasm_runtime",
            Self::Serverless => "serverless",
            Self::LinuxDesktop => "linux_desktop",
            Self::LinuxServer => "linux_server",
            Self::LinuxEmbedded => "linux_embedded",
            Self::Windows => "windows",
            Self::MacOSIntel => "macos_intel",
            Self::MacOSAppleSilicon => "macos_apple_silicon",
            Self::Android => "android",
            Self::iOS => "ios",
            Self::Esp32 => "esp32",
            Self::Esp32S2 => "esp32s2",
            Self::Esp32S3 => "esp32s3",
            Self::Esp32C3 => "esp32c3",
            Self::Stm32F1 => "stm32f1",
            Self::Stm32F4 => "stm32f4",
            Self::Stm32H7 => "stm32h7",
            Self::RpiPico => "rpi_pico",
            Self::Nrf52 => "nrf52",
            Self::RiscV => "risc_v",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub platform: Platform,
    pub category: ComputeCategory,
    pub arch: String,
    pub os: String,
    pub env_vars: HashMap<String, String>,
    pub is_container: bool,
    pub is_embedded: bool,
}

impl Default for PlatformInfo {
    fn default() -> Self {
        Self::detect()
    }
}

impl PlatformInfo {
    pub fn detect() -> Self {
        let platform = Self::detect_platform();
        let category = platform.category();
        
        Self {
            platform,
            category,
            arch: Self::detect_arch(),
            os: Self::detect_os(),
            env_vars: Self::detect_env_vars(),
            is_container: Self::detect_container(),
            is_embedded: category == ComputeCategory::Embedded,
        }
    }
    
    fn detect_platform() -> Platform {
        #[cfg(target_arch = "wasm32")]
        {
            return Self::detect_wasm();
        }
        
        #[cfg(target_os = "linux")]
        {
            return Self::detect_linux();
        }
        
        #[cfg(target_os = "windows")]
        {
            return Platform::Windows;
        }
        
        #[cfg(target_os = "macos")]
        {
            return Self::detect_macos();
        }
        
        #[cfg(target_os = "android")]
        {
            return Platform::Android;
        }
        
        #[cfg(target_os = "ios")]
        {
            return Platform::iOS;
        }
        
        // 嵌入式检测 (编译时)
        #[cfg(feature = "esp32")]
        return Platform::Esp32;
        
        #[cfg(feature = "stm32h7")]
        return Platform::Stm32H7;
        
        Platform::Unknown
    }
    
    fn detect_wasm() -> Platform {
        // 检测是否在浏览器中
        #[cfg(all(target_arch = "wasm32", feature = "browser"))]
        {
            if let Some(window) = web_sys::window() {
                if window.location().origin().is_ok() {
                    return Platform::WasmBrowser;
                }
            }
        }
        
        Platform::WasmRuntime
    }
    
    fn detect_linux() -> Platform {
        // 检测容器
        if Self::detect_container() {
            if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
                return Platform::Kubernetes;
            }
            return Platform::Docker;
        }
        
        // 检测是否为嵌入式 Linux
        if std::path::Path::new("/proc/device-tree/model").exists() {
            if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
                if model.contains("Raspberry Pi") {
                    return Platform::LinuxEmbedded;
                }
            }
        }
        
        // 检测桌面环境
        if std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok() {
            return Platform::LinuxDesktop;
        }
        
        Platform::LinuxServer
    }
    
    fn detect_macos() -> Platform {
        // 检测 Apple Silicon
        #[cfg(target_arch = "aarch64")]
        {
            return Platform::MacOSAppleSilicon;
        }
        
        Platform::MacOSIntel
    }
    
    fn detect_arch() -> String {
        std::env::consts::ARCH.to_string()
    }
    
    fn detect_os() -> String {
        std::env::consts::OS.to_string()
    }
    
    fn detect_env_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        for (key, value) in std::env::vars() {
            vars.insert(key, value);
        }
        vars
    }
    
    fn detect_container() -> bool {
        // 检查 cgroup
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker") || cgroup.contains("containerd") || cgroup.contains("kubepods") {
                return true;
            }
        }
        
        // 检查 .dockerenv
        if std::path::Path::new("/.dockerenv").exists() {
            return true;
        }
        
        // 检查环境变量
        if std::env::var("DOCKER_CONTAINER").is_ok() {
            return true;
        }
        
        false
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
