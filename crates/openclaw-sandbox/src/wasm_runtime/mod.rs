//! WASM 运行时抽象层
//!
//! 提供统一的 trait 接口，支持多种 WASM 运行时 (wasmtime, wasmi)

pub mod runtime_trait;
pub mod config;

#[cfg(feature = "wasm-runtime-wasmtime")]
pub mod wasmtime_runtime;

#[cfg(feature = "wasm-runtime-wasmi")]
pub mod wasmi_runtime;

pub mod integration_tests;

pub use runtime_trait::{WasmRuntime, WasmModule, WasmExecutionInput, WasmExecutionResult, WasmError, create_wasm_runtime, WasmRuntimeType};
pub use config::WasmRuntimeConfig;
