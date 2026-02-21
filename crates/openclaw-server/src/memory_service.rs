use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_memory::MemoryManager;

pub struct MemoryService {
    memory_manager: Arc<RwLock<Option<Arc<MemoryManager>>>>,
}

impl MemoryService {
    pub fn new() -> Self {
        Self {
            memory_manager: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn init(&self, memory: Arc<MemoryManager>) {
        *self.memory_manager.write().await = Some(memory);
    }

    pub async fn get_memory(&self) -> Option<Arc<MemoryManager>> {
        self.memory_manager.read().await.clone()
    }

    pub async fn is_initialized(&self) -> bool {
        self.memory_manager.read().await.is_some()
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

    #[tokio::test]
    async fn test_memory_service_new() {
        let service = MemoryService::new();
        assert!(!service.is_initialized().await);
    }

    #[tokio::test]
    async fn test_memory_service_init() {
        let service = MemoryService::new();
        
        let memory = Arc::new(MemoryManager::default());
        
        service.init(memory.clone()).await;
        
        assert!(service.is_initialized().await);
        assert!(service.get_memory().await.is_some());
    }

    #[tokio::test]
    async fn test_memory_service_getters() {
        let service = MemoryService::new();
        
        let memory = Arc::new(MemoryManager::default());
        
        service.init(memory.clone()).await;
        
        let retrieved_memory = service.get_memory().await;
        assert!(retrieved_memory.is_some());
    }
}
