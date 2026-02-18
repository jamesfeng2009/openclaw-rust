use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use openclaw_vector::{VectorStore, StoreBackend};

pub type VectorStoreCreator = dyn Send + Sync + Fn() -> Arc<dyn VectorStore>;

pub struct VectorStoreRegistry {
    creators: Arc<RwLock<HashMap<String, Box<VectorStoreCreator>>>>,
}

impl VectorStoreRegistry {
    pub fn new() -> Self {
        Self {
            creators: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register<F>(&self, name: String, creator: F)
    where
        F: Send + Sync + Fn() -> Arc<dyn VectorStore> + 'static,
    {
        let mut creators = self.creators.write().await;
        creators.insert(name, Box::new(creator));
    }

    pub async fn create(&self, name: &str) -> Option<Arc<dyn VectorStore>> {
        let creators = self.creators.read().await;
        creators.get(name).map(|f| f())
    }

    pub async fn list(&self) -> Vec<String> {
        let creators = self.creators.read().await;
        creators.keys().cloned().collect()
    }

    pub async fn register_defaults(&self) {
        use openclaw_vector::MemoryStore;
        
        self.register("memory".to_string(), || {
            Arc::new(MemoryStore::new()) as Arc<dyn VectorStore>
        }).await;
    }
}

impl Default for VectorStoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_vector_store(backend: &StoreBackend) -> Option<Arc<dyn VectorStore>> {
    match backend {
        StoreBackend::Memory => {
            Some(Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn VectorStore>)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_registry_new() {
        let registry = VectorStoreRegistry::new();
        let list = registry.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_vector_store_registry_register_and_create() {
        let registry = VectorStoreRegistry::new();
        
        registry.register("test".to_string(), || {
            Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn VectorStore>
        }).await;
        
        let list = registry.list().await;
        assert!(list.contains(&"test".to_string()));
        
        let store = registry.create("test").await;
        assert!(store.is_some());
    }

    #[tokio::test]
    async fn test_vector_store_registry_create_nonexistent() {
        let registry = VectorStoreRegistry::new();
        
        let store = registry.create("nonexistent").await;
        assert!(store.is_none());
    }

    #[tokio::test]
    async fn test_vector_store_registry_register_defaults() {
        let registry = VectorStoreRegistry::new();
        
        registry.register_defaults().await;
        
        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
        
        let store = registry.create("memory").await;
        assert!(store.is_some());
    }

    #[test]
    fn test_create_vector_store_memory() {
        let store = create_vector_store(&StoreBackend::Memory);
        assert!(store.is_some());
    }

    #[test]
    fn test_create_vector_store_unknown() {
        let store = create_vector_store(&StoreBackend::LanceDB { 
            path: std::path::PathBuf::from("/tmp/test") 
        });
        assert!(store.is_none());
    }
}
