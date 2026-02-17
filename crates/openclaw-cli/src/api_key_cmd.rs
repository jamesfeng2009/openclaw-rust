//! API Key 管理 CLI 工具
//!
//! 提供命令行接口来管理用户的 API Key

use clap::Subcommand;
use openclaw_core::{OpenClawError, UserConfigManager, UserProviderConfig};

#[derive(Debug, Subcommand)]
pub enum ApiKeyCommand {
    /// 设置 API Key
    Set {
        /// 提供商名称 (openai, anthropic, gemini, glm, qwen, kimi, minimax, deepseek)
        provider: String,
        /// API Key
        api_key: String,
        /// 默认模型（可选）
        #[arg(short, long)]
        model: Option<String>,
        /// Base URL（可选，用于自定义端点）
        #[arg(short, long)]
        url: Option<String>,
    },

    /// 获取 API Key（部分显示）
    Get {
        /// 提供商名称
        provider: String,
    },

    /// 删除 API Key
    Remove {
        /// 提供商名称
        provider: String,
    },

    /// 列出所有提供商
    List,

    /// 设置默认提供商
    Default {
        /// 提供商名称
        provider: String,
    },

    /// 导出配置（隐藏敏感信息）
    Export,

    /// 验证 API Key 格式
    Validate {
        /// 提供商名称
        provider: String,
        /// API Key
        api_key: String,
    },
}

impl ApiKeyCommand {
    /// 执行命令
    pub async fn execute(&self) -> Result<(), OpenClawError> {
        let mut manager = UserConfigManager::new(None)?;

        match self {
            ApiKeyCommand::Set {
                provider,
                api_key,
                model,
                url,
            } => {
                // 验证 API Key 格式
                if !openclaw_core::UserConfig::validate_api_key(provider, api_key)? {
                    println!("⚠️  API Key 格式可能不正确");
                }

                // 创建配置
                let config = UserProviderConfig {
                    name: provider.clone(),
                    api_key: Some(api_key.clone()),
                    base_url: url.clone(),
                    default_model: model.clone().unwrap_or_else(|| get_default_model(provider)),
                    enabled: true,
                    quota: None,
                };

                manager
                    .get_config_mut()
                    .set_provider(provider.clone(), config);
                manager.save()?;

                println!("✅ 成功设置 {} 的 API Key", provider);
                println!(
                    "   默认模型: {}",
                    model.as_ref().unwrap_or(&get_default_model(provider))
                );
                if let Some(url) = url {
                    println!("   Base URL: {}", url);
                }
            }

            ApiKeyCommand::Get { provider } => {
                if let Some(config) = manager.get_config().get_provider(provider) {
                    if let Some(key) = &config.api_key {
                        let masked = mask_api_key(key);
                        println!("提供商: {}", provider);
                        println!("API Key: {}", masked);
                        println!("默认模型: {}", config.default_model);
                        println!("状态: {}", if config.enabled { "启用" } else { "禁用" });
                    } else {
                        println!("⚠️  未设置 API Key");
                    }
                } else {
                    println!("❌ 未找到提供商: {}", provider);
                }
            }

            ApiKeyCommand::Remove { provider } => {
                if manager.remove_api_key(provider).is_ok() {
                    println!("✅ 已删除 {} 的 API Key", provider);
                } else {
                    println!("❌ 未找到提供商: {}", provider);
                }
            }

            ApiKeyCommand::List => {
                let providers = manager.list_providers();
                if providers.is_empty() {
                    println!("暂无配置的提供商");
                    println!("\n使用方法:");
                    println!("  openclaw api-key set openai sk-xxx");
                } else {
                    println!("已配置的提供商:");
                    println!();
                    for provider in providers {
                        if let Some(config) = manager.get_config().get_provider(provider) {
                            let key_status = if config.api_key.is_some() {
                                "✅"
                            } else {
                                "❌"
                            };
                            let default_marker =
                                if manager.get_config().default_provider == *provider {
                                    " (默认)"
                                } else {
                                    ""
                                };
                            println!(
                                "  {} {}{} - {}",
                                key_status, provider, default_marker, config.default_model
                            );
                        }
                    }
                }
            }

            ApiKeyCommand::Default { provider } => {
                if manager.get_config().get_provider(provider).is_some() {
                    manager
                        .get_config_mut()
                        .set_default_provider(provider.clone());
                    manager.save()?;
                    println!("✅ 已设置默认提供商: {}", provider);
                } else {
                    println!("❌ 未找到提供商: {}", provider);
                    println!("请先设置该提供商的 API Key");
                }
            }

            ApiKeyCommand::Export => {
                let safe_config = manager.export_safe();
                println!("{}", serde_json::to_string_pretty(&safe_config).unwrap());
            }

            ApiKeyCommand::Validate { provider, api_key } => {
                match openclaw_core::UserConfig::validate_api_key(provider, api_key) {
                    Ok(true) => println!("✅ API Key 格式正确"),
                    Ok(false) => println!("⚠️  API Key 格式可能不正确"),
                    Err(e) => println!("❌ 验证失败: {}", e),
                }
            }
        }

        Ok(())
    }
}

/// 获取默认模型
fn get_default_model(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "openai" => "gpt-4o-mini".to_string(),
        "anthropic" => "claude-3-5-sonnet-20241022".to_string(),
        "gemini" | "google" => "gemini-2.0-flash".to_string(),
        "glm" | "zhipu" => "glm-4-flash".to_string(),
        "qwen" | "tongyi" => "qwen-plus".to_string(),
        "kimi" | "moonshot" => "moonshot-v1-8k".to_string(),
        "minimax" => "abab6.5s-chat".to_string(),
        "deepseek" => "deepseek-chat".to_string(),
        _ => "default".to_string(),
    }
}

/// 隐藏 API Key 中间部分
fn mask_api_key(key: &str) -> String {
    if key.len() <= 8 {
        return "*".repeat(key.len());
    }

    let start = &key[..4];
    let end = &key[key.len().saturating_sub(4)..];
    let middle_len = (key.len() - 8).min(4);
    let middle = "*".repeat(middle_len);

    format!("{}{}{}", start, middle, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-short"), "********");
        assert_eq!(mask_api_key("sk-1234567890abcdef"), "sk-1****cdef");
    }

    #[test]
    fn test_get_default_model() {
        assert_eq!(get_default_model("openai"), "gpt-4o-mini");
        assert_eq!(get_default_model("anthropic"), "claude-3-5-sonnet-20241022");
    }
}
