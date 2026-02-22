use std::sync::Arc;

use crate::factory::ChannelFactoryRegistry;
use openclaw_core::Result;

pub async fn register_default_channels(registry: &ChannelFactoryRegistry) {
    register_telegram(registry).await;
    register_discord(registry).await;
}

async fn register_telegram(registry: &ChannelFactoryRegistry) {
    use crate::telegram::{TelegramBot, TelegramConfig};

    let creator = move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
        let telegram_config = TelegramConfig {
            bot_token: String::new(),
            enabled: true,
        };
        let bot = TelegramBot::new(telegram_config);
        Ok(Arc::new(tokio::sync::RwLock::new(bot)))
    };
    registry.register("telegram".to_string(), creator).await;
}

async fn register_discord(registry: &ChannelFactoryRegistry) {
    use crate::discord::{DiscordChannel, DiscordConfig};

    let creator = move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
        let discord_config = DiscordConfig {
            bot_token: String::new(),
            webhook_url: None,
            enabled: true,
        };
        let channel = DiscordChannel::new(discord_config);
        Ok(Arc::new(tokio::sync::RwLock::new(channel)))
    };
    registry.register("discord".to_string(), creator).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_default_channels() {
        let registry = ChannelFactoryRegistry::new();
        
        register_default_channels(&registry).await;
        
        let types = registry.list_types().await;
        assert!(types.contains(&"telegram".to_string()));
        assert!(types.contains(&"discord".to_string()));
    }

    #[tokio::test]
    async fn test_register_specific_channel() {
        let registry = ChannelFactoryRegistry::new();
        
        register_telegram(&registry).await;
        
        assert!(registry.contains("telegram").await);
        assert!(!registry.contains("discord").await);
    }

    #[tokio::test]
    async fn test_create_channel_from_registry() {
        let registry = ChannelFactoryRegistry::new();
        
        register_telegram(&registry).await;
        
        let config = serde_json::json!({
            "bot_token": "test_token",
            "enabled": true
        });
        
        let channel = registry.create("telegram", config).await;
        assert!(channel.is_ok());
    }
}
