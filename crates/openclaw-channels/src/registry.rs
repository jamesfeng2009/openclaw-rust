use std::sync::Arc;

use crate::factory::ChannelFactoryRegistry;
use openclaw_core::Result;

pub async fn register_default_channels(registry: &ChannelFactoryRegistry) {
    register_telegram(registry).await;
    register_discord(registry).await;
    register_slack(registry).await;
    register_teams(registry).await;
    register_feishu(registry).await;
    register_wecom(registry).await;
    register_dingtalk(registry).await;
    register_whatsapp(registry).await;
}

async fn register_telegram(registry: &ChannelFactoryRegistry) {
    use crate::telegram::{TelegramBot, TelegramConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
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

    let creator =
        move |config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let discord_config = if let Some(obj) = config.as_object() {
                DiscordConfig {
                    bot_token: obj.get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    webhook_url: obj.get("webhook_url")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    enabled: obj.get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true),
                    #[cfg(feature = "discord")]
                    use_gateway: obj.get("use_gateway")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                }
            } else {
                DiscordConfig {
                    bot_token: String::new(),
                    webhook_url: None,
                    enabled: true,
                    #[cfg(feature = "discord")]
                    use_gateway: false,
                }
            };
            let channel = DiscordChannel::new(discord_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("discord".to_string(), creator).await;
}

async fn register_slack(registry: &ChannelFactoryRegistry) {
    use crate::slack::{SlackChannel, SlackConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let slack_config = SlackConfig {
                bot_token: None,
                webhook_url: None,
                app_token: None,
                enabled: true,
            };
            let channel = SlackChannel::new(slack_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("slack".to_string(), creator).await;
}

async fn register_teams(registry: &ChannelFactoryRegistry) {
    use crate::teams::{TeamsChannel, TeamsConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let teams_config = TeamsConfig {
                webhook_url: None,
                bot_id: None,
                bot_password: None,
                enabled: true,
            };
            let channel = TeamsChannel::new(teams_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("teams".to_string(), creator).await;
}

async fn register_feishu(registry: &ChannelFactoryRegistry) {
    use crate::feishu::{FeishuChannel, FeishuConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let feishu_config = FeishuConfig {
                app_id: String::new(),
                app_secret: String::new(),
                webhook: None,
                enabled: true,
            };
            let channel = FeishuChannel::new(feishu_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("feishu".to_string(), creator).await;
}

async fn register_wecom(registry: &ChannelFactoryRegistry) {
    use crate::wecom::{WeComChannel, WeComConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let wecom_config = WeComConfig {
                webhook: String::new(),
                enabled: true,
            };
            let channel = WeComChannel::new(wecom_config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("wecom".to_string(), creator).await;
}

async fn register_dingtalk(registry: &ChannelFactoryRegistry) {
    use crate::dingtalk::{DingTalkChannel, DingTalkConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = DingTalkConfig {
                webhook: String::new(),
                secret: None,
                enabled: true,
            };
            let channel = DingTalkChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("dingtalk".to_string(), creator).await;
}

async fn register_whatsapp(registry: &ChannelFactoryRegistry) {
    use crate::whatsapp::{WhatsAppChannel, WhatsAppConfig};

    let creator =
        move |_config: serde_json::Value| -> Result<Arc<tokio::sync::RwLock<dyn crate::Channel>>> {
            let config = WhatsAppConfig {
                business_account_id: String::new(),
                phone_number_id: String::new(),
                access_token: String::new(),
                verify_token: None,
                enabled: true,
            };
            let channel = WhatsAppChannel::new(config);
            Ok(Arc::new(tokio::sync::RwLock::new(channel)))
        };
    registry.register("whatsapp".to_string(), creator).await;
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
