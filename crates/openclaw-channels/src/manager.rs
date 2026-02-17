use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::base::{Channel, ChannelEvent, ChannelHandler};
use crate::types::{ChannelMessage, SendMessage};
use openclaw_core::Result;

pub struct ChannelManager {
    channels: Arc<RwLock<HashMap<String, Arc<RwLock<dyn Channel>>>>>,
    handlers: Arc<RwLock<Vec<Arc<dyn ChannelHandler>>>>,
    event_tx: tokio::sync::mpsc::Sender<ChannelEvent>,
}

impl ChannelManager {
    pub fn new() -> Self {
        let (event_tx, _) = tokio::sync::mpsc::channel(1000);
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            event_tx,
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

    pub async fn add_handler(&self, handler: Arc<dyn ChannelHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }

    pub async fn start_all(&self) -> Result<()> {
        let channels = self.channels.read().await;
        for (name, channel) in channels.iter() {
            tracing::info!("Starting channel: {}", name);
            channel.write().await.start().await?;
        }
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<()> {
        let channels = self.channels.read().await;
        for (name, channel) in channels.iter() {
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

    pub async fn process_event(&self, event: ChannelEvent) {
        let handlers = self.handlers.read().await;
        for handler in handlers.iter() {
            if let Err(e) = handler.handle_event(event.clone()).await {
                tracing::error!("Handler error: {}", e);
            }
        }
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

    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let channels = self.channels.read().await;
        let mut results = HashMap::new();

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
