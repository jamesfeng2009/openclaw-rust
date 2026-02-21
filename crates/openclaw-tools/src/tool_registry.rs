use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, args: serde_json::Value) -> openclaw_core::Result<serde_json::Value>;
}

#[derive(Clone)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, tool: Arc<dyn Tool>) {
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub async fn execute(&self, name: &str, args: serde_json::Value) -> openclaw_core::Result<serde_json::Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| openclaw_core::OpenClawError::Tool(format!("Tool not found: {}", name)))?;
        
        tool.execute(args).await
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestTool {
        name: String,
        description: String,
        execute_count: Arc<AtomicUsize>,
        should_panic: AtomicBool,
    }

    impl TestTool {
        fn new(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                execute_count: Arc::new(AtomicUsize::new(0)),
                should_panic: AtomicBool::new(false),
            }
        }

        fn with_panic(name: &str, description: &str) -> Self {
            Self {
                name: name.to_string(),
                description: description.to_string(),
                execute_count: Arc::new(AtomicUsize::new(0)),
                should_panic: AtomicBool::new(true),
            }
        }

        fn get_execute_count(&self) -> usize {
            self.execute_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            &self.description
        }

        async fn execute(&self, args: serde_json::Value) -> openclaw_core::Result<serde_json::Value> {
            self.execute_count.fetch_add(1, Ordering::SeqCst);
            
            if self.should_panic.load(Ordering::SeqCst) {
                return Err(openclaw_core::OpenClawError::Tool("Tool panicked".to_string()));
            }
            
            Ok(serde_json::json!({
                "tool": self.name,
                "args": args,
                "executed": true
            }))
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new();
        assert!(registry.list_tools().is_empty());
        assert!(!registry.has_tool("any"));
    }

    #[test]
    fn test_registry_register_single() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("test", "A test tool"));
        
        registry.register("test".to_string(), tool);
        
        assert!(registry.has_tool("test"));
        assert_eq!(registry.list_tools(), vec!["test"]);
    }

    #[test]
    fn test_registry_register_multiple() {
        let mut registry = ToolRegistry::new();
        
        registry.register("tool1".to_string(), Arc::new(TestTool::new("tool1", "Tool 1")));
        registry.register("tool2".to_string(), Arc::new(TestTool::new("tool2", "Tool 2")));
        registry.register("tool3".to_string(), Arc::new(TestTool::new("tool3", "Tool 3")));
        
        assert_eq!(registry.list_tools().len(), 3);
        assert!(registry.has_tool("tool1"));
        assert!(registry.has_tool("tool2"));
        assert!(registry.has_tool("tool3"));
    }

    #[test]
    fn test_registry_register_overwrite() {
        let mut registry = ToolRegistry::new();
        
        registry.register("test".to_string(), Arc::new(TestTool::new("test", "First")));
        registry.register("test".to_string(), Arc::new(TestTool::new("test", "Second")));
        
        assert_eq!(registry.list_tools().len(), 1);
    }

    #[test]
    fn test_registry_get_existing() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("test", "A test tool"));
        registry.register("test".to_string(), tool);
        
        let retrieved = registry.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test");
    }

    #[test]
    fn test_registry_get_nonexistent() {
        let registry = ToolRegistry::new();
        let retrieved = registry.get("nonexistent");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_registry_execute_success() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("echo", "Echo tool"));
        registry.register("echo".to_string(), tool);
        
        let args = serde_json::json!({"message": "hello", "count": 42});
        let result = registry.execute("echo", args).await;
        
        assert!(result.is_ok());
        let value = result.unwrap();
        assert_eq!(value["tool"], "echo");
        assert_eq!(value["args"]["message"], "hello");
        assert_eq!(value["args"]["count"], 42);
        assert_eq!(value["executed"], true);
    }

    #[tokio::test]
    async fn test_registry_execute_not_found() {
        let registry = ToolRegistry::new();
        let args = serde_json::json!({});
        let result = registry.execute("nonexistent", args).await;
        
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Tool not found"));
    }

    #[tokio::test]
    async fn test_registry_execute_multiple_times() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("counter", "Counter tool"));
        let tool_clone = tool.clone();
        registry.register("counter".to_string(), tool);
        
        for i in 0..5 {
            let args = serde_json::json!({"iteration": i});
            registry.execute("counter", args).await.unwrap();
        }
        
        assert_eq!(tool_clone.get_execute_count(), 5);
    }

    #[tokio::test]
    async fn test_registry_execute_failure_propagates() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::with_panic("failing", "Failing tool"));
        registry.register("failing".to_string(), tool);
        
        let args = serde_json::json!({});
        let result = registry.execute("failing", args).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_empty_execute() {
        let registry = ToolRegistry::new();
        let result = registry.execute("empty", serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        assert!(registry.list_tools().is_empty());
    }

    #[tokio::test]
    async fn test_registry_complex_args() {
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("complex", "Complex tool"));
        registry.register("complex".to_string(), tool);
        
        let complex_args = serde_json::json!({
            "nested": {
                "deep": {
                    "value": "test"
                }
            },
            "array": [1, 2, 3],
            "null": null,
            "boolean": true
        });
        
        let result = registry.execute("complex", complex_args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tool_name_and_description() {
        let tool = TestTool::new("my_tool", "This is my tool description");
        assert_eq!(tool.name(), "my_tool");
        assert_eq!(tool.description(), "This is my tool description");
    }

    #[tokio::test]
    async fn test_concurrent_execution() {
        use tokio::task;
        
        let mut registry = ToolRegistry::new();
        let tool = Arc::new(TestTool::new("concurrent", "Concurrent tool"));
        let tool_clone = tool.clone();
        registry.register("concurrent".to_string(), tool);
        
        let mut handles = vec![];
        for i in 0..10 {
            let registry_clone = registry.clone();
            let handle = task::spawn(async move {
                registry_clone.execute("concurrent", serde_json::json!({"i": i})).await
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        
        assert_eq!(tool_clone.get_execute_count(), 10);
    }
}
