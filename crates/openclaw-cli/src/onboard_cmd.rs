//! Onboard 命令 - 交互式初始化

use anyhow::{Context, Result};
use dialoguer::{Input, Select, MultiSelect, Confirm, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DEFAULT_PROVIDERS: &[&str] = &[
    "OpenAI",
    "Anthropic (Claude)",
    "Google (Gemini)",
    "DeepSeek",
    "通义千问 (Qwen)",
    "智谱 GLM",
    "Moonshot (Kimi)",
    "MiniMax",
    "豆包 (Doubao)",
    "自定义 (OpenAI兼容)",
];

const DEFAULT_MODELS: &[(&str, &[&str])] = &[
    ("openai", &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-preview"]),
    ("anthropic", &["claude-4-opus", "claude-4-sonnet", "claude-3.5-sonnet", "claude-3-opus"]),
    ("google", &["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"]),
    ("deepseek", &["deepseek-chat", "deepseek-coder", "deepseek-reasoner"]),
    ("qwen", &["qwen-max", "qwen-plus", "qwen-turbo", "qwen-vl-max"]),
    ("zhipu", &["glm-4", "glm-4-plus", "glm-3-turbo"]),
    ("moonshot", &["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"]),
    ("minimax", &["abab6.5s-chat", "abab6.5g-chat"]),
    ("doubao", &["doubao-lite", "doubao-pro"]),
    ("custom", &["gpt-4", "claude-3"]),
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OnboardConfig {
    pub user_name: Option<String>,
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: AgentsConfig,
    pub channels: ChannelsConfig,
    pub features: FeaturesConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    #[serde(default)]
    pub defaults: DefaultAgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultAgentConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
}

impl Default for DefaultAgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: default_provider(),
        }
    }
}

fn default_model() -> String {
    "gpt-4o".to_string()
}

fn default_provider() -> String {
    "openai".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: ChannelConfig,
    #[serde(default)]
    pub discord: ChannelConfig,
    #[serde(default)]
    pub whatsapp: ChannelConfig,
    #[serde(default)]
    pub feishu: ChannelConfig,
    #[serde(default)]
    pub dingtalk: ChannelConfig,
    #[serde(default)]
    pub wecom: ChannelConfig,
    #[serde(default)]
    pub slack: ChannelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_from: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeaturesConfig {
    #[serde(default = "default_true")]
    pub voice: bool,
    #[serde(default = "default_true")]
    pub browser: bool,
    #[serde(default = "default_true")]
    pub canvas: bool,
    #[serde(default = "default_true")]
    pub cron: bool,
    #[serde(default = "default_true")]
    pub webhook: bool,
    #[serde(default = "default_true")]
    pub sandbox: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub dm_policy_pairing: bool,
    #[serde(default = "default_true")]
    pub sandbox_tools: bool,
    #[serde(default = "default_true")]
    pub prompt_injection_protection: bool,
    #[serde(default = "default_true")]
    pub network_allowlisting: bool,
    #[serde(default)]
    pub allowed_endpoints: Vec<String>,
}

fn default_true() -> bool {
    true
}

pub async fn run(quick: bool) -> Result<()> {
    println!();
    println!("\x1b[36m\x1b[1m🦀 Welcome to OpenClaw Rust!\x1b[0m");
    println!();
    println!("\x1b[2mLet's get you set up in just a few steps...\x1b[0m");
    println!();

    let config_path = get_config_path()?;
    let existing = load_existing_config(&config_path);

    let mut config = existing.unwrap_or_default();

    if quick {
        run_quick_mode(&mut config)?;
    } else {
        run_interactive_mode(&mut config).await?;
    }

    save_config(&config_path, &config)?;

    println!();
    println!("\x1b[32m✅ All done!\x1b[0m");
    println!();
    println!("  Config saved to: {}", config_path.display());
    println!();
    println!("\x1b[33mNext steps:\x1b[0m");
    println!("  • \x1b[36mopenclaw-rust doctor\x1b[0m - Check system health");
    println!("  • \x1b[36mopenclaw-rust gateway\x1b[0m - Start the service");
    println!();

    Ok(())
}

fn run_quick_mode(config: &mut OnboardConfig) -> Result<()> {
    println!("\x1b[33m🚀 Quick mode - minimal setup\x1b[0m");
    println!();

    config.user_name = Some(
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Your name")
            .default("User".to_string())
            .interact()?
    );

    let provider_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select AI provider")
        .items(DEFAULT_PROVIDERS)
        .default(0)
        .interact()?;

    let provider_name = DEFAULT_PROVIDERS[provider_idx].to_lowercase();
    let (actual_name, models) = get_provider_info(provider_idx);

    let model_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select model")
        .items(models)
        .default(0)
        .interact()?;

    config.agents.defaults.provider = actual_name.to_string();
    config.agents.defaults.model = models[model_idx].to_string();

    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("{} API Key (skip if using env var)", DEFAULT_PROVIDERS[provider_idx]))
        .allow_empty(true)
        .interact()?;

    if !api_key.is_empty() {
        config.providers.insert(
            actual_name.to_string(),
            ProviderConfig { api_key: Some(api_key), api_base: None },
        );
    }

    Ok(())
}

async fn run_interactive_mode(config: &mut OnboardConfig) -> Result<()> {
    println!("\x1b[33m\x1b[1m📝 Step 1: Basic Info\x1b[0m");
    
    config.user_name = Some(
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("What's your name?")
            .default("User".to_string())
            .interact()?
    );

    println!();
    println!("\x1b[33m\x1b[1m🤖 Step 2: AI Provider\x1b[0m");

    let provider_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your primary AI provider")
        .items(DEFAULT_PROVIDERS)
        .default(0)
        .interact()?;

    let (actual_name, models) = get_provider_info(provider_idx);

    let model_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select default model")
        .items(models)
        .default(0)
        .interact()?;

    config.agents.defaults.provider = actual_name.to_string();
    config.agents.defaults.model = models[model_idx].to_string();

    println!();
    println!("\x1b[33m\x1b[1m🔑 Step 3: API Key\x1b[0m");

    let env_var = get_env_var_name(actual_name);
    let has_env = std::env::var(&env_var).is_ok();

    if has_env {
        println!("  \x1b[32m{} detected in environment ✅\x1b[0m", env_var);
    } else {
        let api_key: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Enter {} API Key (or press Enter to skip)", DEFAULT_PROVIDERS[provider_idx]))
            .allow_empty(true)
            .interact()?;
        
        if !api_key.is_empty() {
            config.providers.insert(
                actual_name.to_string(),
                ProviderConfig { api_key: Some(api_key), api_base: None },
            );
        }
    }

    println!();
    println!("\x1b[33m\x1b[1m⚡ Step 4: Features\x1b[0m");

    let feature_items = vec![
        "Voice (STT/TTS)",
        "Browser Control",
        "Real-time Canvas",
        "Cron Jobs",
        "Webhooks",
        "Sandbox (Docker/WASM)",
    ];

    let selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select features to enable")
        .items(&feature_items)
        .defaults(&[true, false, false, true, true, false])
        .interact()?;

    config.features.voice = selected.contains(&0);
    config.features.browser = selected.contains(&1);
    config.features.canvas = selected.contains(&2);
    config.features.cron = selected.contains(&3);
    config.features.webhook = selected.contains(&4);
    config.features.sandbox = selected.contains(&5);

    println!();
    println!("\x1b[33m\x1b[1m🔒 Step 4b: Security Settings\x1b[0m");

    let security_items = vec![
        "DM Pairing (require approval for unknown senders)",
        "Sandbox untrusted tools (WASM/Docker)",
        "Prompt injection protection",
        "Network allowlisting",
    ];

    let security_selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select security options")
        .items(&security_items)
        .defaults(&[true, true, true, false])
        .interact()?;

    config.security.dm_policy_pairing = security_selected.contains(&0);
    config.security.sandbox_tools = security_selected.contains(&1);
    config.security.prompt_injection_protection = security_selected.contains(&2);
    config.security.network_allowlisting = security_selected.contains(&3);

    if config.security.network_allowlisting {
        let endpoints: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Allowed endpoints (comma-separated, empty for all)")
            .allow_empty(true)
            .default("api.openai.com,api.anthropic.com".to_string())
            .interact()?;
        
        if !endpoints.is_empty() {
            config.security.allowed_endpoints = endpoints
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
    }

    println!();
    println!("\x1b[33m\x1b[1m💬 Step 5: Chat Channels (optional)\x1b[0m");
    
    let channel_items = vec![
        "Telegram",
        "Discord",
        "WhatsApp",
        "飞书 (Feishu)",
        "钉钉 (DingTalk)",
        "企业微信 (WeCom)",
        "Slack",
    ];

    let selected_channels = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select channels to configure")
        .items(&channel_items)
        .interact()?;

    if selected_channels.contains(&0) {
        let token: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Telegram Bot Token (from @BotFather)")
            .allow_empty(true)
            .interact()?;
        if !token.is_empty() {
            config.channels.telegram.enabled = true;
            config.channels.telegram.token = Some(token);
        }
    }

    if selected_channels.contains(&1) {
        let token: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Discord Bot Token")
            .allow_empty(true)
            .interact()?;
        if !token.is_empty() {
            config.channels.discord.enabled = true;
            config.channels.discord.token = Some(token);
        }
    }

    if selected_channels.contains(&2) {
        println!("  \x1b[2mWhatsApp: Run 'openclaw-rust channels login whatsapp' after setup\x1b[0m");
    }

    if selected_channels.contains(&3) {
        let app_id: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Feishu App ID")
            .allow_empty(true)
            .interact()?;
        let app_secret: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Feishu App Secret")
            .allow_empty(true)
            .interact()?;
        if !app_id.is_empty() && !app_secret.is_empty() {
            config.channels.feishu.enabled = true;
            config.channels.feishu.token = Some(app_id);
            config.channels.feishu.api_key = Some(app_secret);
        }
    }

    if selected_channels.contains(&4) {
        let app_key: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("DingTalk App Key")
            .allow_empty(true)
            .interact()?;
        let app_secret: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("DingTalk App Secret")
            .allow_empty(true)
            .interact()?;
        if !app_key.is_empty() && !app_secret.is_empty() {
            config.channels.dingtalk.enabled = true;
            config.channels.dingtalk.token = Some(app_key);
            config.channels.dingtalk.api_key = Some(app_secret);
        }
    }

    if selected_channels.contains(&5) {
        let corp_id: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("WeCom Corp ID")
            .allow_empty(true)
            .interact()?;
        let agent_id: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("WeCom Agent ID")
            .allow_empty(true)
            .interact()?;
        let secret: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("WeCom Secret")
            .allow_empty(true)
            .interact()?;
        if !corp_id.is_empty() && !agent_id.is_empty() && !secret.is_empty() {
            config.channels.wecom.enabled = true;
            config.channels.wecom.token = Some(format!("{}:{}", corp_id, agent_id));
            config.channels.wecom.api_key = Some(secret);
        }
    }

    if selected_channels.contains(&6) {
        let bot_token: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Slack Bot Token (xoxb-...)")
            .allow_empty(true)
            .interact()?;
        if !bot_token.is_empty() {
            config.channels.slack.enabled = true;
            config.channels.slack.token = Some(bot_token);
        }
    }

    println!();
    println!("\x1b[33m\x1b[1m🧠 Step 6: Memory Settings\x1b[0m");

    let memory_enabled = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Enable persistent memory?")
        .default(true)
        .interact()?;

    if memory_enabled {
        let memory_items = vec![
            "Working memory (recent conversations)",
            "Short-term memory (daily summaries)",
            "Long-term memory (vector search)",
        ];

        let memory_selected = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select memory layers to enable")
            .items(&memory_items)
            .defaults(&[true, true, true])
            .interact()?;

        if memory_selected.contains(&2) {
            let embedding_provider: String = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select embedding provider for long-term memory")
                .items(&["OpenAI", "Ollama (local)", "Custom"])
                .default(0)
                .interact()?;

            match embedding_provider {
                0 => {
                    config.providers.insert(
                        "openai-embedding".to_string(),
                        ProviderConfig { api_key: None, api_base: None },
                    );
                }
                1 => {
                    config.providers.insert(
                        "ollama".to_string(),
                        ProviderConfig { api_key: None, api_base: Some("http://localhost:11434".to_string()) },
                    );
                }
                _ => {}
            }
        }
    }

    let test_connection = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Test provider connection now?")
        .default(true)
        .interact()?;

    if test_connection {
        println!();
        println!("  Testing connection...");
        println!("  \x1b[2m(Connection test skipped in this version)\x1b[0m");
    }

    Ok(())
}

fn get_provider_info(idx: usize) -> (&'static str, &'static [&'static str]) {
    match idx {
        0 => ("openai", &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-preview"]),
        1 => ("anthropic", &["claude-4-opus", "claude-4-sonnet", "claude-3.5-sonnet", "claude-3-opus"]),
        2 => ("google", &["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"]),
        3 => ("deepseek", &["deepseek-chat", "deepseek-coder", "deepseek-reasoner"]),
        4 => ("qwen", &["qwen-max", "qwen-plus", "qwen-turbo", "qwen-vl-max"]),
        5 => ("zhipu", &["glm-4", "glm-4-plus", "glm-3-turbo"]),
        6 => ("moonshot", &["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"]),
        7 => ("minimax", &["abab6.5s-chat", "abab6.5g-chat"]),
        8 => ("doubao", &["doubao-lite", "doubao-pro"]),
        9 => ("custom", &["gpt-4", "claude-3"]),
        _ => ("openai", &["gpt-4o"]),
    }
}

fn get_env_var_name(provider: &str) -> String {
    match provider {
        "openai" => "OPENAI_API_KEY".to_string(),
        "anthropic" => "ANTHROPIC_API_KEY".to_string(),
        "google" => "GOOGLE_API_KEY".to_string(),
        "deepseek" => "DEEPSEEK_API_KEY".to_string(),
        "qwen" => "DASHSCOPE_API_KEY".to_string(),
        "zhipu" => "ZHIPUAI_API_KEY".to_string(),
        "moonshot" => "MOONSHOT_API_KEY".to_string(),
        "minimax" => "MINIMAX_API_KEY".to_string(),
        "doubao" => "DOUBAO_API_KEY".to_string(),
        _ => format!("{}_API_KEY", provider.to_uppercase()),
    }
}

fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot find home directory")?;
    let config_dir = home.join(".openclaw");
    Ok(config_dir.join("config.json"))
}

fn load_existing_config(path: &PathBuf) -> Option<OnboardConfig> {
    if path.exists() {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
    } else {
        None
    }
}

fn save_config(path: &PathBuf, config: &OnboardConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;
    
    Ok(())
}
