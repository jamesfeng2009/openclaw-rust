use std::collections::HashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait DeviceTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: serde_json::Value) -> DeviceToolResult;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToolResult {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

impl DeviceToolResult {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
    
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

pub struct DeviceToolRegistry {
    tools: HashMap<String, Box<dyn DeviceTool>>,
}

impl DeviceToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    pub fn register<T: DeviceTool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        self.tools.insert(name, Box::new(tool));
    }
    
    pub async fn execute(&self, name: &str, params: serde_json::Value) -> DeviceToolResult {
        match self.tools.get(name) {
            Some(tool) => tool.execute(params).await,
            None => DeviceToolResult::failure(format!("Tool '{}' not found", name)),
        }
    }
    
    pub fn list_tools(&self) -> Vec<DeviceToolInfo> {
        self.tools.values()
            .map(|tool| DeviceToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
            })
            .collect()
    }
    
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

impl Default for DeviceToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceToolInfo {
    pub name: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockTool {
        name: String,
        description: String,
    }
    
    impl MockTool {
        fn new(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
            }
        }
    }
    
    #[async_trait]
    impl DeviceTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }
        
        fn description(&self) -> &str {
            &self.description
        }
        
        async fn execute(&self, params: serde_json::Value) -> DeviceToolResult {
            DeviceToolResult::success(params)
        }
    }
    
    #[test]
    fn test_registry_creation() {
        let registry = DeviceToolRegistry::new();
        assert!(registry.list_tools().is_empty());
    }
    
    #[test]
    fn test_register_tool() {
        let mut registry = DeviceToolRegistry::new();
        registry.register(MockTool::new("test_tool", "A test tool"));
        
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }
    
    #[tokio::test]
    async fn test_execute_tool() {
        let mut registry = DeviceToolRegistry::new();
        registry.register(MockTool::new("test_tool", "A test tool"));
        
        let result = registry.execute("test_tool", serde_json::json!({"key": "value"})).await;
        assert!(result.success);
        assert!(result.data.is_some());
    }
    
    #[tokio::test]
    async fn test_execute_nonexistent_tool() {
        let registry = DeviceToolRegistry::new();
        
        let result = registry.execute("nonexistent", serde_json::json!({})).await;
        assert!(!result.success);
        assert!(result.error.is_some());
    }
    
    #[test]
    fn test_has_tool() {
        let mut registry = DeviceToolRegistry::new();
        registry.register(MockTool::new("test_tool", "A test tool"));
        
        assert!(registry.has_tool("test_tool"));
        assert!(!registry.has_tool("nonexistent"));
    }
    
    #[test]
    fn test_device_tool_result_success() {
        let result = DeviceToolResult::success(serde_json::json!({"key": "value"}));
        assert!(result.success);
        assert!(result.data.is_some());
        assert!(result.error.is_none());
    }
    
    #[test]
    fn test_device_tool_result_failure() {
        let result = DeviceToolResult::failure("Error message".to_string());
        assert!(!result.success);
        assert!(result.data.is_none());
        assert!(result.error.is_some());
    }
    
    #[test]
    fn test_multiple_tools() {
        let mut registry = DeviceToolRegistry::new();
        registry.register(MockTool::new("tool1", "Tool 1"));
        registry.register(MockTool::new("tool2", "Tool 2"));
        registry.register(MockTool::new("tool3", "Tool 3"));
        
        let tools = registry.list_tools();
        assert_eq!(tools.len(), 3);
    }
}
