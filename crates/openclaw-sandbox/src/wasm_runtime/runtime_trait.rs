//! WASM 运行时 trait 定义

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// WASM 运行时类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum WasmRuntimeType {
    #[default]
    #[serde(rename = "wasmtime")]
    Wasmtime,
    #[serde(rename = "wasmi")]
    Wasmi,
}

impl std::fmt::Display for WasmRuntimeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmRuntimeType::Wasmtime => write!(f, "wasmtime"),
            WasmRuntimeType::Wasmi => write!(f, "wasmi"),
        }
    }
}

impl WasmRuntimeType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "wasmtime" => Some(WasmRuntimeType::Wasmtime),
            "wasmi" => Some(WasmRuntimeType::Wasmi),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            WasmRuntimeType::Wasmtime => "高性能 JIT 编译器",
            WasmRuntimeType::Wasmi => "轻量解释器，内存占用低",
        }
    }
}

/// WASM 模块元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmModule {
    pub name: String,
    pub size_bytes: u64,
}

/// WASM 执行输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionInput {
    pub function: String,
    pub params: serde_json::Value,
}

/// WASM 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub memory_used_bytes: u64,
    pub execution_time_ms: u64,
}

impl WasmExecutionResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
            memory_used_bytes: 0,
            execution_time_ms: 0,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(msg.to_string()),
            memory_used_bytes: 0,
            execution_time_ms: 0,
        }
    }
}

/// WASM 运行时错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmError {
    #[serde(rename = "init_failed")]
    InitFailed(String),
    #[serde(rename = "load_failed")]
    LoadFailed(String),
    #[serde(rename = "execution_failed")]
    ExecutionFailed(String),
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "memory_limit_exceeded")]
    MemoryLimitExceeded,
    #[serde(rename = "compute_limit_exceeded")]
    ComputeLimitExceeded,
    #[serde(rename = "network_denied")]
    NetworkDenied,
    #[serde(rename = "function_not_found")]
    FunctionNotFound(String),
    #[serde(rename = "runtime_not_available")]
    RuntimeNotAvailable(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::InitFailed(s) => write!(f, "WASM init failed: {}", s),
            WasmError::LoadFailed(s) => write!(f, "WASM load failed: {}", s),
            WasmError::ExecutionFailed(s) => write!(f, "WASM execution failed: {}", s),
            WasmError::Timeout => write!(f, "Execution timeout"),
            WasmError::MemoryLimitExceeded => write!(f, "Memory limit exceeded"),
            WasmError::ComputeLimitExceeded => write!(f, "Compute limit exceeded"),
            WasmError::NetworkDenied => write!(f, "Network access denied"),
            WasmError::FunctionNotFound(s) => write!(f, "Function not found: {}", s),
            WasmError::RuntimeNotAvailable(s) => write!(f, "Runtime not available: {}", s),
        }
    }
}

impl std::error::Error for WasmError {}

/// WASM 运行时 trait
#[async_trait]
pub trait WasmRuntime: Send + Sync {
    /// 加载 WASM 模块
    async fn load_module(&self, wasm_bytes: &[u8], name: &str) -> Result<WasmModule, WasmError>;

    /// 执行 WASM 函数
    async fn execute(
        &self,
        module: &WasmModule,
        input: &WasmExecutionInput,
    ) -> Result<WasmExecutionResult, WasmError>;

    /// 获取运行时类型
    fn runtime_type(&self) -> WasmRuntimeType;

    /// 检查运行时是否可用
    fn is_available(&self) -> bool;
}

/// 创建 WASM 运行时的工厂函数
pub fn create_wasm_runtime(
    runtime_type: WasmRuntimeType,
    config: &super::config::WasmRuntimeConfig,
) -> Result<Box<dyn WasmRuntime>, WasmError> {
    match runtime_type {
        #[cfg(feature = "wasm-runtime-wasmtime")]
        WasmRuntimeType::Wasmtime => {
            Ok(Box::new(super::wasmtime_runtime::WasmtimeRuntime::new(config)?))
        }
        #[cfg(not(feature = "wasm-runtime-wasmtime"))]
        WasmRuntimeType::Wasmtime => {
            Err(WasmError::RuntimeNotAvailable("wasmtime not compiled".to_string()))
        }
        #[cfg(feature = "wasm-runtime-wasmi")]
        WasmRuntimeType::Wasmi => {
            Ok(Box::new(super::wasmi_runtime::WasmiRuntime::new(config)?))
        }
        #[cfg(not(feature = "wasm-runtime-wasmi"))]
        WasmRuntimeType::Wasmi => {
            Err(WasmError::RuntimeNotAvailable("wasmi not compiled".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_execution_result_success() {
        let result = WasmExecutionResult::success(serde_json::json!({"status": "ok"}));
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_wasm_execution_result_error() {
        let result = WasmExecutionResult::error("test error");
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_wasm_runtime_type_from_str() {
        assert_eq!(WasmRuntimeType::from_str("wasmtime"), Some(WasmRuntimeType::Wasmtime));
        assert_eq!(WasmRuntimeType::from_str("wasmi"), Some(WasmRuntimeType::Wasmi));
        assert_eq!(WasmRuntimeType::from_str("unknown"), None);
    }

    #[test]
    fn test_wasm_runtime_type_display() {
        assert_eq!(WasmRuntimeType::Wasmtime.to_string(), "wasmtime");
        assert_eq!(WasmRuntimeType::Wasmi.to_string(), "wasmi");
    }

    #[test]
    fn test_wasm_runtime_type_description() {
        assert!(!WasmRuntimeType::Wasmtime.description().is_empty());
        assert!(!WasmRuntimeType::Wasmi.description().is_empty());
    }
}
