//! WASM 运行时集成测试

#[cfg(all(test, feature = "wasm-runtime-wasmi"))]
mod integration_tests {
    use crate::wasm_runtime::{
        create_wasm_runtime, WasmExecutionInput, WasmRuntimeConfig, WasmRuntimeType,
    };

    #[tokio::test]
    async fn test_create_wasmi_runtime() {
        let config = WasmRuntimeConfig::default();
        let result = create_wasm_runtime(WasmRuntimeType::Wasmi, &config);
        
        assert!(result.is_ok());
        let runtime = result.unwrap();
        assert_eq!(runtime.runtime_type(), WasmRuntimeType::Wasmi);
        assert!(runtime.is_available());
    }

    #[tokio::test]
    async fn test_wasmi_load_invalid_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = create_wasm_runtime(WasmRuntimeType::Wasmi, &config).unwrap();

        let invalid_wasm = b"invalid wasm data";
        let result = runtime.load_module(invalid_wasm, "test_module").await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wasmi_execution_with_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = create_wasm_runtime(WasmRuntimeType::Wasmi, &config).unwrap();

        let module = crate::wasm_runtime::WasmModule {
            name: "test".to_string(),
            size_bytes: 100,
        };

        let input = WasmExecutionInput {
            function: "run".to_string(),
            params: serde_json::json!({"test": true}),
        };

        let result = runtime.execute(&module, &input).await;
        assert!(result.is_ok());
    }
}

#[cfg(all(test, feature = "wasm-runtime-wasmtime"))]
mod wasmtime_tests {
    use crate::wasm_runtime::{
        create_wasm_runtime, WasmExecutionInput, WasmRuntimeConfig, WasmRuntimeType,
    };

    #[tokio::test]
    async fn test_create_wasmtime_runtime() {
        let config = WasmRuntimeConfig::default();
        let result = create_wasm_runtime(WasmRuntimeType::Wasmtime, &config);
        
        assert!(result.is_ok());
        let runtime = result.unwrap();
        assert_eq!(runtime.runtime_type(), WasmRuntimeType::Wasmtime);
        assert!(runtime.is_available());
    }

    #[tokio::test]
    async fn test_wasmtime_load_invalid_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = create_wasm_runtime(WasmRuntimeType::Wasmtime, &config).unwrap();

        let invalid_wasm = b"invalid wasm data";
        let result = runtime.load_module(invalid_wasm, "test_module").await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wasmtime_execution_with_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = create_wasm_runtime(WasmRuntimeType::Wasmtime, &config).unwrap();

        let module = crate::wasm_runtime::WasmModule {
            name: "test".to_string(),
            size_bytes: 100,
        };

        let input = WasmExecutionInput {
            function: "run".to_string(),
            params: serde_json::json!({"test": true}),
        };

        let result = runtime.execute(&module, &input).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success || result.error.is_some());
    }
}

#[cfg(test)]
mod common_tests {
    use crate::wasm_runtime::{WasmRuntimeConfig, WasmRuntimeType};

    #[test]
    fn test_runtime_type_characteristics() {
        let wasmtime = WasmRuntimeType::Wasmtime;
        let wasmi = WasmRuntimeType::Wasmi;

        assert_eq!(wasmtime.to_string(), "wasmtime");
        assert_eq!(wasmi.to_string(), "wasmi");

        assert!(!wasmtime.description().is_empty());
        assert!(!wasmi.description().is_empty());

        assert_ne!(wasmtime, wasmi);
    }

    #[test]
    fn test_config_different_limits() {
        let small_config = WasmRuntimeConfig {
            memory_limit_mb: 8,
            compute_limit: 512,
            timeout_secs: 60,
            allow_network: true,
            allowed_functions: vec![],
        };

        let large_config = WasmRuntimeConfig {
            memory_limit_mb: 256,
            compute_limit: 512,
            timeout_secs: 60,
            allow_network: true,
            allowed_functions: vec![],
        };

        assert_eq!(small_config.memory_limit_mb, 8);
        assert_eq!(large_config.memory_limit_mb, 256);
    }

    #[test]
    fn test_runtime_type_from_str() {
        assert_eq!(WasmRuntimeType::from_str("wasmtime"), Some(WasmRuntimeType::Wasmtime));
        assert_eq!(WasmRuntimeType::from_str("Wasmtime"), Some(WasmRuntimeType::Wasmtime));
        assert_eq!(WasmRuntimeType::from_str("wasmi"), Some(WasmRuntimeType::Wasmi));
        assert_eq!(WasmRuntimeType::from_str("unknown"), None);
    }
}
