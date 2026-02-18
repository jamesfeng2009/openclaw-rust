use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_memory::MemoryManager;
use openclaw_vector::VectorStore;

pub struct MemoryService {
    memory_manager: Arc<RwLock<Option<Arc<MemoryManager>>>>,
    vector_store: Arc<RwLock<Option<Arc<dyn VectorStore>>>>,
}

impl MemoryService {
    pub fn new() -> Self {
        Self {
            memory_manager: Arc::new(RwLock::new(None)),
            vector_store: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn init(&self, memory: Arc<MemoryManager>, vector: Arc<dyn VectorStore>) {
        *self.memory_manager.write().await = Some(memory);
        *self.vector_store.write().await = Some(vector);
    }

    pub async fn get_memory(&self) -> Option<Arc<MemoryManager>> {
        self.memory_manager.read().await.clone()
    }

    pub async fn get_vector_store(&self) -> Option<Arc<dyn VectorStore>> {
        self.vector_store.read().await.clone()
    }

    pub async fn is_initialized(&self) -> bool {
        self.memory_manager.read().await.is_some() && self.vector_store.read().await.is_some()
    }
}

impl Default for MemoryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_vector::MemoryStore;

    #[tokio::test]
    async fn test_memory_service_new() {
        let service = MemoryService::new();
        assert!(!service.is_initialized().await);
    }

    #[tokio::test]
    async fn test_memory_service_init() {
        let service = MemoryService::new();
        
        let memory = Arc::new(MemoryManager::default());
        let vector: Arc<dyn VectorStore> = Arc::new(MemoryStore::new());
        
        service.init(memory.clone(), vector.clone()).await;
        
        assert!(service.is_initialized().await);
        assert!(service.get_memory().await.is_some());
        assert!(service.get_vector_store().await.is_some());
    }

    #[tokio::test]
    async fn test_memory_service_getters() {
        let service = MemoryService::new();
        
        let memory = Arc::new(MemoryManager::default());
        let vector: Arc<dyn VectorStore> = Arc::new(MemoryStore::new());
        
        service.init(memory.clone(), vector.clone()).await;
        
        let retrieved_memory = service.get_memory().await;
        assert!(retrieved_memory.is_some());
        
        let retrieved_vector = service.get_vector_store().await;
        assert!(retrieved_vector.is_some());
    }
}
