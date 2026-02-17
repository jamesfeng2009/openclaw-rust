use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_channels::{Channel, ChannelMessage, SendMessage};
use openclaw_core::Result;

pub struct AgentChannels {
    manager: Arc<ChannelManager>,
    enabled: Arc<RwLock<bool>>,
}

pub struct ChannelManager {
    channels: Arc<RwLock<std::collections::HashMap<String, Arc<RwLock<dyn Channel>>>>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn register_channel(&self, name: String, channel: Arc<RwLock<dyn Channel>>) {
        let mut channels = self.channels.write().await;
        channels.insert(name, channel);
    }

    pub async fn unregister_channel(&self, name: &str) {
        let mut channels = self.channels.write().await;
        channels.remove(name);
    }

    pub async fn get_channel(&self, name: &str) -> Option<Arc<RwLock<dyn Channel>>> {
        let channels = self.channels.read().await;
        channels.get(name).cloned()
    }

    pub async fn list_channels(&self) -> Vec<String> {
        let channels = self.channels.read().await;
        channels.keys().cloned().collect()
    }

    pub async fn start_all(&self) -> Result<()> {
        let mut channels = self.channels.write().await;
        for (name, channel) in channels.iter_mut() {
            tracing::info!("Starting channel: {}", name);
            channel.write().await.start().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        let mut channels = self.channels.write().await;
        for (name, channel) in channels.iter_mut() {
            tracing::info!("Stopping channel: {}", name);
            channel.write().await.stop().await?;
        }
        Ok(())
    }

    pub async fn send_to_channel(
        &self,
        channel_name: &str,
        message: SendMessage,
    ) -> Result<ChannelMessage> {
        let channel = {
            let channels = self.channels.read().await;
            channels.get(channel_name).cloned().ok_or_else(|| {
                openclaw_core::OpenClawError::Config(format!("Channel not found: {}", channel_name))
            })?
        };

        channel.write().await.send(message).await
    }

    pub async fn broadcast(&self, message: SendMessage) -> Result<Vec<ChannelMessage>> {
        let channels = self.channels.read().await;
        let mut results = Vec::new();

        for (name, channel) in channels.iter() {
            match channel.write().await.send(message.clone()).await {
                Ok(msg) => results.push(msg),
                Err(e) => tracing::warn!("Failed to send to channel {}: {}", name, e),
            }
        }

        Ok(results)
    }

    pub async fn health_check_all(&self) -> std::collections::HashMap<String, bool> {
        let channels = self.channels.read().await;
        let mut results = std::collections::HashMap::new();

        for (name, channel) in channels.iter() {
            let health = channel.read().await.health_check().await.unwrap_or(false);
            results.insert(name.clone(), health);
        }

        results
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentChannels {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(ChannelManager::new()),
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_channel(self, name: impl Into<String>, channel: Arc<RwLock<dyn Channel>>) -> Self {
        let manager = self.manager.clone();
        let name_str = name.into();
        tokio::spawn(async move {
            manager.register_channel(name_str, channel).await;
        });
        self
    }

    pub async fn enable(&self) -> Result<()> {
        self.manager.start_all().await?;
        *self.enabled.write().await = true;
        Ok(())
    }

    pub async fn disable(&self) -> Result<()> {
        self.manager.stop_all().await?;
        *self.enabled.write().await = false;
        Ok(())
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn send_to_channel(
        &self,
        channel_name: &str,
        message: SendMessage,
    ) -> Result<ChannelMessage> {
        self.manager.send_to_channel(channel_name, message).await
    }

    pub async fn broadcast(&self, message: SendMessage) -> Result<Vec<ChannelMessage>> {
        self.manager.broadcast(message).await
    }

    pub async fn list_channels(&self) -> Vec<String> {
        self.manager.list_channels().await
    }

    pub async fn health_check(&self) -> std::collections::HashMap<String, bool> {
        self.manager.health_check_all().await
    }

    pub fn manager(&self) -> Arc<ChannelManager> {
        self.manager.clone()
    }
}

impl Default for AgentChannels {
    fn default() -> Self {
        Self::new()
    }
}
