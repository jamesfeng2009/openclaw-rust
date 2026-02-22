//! 通道工厂模块
//!
//! 提供通道的工厂模式实现，支持通过配置动态创建通道

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::base::Channel;
use openclaw_core::{OpenClawError, Result};

pub type ChannelCreator = Box<dyn Fn(serde_json::Value) -> Result<Arc<RwLock<dyn Channel>>> + Send + Sync>;

pub struct ChannelFactoryRegistry {
    creators: RwLock<HashMap<String, ChannelCreator>>,
}

impl ChannelFactoryRegistry {
    pub fn new() -> Self {
        Self {
            creators: RwLock::new(HashMap::new()),
        }
    }

    pub async fn register<F>(&self, channel_type: String, creator: F)
    where
        F: Fn(serde_json::Value) -> Result<Arc<RwLock<dyn Channel>>> + Send + Sync + 'static,
    {
        let mut creators = self.creators.write().await;
        creators.insert(channel_type, Box::new(creator));
    }

    pub async fn create(&self, channel_type: &str, config: serde_json::Value) -> Result<Arc<RwLock<dyn Channel>>> {
        let creators = self.creators.read().await;
        let creator = creators.get(channel_type).ok_or_else(|| {
            OpenClawError::Config(format!(
                "Unsupported channel type: {}. Use register_default_channels() first.",
                channel_type
            ))
        })?;
        creator(config)
    }

    pub async fn list_types(&self) -> Vec<String> {
        let creators = self.creators.read().await;
        creators.keys().cloned().collect()
    }

    pub async fn contains(&self, channel_type: &str) -> bool {
        let creators = self.creators.read().await;
        creators.contains_key(channel_type)
    }
}

impl Default for ChannelFactoryRegistry {
    fn default() -> Self {
        Self::new()
    }
}
