use std::sync::Arc;
use tokio::sync::RwLock;

pub struct VoiceService {
    enabled: Arc<RwLock<bool>>,
}

impl VoiceService {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    pub async fn toggle(&self) -> bool {
        let mut enabled = self.enabled.write().await;
        *enabled = !*enabled;
        *enabled
    }
}

impl Default for VoiceService {
    fn default() -> Self {
        Self::new()
    }
}
