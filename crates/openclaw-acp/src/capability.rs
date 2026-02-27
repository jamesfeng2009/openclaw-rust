//! ACP Capability System
//!
//! Provides capability registration and discovery for agents.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub parameters: Vec<CapabilityParam>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
}

impl Capability {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: Vec::new(),
            examples: Vec::new(),
        }
    }

    pub fn with_param(mut self, param: CapabilityParam) -> Self {
        self.parameters.push(param);
        self
    }

    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }
}

/// Capability Registry
pub struct CapabilityRegistry {
    capabilities: Arc<RwLock<HashMap<String, Capability>>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, capability: Capability) {
        let mut capabilities = self.capabilities.write().await;
        capabilities.insert(capability.name.clone(), capability);
    }

    pub async fn get(&self, name: &str) -> Option<Capability> {
        let capabilities = self.capabilities.read().await;
        capabilities.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<Capability> {
        let capabilities = self.capabilities.read().await;
        capabilities.values().cloned().collect()
    }

    pub async fn search(&self, keyword: &str) -> Vec<Capability> {
        let capabilities = self.capabilities.read().await;
        capabilities
            .values()
            .filter(|c| {
                c.name.to_lowercase().contains(&keyword.to_lowercase())
                    || c.description.to_lowercase().contains(&keyword.to_lowercase())
            })
            .cloned()
            .collect()
    }

    pub async fn unregister(&self, name: &str) -> bool {
        let mut capabilities = self.capabilities.write().await;
        capabilities.remove(name).is_some()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Predefined capabilities
pub mod predefined {
    use super::*;

    pub fn code_execution() -> Capability {
        Capability::new("code_execution", "Execute code in a sandboxed environment")
            .with_param(CapabilityParam {
                name: "code".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Code to execute".to_string(),
            })
            .with_param(CapabilityParam {
                name: "language".to_string(),
                param_type: "string".to_string(),
                required: false,
                description: "Programming language".to_string(),
            })
    }

    pub fn code_review() -> Capability {
        Capability::new("code_review", "Review code for bugs, security issues, and best practices")
            .with_param(CapabilityParam {
                name: "code".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Code to review".to_string(),
            })
            .with_param(CapabilityParam {
                name: "language".to_string(),
                param_type: "string".to_string(),
                required: false,
                description: "Programming language".to_string(),
            })
    }

    pub fn research() -> Capability {
        Capability::new("research", "Search and summarize information")
            .with_param(CapabilityParam {
                name: "query".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Research query".to_string(),
            })
    }

    pub fn text_generation() -> Capability {
        Capability::new("text_generation", "Generate text content")
            .with_param(CapabilityParam {
                name: "prompt".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Prompt for text generation".to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capability_registry() {
        let registry = CapabilityRegistry::new();
        
        let cap = Capability::new("test", "Test capability");
        registry.register(cap).await;
        
        let found = registry.get("test").await;
        assert!(found.is_some());
    }
}
