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

    pub async fn register_defaults(&self, enabled_backends: Option<Vec<String>>) {
        use openclaw_vector::MemoryStore;
        
        self.register("memory".to_string(), || {
            Arc::new(MemoryStore::new()) as Arc<dyn VectorStore>
        }).await;
        
        let backends = enabled_backends.unwrap_or_default();
        
        if backends.contains(&"lancedb".to_string()) {
            self.register_lancedb().await;
        }
        
        if backends.contains(&"qdrant".to_string()) {
            self.register_qdrant().await;
        }
        
        if backends.contains(&"pgvector".to_string()) {
            self.register_pgvector().await;
        }
    }

    async fn register_lancedb(&self) {
        #[cfg(feature = "lancedb")]
        {
            use openclaw_vector::LanceDbStore;
            self.register("lancedb".to_string(), || {
                Arc::new(LanceDbStore::new(&std::path::PathBuf::from("./data/lancedb"), "default").unwrap()) as Arc<dyn VectorStore>
            }).await;
        }
    }

    async fn register_qdrant(&self) {
        #[cfg(feature = "qdrant")]
        {
            use openclaw_vector::QdrantStore;
            let store = QdrantStore::new("http://localhost:6334", "default", 384, None);
            match store {
                Ok(s) => {
                    self.register("qdrant".to_string(), move || {
                        Arc::new(s.clone()) as Arc<dyn VectorStore>
                    }).await;
                }
                Err(_) => {}
            }
        }
    }

    async fn register_pgvector(&self) {
        #[cfg(feature = "pgvector")]
        {
            use openclaw_vector::PgVectorStore;
            self.register("pgvector".to_string(), || {
                Arc::new(PgVectorStore::new("postgres://localhost:5432/openclaw", "vectors", 384).unwrap()) as Arc<dyn VectorStore>
            }).await;
        }
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
        
        registry.register_defaults(None).await;
        
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

    #[tokio::test]
    async fn test_register_defaults_with_memory_only() {
        let registry = VectorStoreRegistry::new();
        registry.register_defaults(Some(vec!["memory".to_string()])).await;
        
        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_register_defaults_empty_list() {
        let registry = VectorStoreRegistry::new();
        registry.register_defaults(Some(vec![])).await;
        
        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_register_custom_backend() {
        let registry = VectorStoreRegistry::new();
        
        registry.register("custom".to_string(), || {
            Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn VectorStore>
        }).await;
        
        let list = registry.list().await;
        assert!(list.contains(&"custom".to_string()));
        
        let store = registry.create("custom").await;
        assert!(store.is_some());
    }
}
