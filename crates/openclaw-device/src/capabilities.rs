//! 设备能力检测模块
//!
//! 自动检测设备的 CPU、内存、存储、网络、GPU 等能力

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub cpu: CpuCapability,
    pub memory: MemoryCapability,
    pub storage: StorageCapability,
    pub network: NetworkCapability,
    pub gpu: GpuCapability,
    pub peripherals: Vec<PeripheralType>,
    pub sensors: Vec<SensorType>,
    pub features: FeatureFlags,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self::detect()
    }
}

impl DeviceCapabilities {
    pub fn detect() -> Self {
        Self {
            cpu: CpuCapability::detect(),
            memory: MemoryCapability::detect(),
            storage: StorageCapability::detect(),
            network: NetworkCapability::detect(),
            gpu: GpuCapability::detect(),
            peripherals: Self::detect_peripherals(),
            sensors: Self::detect_sensors(),
            features: FeatureFlags::detect(),
        }
    }

    pub fn is_embedded(&self) -> bool {
        matches!(self.cpu.architecture(), "cortex-m" | "xtensa" | "riscv")
    }

    pub fn is_container(&self) -> bool {
        self.features.is_container
    }

    pub fn is_wasm(&self) -> bool {
        self.features.is_wasm
    }

    pub fn can_run_ai(&self) -> bool {
        self.gpu.has_gpu || self.features.has_npu
    }

    fn detect_peripherals() -> Vec<PeripheralType> {
        let mut peripherals = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // 检测 USB
            if std::path::Path::new("/dev/bus/usb").exists() {
                peripherals.push(PeripheralType::Usb);
            }

            // 检测 GPIO
            if std::path::Path::new("/sys/class/gpio").exists() {
                peripherals.push(PeripheralType::Gpio);
            }

            // 检测 I2C
            if std::path::Path::new("/dev/i2c-").exists() {
                peripherals.push(PeripheralType::I2c);
            }

            // 检测 SPI
            if std::path::Path::new("/dev/spidev").exists() {
                peripherals.push(PeripheralType::Spi);
            }

            // 检测串口
            if std::path::Path::new("/dev/tty").exists() {
                peripherals.push(PeripheralType::Uart);
            }
        }

        #[cfg(target_os = "macos")]
        {
            peripherals.push(PeripheralType::Thunderbolt);
            peripherals.push(PeripheralType::Bluetooth);
        }

        #[cfg(target_os = "windows")]
        {
            peripherals.push(PeripheralType::Com);
        }

        peripherals
    }

    fn detect_sensors() -> Vec<SensorType> {
        let mut sensors = Vec::new();

        #[cfg(target_os = "linux")]
        {
            // 检测硬件传感器
            if std::path::Path::new("/sys/class/hwmon").exists() {
                sensors.push(SensorType::Temperature);
                sensors.push(SensorType::Voltage);
                sensors.push(SensorType::Fan);
            }

            // 检测 GPS
            if std::path::Path::new("/dev/ttyUSB0").exists() {
                sensors.push(SensorType::Gps);
            }

            // 检测摄像头
            if std::path::Path::new("/dev/video0").exists() {
                sensors.push(SensorType::Camera);
            }
        }

        sensors
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCapability {
    pub cores: u32,
    pub threads: u32,
    pub frequency_mhz: u32,
    pub architecture: String,
    pub has_float: bool,
    pub has_simd: bool,
    pub has_crypto: bool,
}

impl CpuCapability {
    fn detect() -> Self {
        let cores = Self::detect_cores();

        Self {
            cores,
            threads: cores,
            frequency_mhz: Self::detect_frequency(),
            architecture: Self::detect_architecture(),
            has_float: true,
            has_simd: Self::detect_simd(),
            has_crypto: Self::detect_crypto(),
        }
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    fn detect_cores() -> u32 {
        #[cfg(target_arch = "wasm32")]
        return 1;

        std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(1)
    }

    fn detect_frequency() -> u32 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(freq) = std::fs::read_to_string("/proc/cpuinfo") {
                for line in freq.lines() {
                    if line.starts_with("cpu MHz") {
                        if let Some(freq) = line.split(':').nth(1) {
                            if let Ok(f) = freq.trim().parse::<f64>() {
                                return f as u32;
                            }
                        }
                    }
                }
            }
        }

        // 默认值
        match std::env::consts::ARCH {
            "x86_64" => 3000,
            "aarch64" => 2000,
            "arm" => 1000,
            _ => 100,
        }
    }

    fn detect_architecture() -> String {
        std::env::consts::ARCH.to_string()
    }

    fn detect_simd() -> bool {
        match std::env::consts::ARCH {
            "x86_64" | "aarch64" | "arm" => true,
            _ => false,
        }
    }

    fn detect_crypto() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            if let Ok(_cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                return true; // 假设大多数 x86 有 AES-NI
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapability {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_bytes: u64,
    pub page_size_bytes: u64,
}

impl MemoryCapability {
    fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                let mut total = 0u64;
                let mut available = 0u64;
                let mut swap = 0u64;
                let mut page_size = 4096u64;

                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        total = Self::parse_meminfo_value(line) * 1024;
                    } else if line.starts_with("MemAvailable:") {
                        available = Self::parse_meminfo_value(line) * 1024;
                    } else if line.starts_with("SwapTotal:") {
                        swap = Self::parse_meminfo_value(line) * 1024;
                    }
                }

                return Self {
                    total_bytes: total,
                    available_bytes: available,
                    swap_bytes: swap,
                    page_size_bytes: page_size,
                };
            }
        }

        // 默认值
        Self {
            total_bytes: 8 * 1024 * 1024 * 1024,     // 8GB
            available_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            swap_bytes: 0,
            page_size_bytes: 4096,
        }
    }

    fn parse_meminfo_value(line: &str) -> u64 {
        line.split(':')
            .nth(1)
            .and_then(|v| v.trim().split_whitespace().next())
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCapability {
    pub has_flash: bool,
    pub has_sdcard: bool,
    pub has_emmc: bool,
    pub has_ssd: bool,
    pub has_hdd: bool,
    pub total_bytes: u64,
}

impl StorageCapability {
    fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            let mut has_flash = std::path::Path::new("/sys/class/mtd").exists();
            let has_sdcard = std::path::Path::new("/dev/mmcblk0").exists();
            let has_emmc = std::path::Path::new("/dev/mmcblk0boot0").exists();

            // 检测挂载的存储
            let mut total = 0u64;
            if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
                for line in mounts.lines() {
                    if line.contains("/dev/") {
                        // 简单估算
                        total += 10 * 1024 * 1024 * 1024; // 10GB 估算
                    }
                }
            }

            return Self {
                has_flash,
                has_sdcard,
                has_emmc,
                has_ssd: std::path::Path::new("/dev/nvme").exists(),
                has_hdd: std::path::Path::new("/dev/sd").exists(),
                total_bytes: total,
            };
        }

        Self::default()
    }
}

impl Default for StorageCapability {
    fn default() -> Self {
        Self {
            has_flash: false,
            has_sdcard: false,
            has_emmc: false,
            has_ssd: true,
            has_hdd: false,
            total_bytes: 100 * 1024 * 1024 * 1024, // 100GB
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapability {
    pub has_ethernet: bool,
    pub has_wifi: bool,
    pub has_ble: bool,
    pub has_cellular: bool,
    pub has_usb_ethernet: bool,
    pub max_speed_mbps: u32,
    #[serde(default)]
    pub supported_protocols: Vec<NetworkProtocol>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProtocol {
    Ethernet100M,
    Ethernet1G,
    Ethernet10G,
    Wifi4,
    Wifi5,
    Wifi6,
    Wifi6E,
    Lte,
    LteAdvanced,
    FiveG,
    LoraWan,
    NbIot,
    Zigbee,
    Thread,
    Ble,
    Usb,
}

impl NetworkCapability {
    fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            let has_ethernet = std::path::Path::new("/sys/class/net/eth0").exists();
            let has_wifi = std::path::Path::new("/sys/class/net/wlan0").exists();
            let has_ble = std::path::Path::new("/sys/class/bluetooth").exists();

            let mut protocols = Vec::new();
            if has_ethernet {
                protocols.push(NetworkProtocol::Ethernet1G);
            }
            if has_wifi {
                protocols.push(NetworkProtocol::Wifi5);
            }
            if has_ble {
                protocols.push(NetworkProtocol::Ble);
            }

            return Self {
                has_ethernet,
                has_wifi,
                has_ble,
                has_cellular: std::path::Path::new("/dev/cdc-wdm0").exists(),
                has_usb_ethernet: std::path::Path::new("/sys/class/net/usb0").exists(),
                max_speed_mbps: if has_ethernet {
                    1000
                } else if has_wifi {
                    1200
                } else {
                    100
                },
                supported_protocols: protocols,
            };
        }

        Self::default()
    }
}

impl Default for NetworkCapability {
    fn default() -> Self {
        Self {
            has_ethernet: true,
            has_wifi: true,
            has_ble: true,
            has_cellular: false,
            has_usb_ethernet: false,
            max_speed_mbps: 1000,
            supported_protocols: vec![
                NetworkProtocol::Ethernet1G,
                NetworkProtocol::Wifi5,
                NetworkProtocol::Ble,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapability {
    pub has_gpu: bool,
    pub gpu_name: Option<String>,
    pub vram_bytes: u64,
    pub has_npu: bool,
    pub npu_name: Option<String>,
    pub supports_vulkan: bool,
    pub supports_cuda: bool,
    pub supports_opencl: bool,
}

impl GpuCapability {
    fn detect() -> Self {
        #[cfg(target_os = "linux")]
        {
            let has_gpu = std::path::Path::new("/dev/dri").exists();
            let has_npu = std::path::Path::new("/dev/npu").exists();

            return Self {
                has_gpu,
                gpu_name: Self::detect_gpu_name(),
                vram_bytes: Self::detect_vram(),
                has_npu,
                npu_name: if has_npu {
                    Some("NPU".to_string())
                } else {
                    None
                },
                supports_vulkan: std::path::Path::new("/dev/dri/renderD128").exists(),
                supports_cuda: std::path::Path::new("/usr/bin/nvidia-smi").is_ok(),
                supports_opencl: std::path::Path::new("/etc/OpenCL").exists(),
            };
        }

        Self::default()
    }

    fn detect_gpu_name() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(lspci) = std::process::Command::new("lspci")
                .arg("-mm")
                .arg("-n")
                .output()
            {
                let output = String::from_utf8_lossy(&lspci.stdout);
                if output.contains("VGA") || output.contains("Display") {
                    for line in output.lines() {
                        if line.contains("VGA") || line.contains("Display") {
                            return Some(line.split(':').nth(2).unwrap_or("Unknown").to_string());
                        }
                    }
                }
            }
        }
        None
    }

    fn detect_vram() -> u64 {
        #[cfg(target_os = "linux")]
        {
            // 简化检测
            return 8 * 1024 * 1024 * 1024; // 8GB
        }
        4 * 1024 * 1024 * 1024 // 4GB
    }
}

impl Default for GpuCapability {
    fn default() -> Self {
        Self {
            has_gpu: false,
            gpu_name: None,
            vram_bytes: 0,
            has_npu: false,
            npu_name: None,
            supports_vulkan: false,
            supports_cuda: false,
            supports_opencl: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub is_container: bool,
    pub is_wasm: bool,
    pub is_virtualized: bool,
    pub has_sgx: bool,
    pub has_tpm: bool,
    pub has_secure_boot: bool,
    pub supports_hotplug: bool,
    pub has_npu: bool,
}

impl FeatureFlags {
    fn detect() -> Self {
        Self {
            is_container: Self::detect_container(),
            is_wasm: Self::detect_wasm(),
            is_virtualized: Self::detect_virtualization(),
            has_sgx: Self::detect_sgx(),
            has_tpm: std::path::Path::new("/dev/tpm0").exists(),
            has_secure_boot: Self::detect_secure_boot(),
            supports_hotplug: Self::detect_hotplug(),
            has_npu: Self::detect_npu(),
        }
    }

    fn detect_npu() -> bool {
        #[cfg(target_os = "linux")]
        {
            std::path::Path::new("/dev/accel/accel0").exists()
                || std::path::Path::new("/dev/dri/card0").exists()
                || std::env::var("HABANA_VISIBLE_DEVICES").is_ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn detect_container() -> bool {
        std::path::Path::new("/.dockerenv").exists() || std::env::var("DOCKER_CONTAINER").is_ok()
    }

    fn detect_wasm() -> bool {
        #[cfg(target_arch = "wasm32")]
        return true;
        false
    }

    fn detect_virtualization() -> bool {
        #[cfg(target_os = "linux")]
        {
            if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
                if cpuinfo.contains("hypervisor")
                    || cpuinfo.contains("QEMU")
                    || cpuinfo.contains("KVM")
                {
                    return true;
                }
            }
        }
        false
    }

    fn detect_sgx() -> bool {
        #[cfg(target_os = "linux")]
        {
            return std::path::Path::new("/dev/isgx").exists();
        }
        false
    }

    fn detect_secure_boot() -> bool {
        #[cfg(target_os = "linux")]
        {
            return std::path::Path::new(
                "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e09832b8cd",
            )
            .is_ok();
        }
        false
    }

    fn detect_hotplug() -> bool {
        #[cfg(target_os = "linux")]
        {
            return std::path::Path::new("/sys/kernel/config").exists();
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeripheralType {
    Usb,
    Gpio,
    I2c,
    Spi,
    Uart,
    Can,
    Thunderbolt,
    Bluetooth,
    Com,
    Ethernet,
}

impl PeripheralType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Usb => "usb",
            Self::Gpio => "gpio",
            Self::I2c => "i2c",
            Self::Spi => "spi",
            Self::Uart => "uart",
            Self::Can => "can",
            Self::Thunderbolt => "thunderbolt",
            Self::Bluetooth => "bluetooth",
            Self::Com => "com",
            Self::Ethernet => "ethernet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorType {
    Temperature,
    Voltage,
    Current,
    Fan,
    Pressure,
    Humidity,
    Gps,
    Camera,
    Microphone,
    Accelerometer,
    Gyroscope,
    Magnetometer,
    Proximity,
    Light,
}

impl SensorType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Voltage => "voltage",
            Self::Current => "current",
            Self::Fan => "fan",
            Self::Pressure => "pressure",
            Self::Humidity => "humidity",
            Self::Gps => "gps",
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::Accelerometer => "accelerometer",
            Self::Gyroscope => "gyroscope",
            Self::Magnetometer => "magnetometer",
            Self::Proximity => "proximity",
            Self::Light => "light",
        }
    }
}
