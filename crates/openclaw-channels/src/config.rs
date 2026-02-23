use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfigs(pub HashMap<String, ChannelConfigEntry>);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfigEntry {
    #[serde(rename = "type")]
    pub channel_type: String,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

pub use crate::bluebubbles::BlueBubblesConfig;
pub use crate::dingtalk::DingTalkConfig;
pub use crate::discord::DiscordConfig;
pub use crate::dm_policy::DmPolicyConfig;
pub use crate::email::EmailConfig;
pub use crate::feishu::FeishuConfig;
pub use crate::googlechat::GoogleChatConfig;
pub use crate::imessage::IMessageConfig;
pub use crate::matrix::MatrixConfig;
pub use crate::signal::SignalConfig;
pub use crate::slack::SlackConfig;
pub use crate::sms::SmsConfig;
pub use crate::teams::TeamsConfig;
pub use crate::telegram::TelegramConfig;
pub use crate::webchat::WebChatConfig;
pub use crate::wecom::WeComConfig;
pub use crate::whatsapp::WhatsAppConfig;
pub use crate::zalo::ZaloConfig;
pub use crate::zalo_personal::ZaloPersonalConfig;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_configs_default() {
        let configs = ChannelConfigs::default();
        assert!(configs.0.is_empty());
    }

    #[test]
    fn test_channel_configs_insert() {
        let mut configs = ChannelConfigs::default();
        configs.0.insert(
            "my_dingtalk".to_string(),
            ChannelConfigEntry {
                channel_type: "dingtalk".to_string(),
                config: serde_json::json!({
                    "webhook": "https://example.com/webhook"
                }),
                enabled: true,
            },
        );

        assert_eq!(configs.0.len(), 1);
        assert!(configs.0.contains_key("my_dingtalk"));
    }

    #[test]
    fn test_channel_config_entry_default() {
        let entry = ChannelConfigEntry {
            channel_type: String::new(),
            config: serde_json::Value::Null,
            enabled: true,
        };

        assert!(entry.channel_type.is_empty());
        assert!(entry.config.is_null());
        assert!(entry.enabled);
    }

    #[test]
    fn test_channel_config_entry_with_values() {
        let entry = ChannelConfigEntry {
            channel_type: "telegram".to_string(),
            config: serde_json::json!({
                "bot_token": "test_token"
            }),
            enabled: false,
        };

        assert_eq!(entry.channel_type, "telegram");
        assert!(!entry.enabled);
        assert_eq!(entry.config["bot_token"], "test_token");
    }

    #[test]
    fn test_channel_configs_serialize_deserialize() {
        let mut configs = ChannelConfigs::default();
        configs.0.insert(
            "dingtalk1".to_string(),
            ChannelConfigEntry {
                channel_type: "dingtalk".to_string(),
                config: serde_json::json!({
                    "webhook": "https://oapi.dingtalk.com/robot/send?access_token=xxx"
                }),
                enabled: true,
            },
        );
        configs.0.insert(
            "telegram1".to_string(),
            ChannelConfigEntry {
                channel_type: "telegram".to_string(),
                config: serde_json::json!({
                    "bot_token": "123456:ABC-DEF"
                }),
                enabled: true,
            },
        );

        let json = serde_json::to_string(&configs).unwrap();
        let parsed: ChannelConfigs = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.0.len(), 2);
        assert_eq!(parsed.0.get("dingtalk1").unwrap().channel_type, "dingtalk");
        assert_eq!(parsed.0.get("telegram1").unwrap().channel_type, "telegram");
    }

    #[test]
    fn test_channel_config_enabled_defaults_to_true() {
        let entry: ChannelConfigEntry = serde_json::from_str(
            r#"{
            "type": "dingtalk",
            "config": {"webhook": "test"}
        }"#,
        )
        .unwrap();

        assert!(entry.enabled);
    }

    #[test]
    fn test_channel_config_enabled_can_be_false() {
        let entry: ChannelConfigEntry = serde_json::from_str(
            r#"{
            "type": "dingtalk",
            "config": {"webhook": "test"},
            "enabled": false
        }"#,
        )
        .unwrap();

        assert!(!entry.enabled);
    }

    #[test]
    fn test_multiple_channels_same_type() {
        let mut configs = ChannelConfigs::default();

        configs.0.insert(
            "prod_dingtalk".to_string(),
            ChannelConfigEntry {
                channel_type: "dingtalk".to_string(),
                config: serde_json::json!({"webhook": "prod webhook"}),
                enabled: true,
            },
        );

        configs.0.insert(
            "dev_dingtalk".to_string(),
            ChannelConfigEntry {
                channel_type: "dingtalk".to_string(),
                config: serde_json::json!({"webhook": "dev webhook"}),
                enabled: true,
            },
        );

        assert_eq!(configs.0.len(), 2);
        assert!(configs.0.get("prod_dingtalk").is_some());
        assert!(configs.0.get("dev_dingtalk").is_some());
    }
}
