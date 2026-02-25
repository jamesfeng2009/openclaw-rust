//! WASM 运行时配置

use serde::{Deserialize, Serialize};

/// WASM 运行时配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmRuntimeConfig {
    /// 内存限制 (MB)
    pub memory_limit_mb: u64,
    /// 计算限制 (指令数)
    pub compute_limit: u64,
    /// 超时时间 (秒)
    pub timeout_secs: u64,
    /// 是否允许网络
    pub allow_network: bool,
    /// 允许的函数列表
    pub allowed_functions: Vec<String>,
}

impl Default for WasmRuntimeConfig {
    fn default() -> Self {
        Self {
            memory_limit_mb: 64,
            compute_limit: 1024,
            timeout_secs: 30,
            allow_network: false,
            allowed_functions: vec!["_start".to_string(), "run".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WasmRuntimeConfig::default();
        assert_eq!(config.memory_limit_mb, 64);
        assert_eq!(config.compute_limit, 1024);
        assert_eq!(config.timeout_secs, 30);
        assert!(!config.allow_network);
    }

    #[test]
    fn test_config_serialization() {
        let config = WasmRuntimeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let decoded: WasmRuntimeConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.memory_limit_mb, 64);
    }
}
