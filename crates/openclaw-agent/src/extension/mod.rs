//! Extension System - 热重载扩展系统
//!
//! 提供运行时动态加载扩展的能力：
//! - Priority: 扩展优先级定义
//! - Extension: 扩展 trait
//! - ExtensionMeta: 扩展元数据
//! - ExtensionSource: 扩展来源
//! - ExtensionRegistry: 扩展注册表

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Priority(pub u8);

impl Priority {
    pub const SYSTEM: Priority = Priority(100);
    pub const HIGH: Priority = Priority(75);
    pub const NORMAL: Priority = Priority(50);
    pub const LOW: Priority = Priority(25);
    pub const USER: Priority = Priority(10);
}

impl Default for Priority {
    fn default() -> Self {
        Priority::NORMAL
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub priority: Priority,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
}

impl ExtensionMeta {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            version: version.into(),
            description: None,
            author: None,
            priority: Priority::default(),
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionSource {
    Builtin,
    Local,
    Remote,
    Wasm,
}

#[async_trait]
pub trait Extension: Send + Sync {
    fn meta(&self) -> &ExtensionMeta;
    fn source(&self) -> ExtensionSource;
    async fn initialize(&self) -> Result<(), ExtensionError>;
    async fn shutdown(&self) -> Result<(), ExtensionError>;
    async fn call(&self, method: &str, args: serde_json::Value) -> Result<serde_json::Value, ExtensionError>;
}

#[derive(Debug, Clone)]
pub enum ExtensionError {
    NotFound(String),
    AlreadyExists(String),
    InitializationFailed(String),
    CallFailed(String),
    DependencyMissing(String),
    VersionConflict(String),
}

impl std::fmt::Display for ExtensionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtensionError::NotFound(id) => write!(f, "Extension not found: {}", id),
            ExtensionError::AlreadyExists(id) => write!(f, "Extension already exists: {}", id),
            ExtensionError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            ExtensionError::CallFailed(msg) => write!(f, "Call failed: {}", msg),
            ExtensionError::DependencyMissing(dep) => write!(f, "Missing dependency: {}", dep),
            ExtensionError::VersionConflict(msg) => write!(f, "Version conflict: {}", msg),
        }
    }
}

impl std::error::Error for ExtensionError {}

pub type ExtensionResult<T> = Result<T, ExtensionError>;

pub struct ExtensionRegistry {
    extensions: Arc<RwLock<HashMap<String, Arc<dyn Extension>>>>,
    watchers: Arc<RwLock<Vec<PathBuf>>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self {
            extensions: Arc::new(RwLock::new(HashMap::new())),
            watchers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, extension: Arc<dyn Extension>) -> ExtensionResult<()> {
        let meta = extension.meta().clone();
        let id = meta.id.clone();
        let name = meta.name.clone();

        let mut extensions = self.extensions.write().await;
        
        if extensions.contains_key(&id) {
            return Err(ExtensionError::AlreadyExists(id.clone()));
        }

        for ext in extensions.values() {
            if ext.meta().name == name {
                return Err(ExtensionError::AlreadyExists(format!("Extension with name '{}' already exists", name)));
            }
        }

        extension.initialize().await.map_err(|e| {
            ExtensionError::InitializationFailed(e.to_string())
        })?;

        extensions.insert(id.clone(), extension);
        
        tracing::info!("Registered extension: {} v{}", meta.name, meta.version);
        
        Ok(())
    }

    pub async fn unregister(&self, id: &str) -> ExtensionResult<()> {
        let extension = {
            let mut extensions = self.extensions.write().await;
            extensions.remove(id).ok_or_else(|| ExtensionError::NotFound(id.to_string()))?
        };

        extension.shutdown().await.map_err(|e| {
            ExtensionError::CallFailed(e.to_string())
        })?;

        tracing::info!("Unregistered extension: {}", id);
        
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Option<Arc<dyn Extension>> {
        let extensions = self.extensions.read().await;
        extensions.get(id).cloned()
    }

    pub async fn list(&self) -> Vec<ExtensionMeta> {
        let extensions = self.extensions.read().await;
        extensions.values().map(|e| e.meta().clone()).collect()
    }

    pub async fn find_by_name(&self, name: &str) -> Option<Arc<dyn Extension>> {
        let extensions = self.extensions.read().await;
        extensions.values().find(|e| e.meta().name == name).cloned()
    }

    pub async fn find_by_tag(&self, tag: &str) -> Vec<Arc<dyn Extension>> {
        let extensions = self.extensions.read().await;
        extensions
            .values()
            .filter(|e| e.meta().tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    pub async fn resolve_conflicts(&self, new_meta: &ExtensionMeta) -> ExtensionResult<()> {
        let extensions = self.extensions.read().await;
        
        let mut existing_ext: Option<ExtensionMeta> = None;
        
        for ext in extensions.values() {
            let existing = ext.meta().clone();
            
            if existing.name == new_meta.name && existing.version != new_meta.version {
                return Err(ExtensionError::VersionConflict(format!(
                    "Version conflict for {}: existing {} vs new {}",
                    existing.name, existing.version, new_meta.version
                )));
            }

            if existing.name == new_meta.name {
                existing_ext = Some(existing);
            }

            for dep in &new_meta.dependencies {
                if !extensions.contains_key(dep) {
                    return Err(ExtensionError::DependencyMissing(dep.clone()));
                }
            }
        }

        if let Some(ref existing) = existing_ext {
            if new_meta.priority <= existing.priority {
                return Err(ExtensionError::AlreadyExists(existing.id.clone()));
            }
        }

        Ok(())
    }

    pub async fn add_watcher(&self, path: PathBuf) {
        let mut watchers = self.watchers.write().await;
        if !watchers.contains(&path) {
            watchers.push(path);
        }
    }

    pub async fn get_watchers(&self) -> Vec<PathBuf> {
        let watchers = self.watchers.read().await;
        watchers.clone()
    }
}

impl Default for ExtensionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExtension {
        meta: ExtensionMeta,
    }

    impl TestExtension {
        fn new(name: &str, version: &str) -> Self {
            Self {
                meta: ExtensionMeta::new(name, version),
            }
        }
    }

    #[async_trait]
    impl Extension for TestExtension {
        fn meta(&self) -> &ExtensionMeta {
            &self.meta
        }

        fn source(&self) -> ExtensionSource {
            ExtensionSource::Builtin
        }

        async fn initialize(&self) -> Result<(), ExtensionError> {
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), ExtensionError> {
            Ok(())
        }

        async fn call(&self, _method: &str, _args: serde_json::Value) -> Result<serde_json::Value, ExtensionError> {
            Ok(serde_json::Value::Null)
        }
    }

    #[tokio::test]
    async fn test_register_extension() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(TestExtension::new("test", "1.0.0"));
        
        let result = registry.register(ext.clone()).await;
        assert!(result.is_ok());
        
        let retrieved = registry.get(&ext.meta().id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_duplicate_registration() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(TestExtension::new("test", "1.0.0"));
        
        registry.register(ext.clone()).await.unwrap();
        
        let ext2 = Arc::new(TestExtension::new("test", "1.0.0"));
        let result = registry.register(ext2).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister_extension() {
        let registry = ExtensionRegistry::new();
        let ext = Arc::new(TestExtension::new("test", "1.0.0"));
        
        registry.register(ext.clone()).await.unwrap();
        
        let result = registry.unregister(&ext.meta().id).await;
        assert!(result.is_ok());
        
        let retrieved = registry.get(&ext.meta().id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_list_extensions() {
        let registry = ExtensionRegistry::new();
        
        let ext1 = Arc::new(TestExtension::new("ext1", "1.0.0"));
        let ext2 = Arc::new(TestExtension::new("ext2", "2.0.0"));
        
        registry.register(ext1).await.unwrap();
        registry.register(ext2).await.unwrap();
        
        let list = registry.list().await;
        assert_eq!(list.len(), 2);
    }
}
