//! OpenClaw Sandbox - 安全沙箱模块
//!
//! 提供 Docker/Podman 沙箱运行、权限管理、WASM 安全执行能力

pub mod capability;
pub mod credential;
pub mod docker;
pub mod endpoint;
pub mod executor;
pub mod leak_detector;
pub mod manager;
pub mod permission;
pub mod podman;
pub mod rate_limit;
pub mod sandbox;
pub mod types;
pub mod wasm;
pub mod wasm_runtime;

pub use capability::*;
pub use credential::*;
pub use docker::*;
pub use endpoint::*;
pub use leak_detector::*;
pub use manager::*;
pub use permission::*;
pub use podman::*;
pub use rate_limit::*;
pub use sandbox::SandboxManager as SandboxManagerImpl;
pub use types::*;
pub use wasm::*;
pub use wasm_runtime::{
    WasmRuntimeType, WasmRuntimeConfig, create_wasm_runtime,
    WasmRuntime, WasmModule, WasmExecutionInput, WasmExecutionResult, WasmError,
};

pub use capability::MemoryCapabilityService;
pub use credential::MemoryCredentialService;
pub use endpoint::MemoryEndpointAllowlist;
pub use leak_detector::{RegexLeakDetector, create_default_detector};
pub use rate_limit::MemoryRateLimiter;

pub use executor::{ExecutionContext, ExecutionResult, SecureToolExecutor, ExecutorError, SecurityCheckResults, ToolConfig};
