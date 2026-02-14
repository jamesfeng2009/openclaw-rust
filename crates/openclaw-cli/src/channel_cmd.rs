//! 通道配置 CLI 工具
//!
//! 提供命令行接口来管理各通道的配置

use clap::Subcommand;
use openclaw_core::OpenClawError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 通道配置文件路径
fn get_channel_config_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".openclaw").join("channels.json")
}

/// 通道配置管理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfigManager {
    /// 各通道配置
    pub channels: HashMap<String, ChannelConfig>,
    /// 默认通道
    #[serde(default)]
    pub default_channel: Option<String>,
}

/// 单个通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 通道类型
    pub channel_type: String,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
    /// 配置参数
    #[serde(flatten)]
    pub config: HashMap<String, serde_json::Value>,
}

impl ChannelConfigManager {
    /// 加载配置
    pub fn load() -> Result<Self, OpenClawError> {
        let path = get_channel_config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| OpenClawError::Config(format!("读取通道配置失败: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| OpenClawError::Serialization(e))
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), OpenClawError> {
        let path = get_channel_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OpenClawError::Config(format!("创建配置目录失败: {}", e)))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| OpenClawError::Serialization(e))?;
        std::fs::write(&path, content)
            .map_err(|e| OpenClawError::Config(format!("保存通道配置失败: {}", e)))?;
        Ok(())
    }

    /// 获取通道配置
    pub fn get_channel(&self, name: &str) -> Option<&ChannelConfig> {
        self.channels.get(name)
    }

    /// 设置通道配置
    pub fn set_channel(&mut self, name: String, config: ChannelConfig) {
        self.channels.insert(name, config);
    }

    /// 删除通道配置
    pub fn remove_channel(&mut self, name: &str) -> bool {
        self.channels.remove(name).is_some()
    }

    /// 列出所有通道
    pub fn list_channels(&self) -> Vec<&String> {
        self.channels.keys().collect()
    }
}

/// 支持的通道类型
const CHANNEL_TYPES: &[(&str, &str)] = &[
    ("dingtalk", "钉钉"),
    ("wecom", "企业微信"),
    ("feishu", "飞书"),
    ("discord", "Discord"),
    ("teams", "Microsoft Teams"),
    ("slack", "Slack"),
    ("whatsapp", "WhatsApp"),
    ("telegram", "Telegram"),
];

#[derive(Debug, Subcommand)]
pub enum ChannelCommand {
    /// 设置通道配置
    Set {
        /// 通道类型 (dingtalk, wecom, feishu, discord, teams, slack, whatsapp, telegram)
        channel_type: String,
        /// 配置参数 (格式: key=value，可多次使用)
        #[arg(short, long = "config", value_parser = parse_key_value, action = clap::ArgAction::Append)]
        configs: Vec<(String, String)>,
        /// 启用通道
        #[arg(short, long)]
        enable: bool,
    },

    /// 获取通道配置
    Get {
        /// 通道类型
        channel_type: String,
    },

    /// 删除通道配置
    Remove {
        /// 通道类型
        channel_type: String,
    },

    /// 列出所有通道配置
    List,

    /// 启用通道
    Enable {
        /// 通道类型
        channel_type: String,
    },

    /// 禁用通道
    Disable {
        /// 通道类型
        channel_type: String,
    },

    /// 设置默认通道
    Default {
        /// 通道类型
        channel_type: String,
    },

    /// 测试通道连接
    Test {
        /// 通道类型
        channel_type: String,
        /// 测试消息
        #[arg(short, long, default_value = "测试消息")]
        message: String,
        /// 目标 ID (如 chat_id, phone number)
        #[arg(short, long)]
        target: Option<String>,
    },

    /// 显示帮助信息
    Help,
}

/// 解析 key=value 格式
fn parse_key_value(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("无效的配置格式: {}，应为 key=value", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

impl ChannelCommand {
    /// 执行命令
    pub async fn execute(&self) -> Result<(), OpenClawError> {
        let mut manager = ChannelConfigManager::load()?;

        match self {
            ChannelCommand::Set { channel_type, configs, enable } => {
                let channel_type_lower = channel_type.to_lowercase();
                
                // 验证通道类型
                if !CHANNEL_TYPES.iter().any(|(t, _)| *t == channel_type_lower) {
                    println!("❌ 不支持的通道类型: {}", channel_type);
                    println!("\n支持的通道类型:");
                    for (t, name) in CHANNEL_TYPES {
                        println!("  {} - {}", t, name);
                    }
                    return Ok(());
                }

                // 构建配置
                let mut config_map = HashMap::new();
                for (key, value) in configs {
                    // 尝试解析为 JSON 值
                    let json_value = if value.starts_with('"') && value.ends_with('"') {
                        serde_json::Value::String(value[1..value.len()-1].to_string())
                    } else if value == "true" || value == "false" {
                        serde_json::Value::Bool(value == "true")
                    } else if let Ok(n) = value.parse::<i64>() {
                        serde_json::Value::Number(n.into())
                    } else {
                        serde_json::Value::String(value.clone())
                    };
                    config_map.insert(key.clone(), json_value);
                }

                let config = ChannelConfig {
                    channel_type: channel_type_lower.clone(),
                    enabled: *enable,
                    config: config_map,
                };

                manager.set_channel(channel_type_lower.clone(), config);
                manager.save()?;

                println!("✅ 成功设置 {} 通道配置", channel_type);
                if *enable {
                    println!("   状态: 已启用");
                }
                println!("\n使用 'openclaw-rust channel test {}' 测试连接", channel_type);
            }

            ChannelCommand::Get { channel_type } => {
                if let Some(config) = manager.get_channel(channel_type) {
                    println!("通道: {} ({})", channel_type, config.channel_type);
                    println!("状态: {}", if config.enabled { "已启用" } else { "已禁用" });
                    println!("\n配置:");
                    for (key, value) in &config.config {
                        // 隐藏敏感信息
                        if key.contains("token") || key.contains("secret") || key.contains("key") {
                            let masked = mask_sensitive_value(value);
                            println!("  {}: {}", key, masked);
                        } else {
                            println!("  {}: {}", key, value);
                        }
                    }
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                    println!("\n使用 'openclaw-rust channel set {}' 创建配置", channel_type);
                }
            }

            ChannelCommand::Remove { channel_type } => {
                if manager.remove_channel(channel_type) {
                    manager.save()?;
                    println!("✅ 已删除 {} 通道配置", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
            }

            ChannelCommand::List => {
                let channels = manager.list_channels();
                if channels.is_empty() {
                    println!("暂无配置的通道");
                    println!("\n使用方法:");
                    println!("  openclaw-rust channel set dingtalk --config webhook=https://xxx --enable");
                    println!("\n支持的通道类型:");
                    for (t, name) in CHANNEL_TYPES {
                        println!("  {} - {}", t, name);
                    }
                } else {
                    println!("已配置的通道:");
                    println!();
                    for channel in channels {
                        if let Some(config) = manager.get_channel(channel) {
                            let status = if config.enabled { "✅ 启用" } else { "⏸️ 禁用" };
                            let default_marker = if manager.default_channel.as_deref() == Some(channel) {
                                " (默认)"
                            } else {
                                ""
                            };
                            println!("  {} {}{}", status, channel, default_marker);
                        }
                    }
                }
            }

            ChannelCommand::Enable { channel_type } => {
                if let Some(config) = manager.channels.get_mut(channel_type) {
                    config.enabled = true;
                    manager.save()?;
                    println!("✅ 已启用 {} 通道", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
            }

            ChannelCommand::Disable { channel_type } => {
                if let Some(config) = manager.channels.get_mut(channel_type) {
                    config.enabled = false;
                    manager.save()?;
                    println!("✅ 已禁用 {} 通道", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
            }

            ChannelCommand::Default { channel_type } => {
                if manager.get_channel(channel_type).is_some() {
                    manager.default_channel = Some(channel_type.clone());
                    manager.save()?;
                    println!("✅ 已设置默认通道: {}", channel_type);
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
            }

            ChannelCommand::Test { channel_type, message, target } => {
                println!("🔍 测试 {} 通道...", channel_type);
                
                if let Some(_config) = manager.get_channel(channel_type) {
                    // TODO: 实际测试通道连接
                    println!("   消息: {}", message);
                    if let Some(t) = target {
                        println!("   目标: {}", t);
                    }
                    println!("\n⚠️  测试功能开发中，请手动验证配置");
                } else {
                    println!("❌ 未找到通道配置: {}", channel_type);
                }
            }

            ChannelCommand::Help => {
                println!("通道配置命令帮助");
                println!("\n支持的通道类型:");
                for (t, name) in CHANNEL_TYPES {
                    println!("  {} - {}", t, name);
                }
                println!("\n配置示例:");
                println!();
                println!("  # 钉钉 (Webhook)");
                println!("  openclaw-rust channel set dingtalk --config webhook=https://oapi.dingtalk.com/robot/send?access_token=xxx --config secret=SECxxx --enable");
                println!();
                println!("  # 企业微信 (Webhook)");
                println!("  openclaw-rust channel set wecom --config webhook=https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx --enable");
                println!();
                println!("  # 飞书 (Bot API)");
                println!("  openclaw-rust channel set feishu --config app_id=cli_xxx --config app_secret=xxx --enable");
                println!();
                println!("  # Discord (Webhook)");
                println!("  openclaw-rust channel set discord --config webhook_url=https://discord.com/api/webhooks/xxx/yyy --enable");
                println!();
                println!("  # Microsoft Teams (Webhook)");
                println!("  openclaw-rust channel set teams --config webhook_url=https://outlook.office.com/webhook/xxx --enable");
                println!();
                println!("  # Slack (Webhook)");
                println!("  openclaw-rust channel set slack --config webhook_url=https://hooks.slack.com/services/xxx --enable");
                println!();
                println!("  # WhatsApp (Cloud API)");
                println!("  openclaw-rust channel set whatsapp --config phone_number_id=123456 --config access_token=EAAxxx --enable");
                println!();
                println!("  # Telegram (Bot)");
                println!("  openclaw-rust channel set telegram --config bot_token=123456:ABC --enable");
            }
        }

        Ok(())
    }
}

/// 隐藏敏感值
fn mask_sensitive_value(value: &serde_json::Value) -> String {
    let s = value.as_str().unwrap_or("");
    if s.len() <= 8 {
        return "*".repeat(s.len());
    }
    let start = &s[..4];
    let end = &s[s.len()-4..];
    format!("{}****{}", start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_value() {
        let result = parse_key_value("webhook=https://example.com").unwrap();
        assert_eq!(result.0, "webhook");
        assert_eq!(result.1, "https://example.com");
    }

    #[test]
    fn test_mask_sensitive_value() {
        let value = serde_json::Value::String("sk-1234567890abcdef".to_string());
        let masked = mask_sensitive_value(&value);
        assert_eq!(masked, "sk-1****cdef");
    }
}
