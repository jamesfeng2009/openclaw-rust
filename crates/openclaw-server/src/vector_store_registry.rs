use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_core::config::{LanceDbConfig, MilvusConfig, QdrantConfig, VectorBackend};
use openclaw_vector::{init_default_factories, get_factory, BackendConfig, StoreBackend, VectorStore};

pub type VectorStoreCreator = dyn Send + Sync + Fn() -> Arc<dyn VectorStore>;

pub struct VectorStoreRegistry {
    stores: Arc<RwLock<HashMap<String, Arc<dyn VectorStore>>>>,
}

impl VectorStoreRegistry {
    pub fn new() -> Self {
        init_default_factories();
        Self {
            stores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, name: String, store: Arc<dyn VectorStore>) {
        let mut stores = self.stores.write().await;
        stores.insert(name, store);
    }

    pub async fn create(&self, name: &str) -> Option<Arc<dyn VectorStore>> {
        let stores = self.stores.read().await;
        stores.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<String> {
        let stores = self.stores.read().await;
        stores.keys().cloned().collect()
    }

    pub async fn register_defaults(&self, enabled_backends: Option<Vec<String>>) {
        use openclaw_vector::VectorStore;
        
        let backends = enabled_backends.unwrap_or_else(|| vec!["memory".to_string()]);

        for name in &backends {
            if let Some(factory) = get_factory(name) {
                let config = BackendConfig {
                    name: name.clone(),
                    ..Default::default()
                };
                match factory.create(&config).await {
                    Ok(store) => {
                        self.register(name.clone(), store).await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create vector store '{}': {}", name, e);
                    }
                }
            } else {
                tracing::warn!("No factory registered for vector store '{}'", name);
            }
        }
    }

    pub async fn register_from_config(
        &self,
        backends: &[String],
        backend_type: &VectorBackend,
        qdrant_config: Option<&QdrantConfig>,
        lancedb_config: Option<&LanceDbConfig>,
        milvus_config: Option<&MilvusConfig>,
    ) {
        for name in backends {
            if let Some(factory) = get_factory(name) {
                let config = self.build_backend_config(name, backend_type, qdrant_config, lancedb_config, milvus_config);
                match factory.create(&config).await {
                    Ok(store) => {
                        self.register(name.clone(), store).await;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create vector store '{}': {}", name, e);
                    }
                }
            } else {
                tracing::warn!("No factory registered for vector store '{}'", name);
            }
        }
    }

    fn build_backend_config(
        &self,
        name: &str,
        backend_type: &VectorBackend,
        qdrant_config: Option<&QdrantConfig>,
        lancedb_config: Option<&LanceDbConfig>,
        milvus_config: Option<&MilvusConfig>,
    ) -> BackendConfig {
        let mut config = BackendConfig {
            name: name.to_string(),
            ..Default::default()
        };

        match backend_type {
            VectorBackend::Qdrant => {
                if let Some(qdrant) = qdrant_config {
                    config.url = Some(qdrant.url.clone());
                    config.collection = Some(qdrant.collection.clone());
                    config.api_key = qdrant.api_key.clone();
                }
            }
            VectorBackend::LanceDB => {
                if let Some(lancedb) = lancedb_config {
                    config.path = Some(lancedb.path.clone());
                }
            }
            VectorBackend::Milvus => {
                if let Some(milvus) = milvus_config {
                    config.url = Some(milvus.url.clone());
                    config.collection = Some(milvus.collection.clone());
                    config.dimensions = milvus.dimension;
                }
            }
            _ => {}
        }

        config
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

        let store = Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn VectorStore>;
        registry.register("test".to_string(), store).await;

        let list = registry.list().await;
        assert!(list.contains(&"test".to_string()));

        let result = registry.create("test").await;
        assert!(result.is_some());
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

    #[cfg(feature = "lancedb")]
    #[test]
    fn test_create_vector_store_lancedb() {
        let store = create_vector_store(&StoreBackend::LanceDB {
            path: std::path::PathBuf::from("/tmp/test"),
        });
        assert!(store.is_some());
    }

    #[test]
    fn test_create_vector_store_unknown() {
        let store = create_vector_store(&StoreBackend::SQLite {
            path: std::path::PathBuf::from("/tmp/test"),
            table: "test".to_string(),
        });
        assert!(store.is_none());
    }

    #[tokio::test]
    async fn test_register_defaults_with_memory_only() {
        let registry = VectorStoreRegistry::new();
        registry
            .register_defaults(Some(vec!["memory".to_string()]))
            .await;

        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_register_defaults_empty_list() {
        let registry = VectorStoreRegistry::new();
        registry.register_defaults(Some(vec![])).await;

        let list = registry.list().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_register_custom_backend() {
        let registry = VectorStoreRegistry::new();

        let store = Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn VectorStore>;
        registry.register("custom".to_string(), store).await;

        let list = registry.list().await;
        assert!(list.contains(&"custom".to_string()));

        let result = registry.create("custom").await;
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_register_from_config_with_memory() {
        let registry = VectorStoreRegistry::new();

        registry
            .register_from_config(
                &["memory".to_string()],
                &VectorBackend::LanceDB,
                None,
                None,
                None,
            )
            .await;

        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_register_from_config_with_qdrant() {
        let registry = VectorStoreRegistry::new();

        let qdrant_config = QdrantConfig {
            url: "http://localhost:6333".to_string(),
            collection: "test".to_string(),
            api_key: None,
        };

        registry
            .register_from_config(
                &["memory".to_string()],
                &VectorBackend::Qdrant,
                Some(&qdrant_config),
                None,
                None,
            )
            .await;

        let list = registry.list().await;
        assert!(list.contains(&"memory".to_string()));
    }

    #[tokio::test]
    async fn test_build_backend_config_qdrant() {
        let registry = VectorStoreRegistry::new();

        let qdrant_config = QdrantConfig {
            url: "http://localhost:6333".to_string(),
            collection: "test_collection".to_string(),
            api_key: Some("test_key".to_string()),
        };

        let config = registry.build_backend_config(
            "qdrant",
            &VectorBackend::Qdrant,
            Some(&qdrant_config),
            None,
            None,
        );

        assert_eq!(config.name, "qdrant");
        assert_eq!(config.url, Some("http://localhost:6333".to_string()));
        assert_eq!(config.collection, Some("test_collection".to_string()));
        assert_eq!(config.api_key, Some("test_key".to_string()));
    }

    #[tokio::test]
    async fn test_build_backend_config_lancedb() {
        let registry = VectorStoreRegistry::new();

        let lancedb_config = LanceDbConfig {
            path: std::path::PathBuf::from("/tmp/lancedb"),
        };

        let config = registry.build_backend_config(
            "lancedb",
            &VectorBackend::LanceDB,
            None,
            Some(&lancedb_config),
            None,
        );

        assert_eq!(config.name, "lancedb");
        assert_eq!(config.path, Some(std::path::PathBuf::from("/tmp/lancedb")));
    }
}
