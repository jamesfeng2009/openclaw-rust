use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_channels::{Channel, ChannelManager, ChannelMessage, ChannelType, SendMessage};

pub struct ChannelService {
    manager: Arc<RwLock<ChannelManager>>,
}

impl ChannelService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(ChannelManager::new())),
        }
    }

    pub async fn register_channel(&self, name: String, channel: Arc<RwLock<dyn Channel>>) {
        let mut manager = self.manager.write().await;
        manager.register_channel(name, channel).await;
    }

    pub async fn unregister_channel(&self, name: &str) {
        let mut manager = self.manager.write().await;
        manager.unregister_channel(name).await;
    }

    pub async fn get_channel(&self, name: &str) -> Option<Arc<RwLock<dyn Channel>>> {
        let manager = self.manager.read().await;
        manager.get_channel(name).await
    }

    pub async fn list_channels(&self) -> Vec<String> {
        let manager = self.manager.read().await;
        manager.list_channels().await
    }

    pub async fn start_all(&self) -> openclaw_core::Result<()> {
        let manager = self.manager.read().await;
        manager.start_all().await
    }

    pub async fn stop_all(&self) -> openclaw_core::Result<()> {
        let manager = self.manager.read().await;
        manager.stop_all().await
    }

    pub async fn send_message(
        &self,
        channel_name: &str,
        message: SendMessage,
    ) -> openclaw_core::Result<ChannelMessage> {
        let manager = self.manager.read().await;
        manager.send_to_channel(channel_name, message).await
    }

    pub async fn broadcast(
        &self,
        message: SendMessage,
    ) -> openclaw_core::Result<Vec<ChannelMessage>> {
        let manager = self.manager.read().await;
        manager.broadcast(message).await
    }

    pub async fn health_check(&self) -> std::collections::HashMap<String, bool> {
        let manager = self.manager.read().await;
        manager.health_check_all().await
    }
}

impl Default for ChannelService {
    fn default() -> Self {
        Self::new()
    }
}
