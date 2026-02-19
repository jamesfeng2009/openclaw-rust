//! 设置向导命令

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 向导配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardConfig {
    pub user_name: String,
    pub default_provider: String,
    pub default_model: String,
    pub api_keys: HashMap<String, String>,
    pub enabled_features: Vec<String>,
    pub voice_enabled: bool,
    pub voice_provider: Option<String>,
    pub channels_enabled: Vec<String>,
    pub browser_headless: bool,
    pub sandbox_enabled: bool,
}

impl Default for WizardConfig {
    fn default() -> Self {
        Self {
            user_name: String::new(),
            default_provider: "openai".to_string(),
            default_model: "gpt-4o".to_string(),
            api_keys: HashMap::new(),
            enabled_features: vec!["chat".to_string()],
            voice_enabled: false,
            voice_provider: None,
            channels_enabled: vec![],
            browser_headless: true,
            sandbox_enabled: false,
        }
    }
}

/// 运行设置向导
pub async fn run(quick: bool, force: bool) -> Result<()> {
    println!("\n🧙 OpenClaw 设置向导\n");
    println!("欢迎使用 OpenClaw！让我帮您完成初始配置。\n");

    // 检查现有配置
    let config_path = get_config_path()?;
    if config_path.exists() && !force {
        let overwrite = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("配置文件已存在，是否覆盖？")
            .default(false)
            .interact()?;

        if !overwrite {
            println!("\n已取消设置向导。");
            return Ok(());
        }
    }

    let mut config = WizardConfig::default();

    // 1. 用户信息
    println!("\n📝 基本设置\n");

    config.user_name = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("请输入您的名字")
        .default("User".to_string())
        .interact()?;

    // 2. AI 提供商选择
    println!("\n🤖 AI 提供商设置\n");

    let providers = vec![
        "OpenAI",
        "Anthropic (Claude)",
        "Google (Gemini)",
        "DeepSeek",
        "通义千问 (Qwen)",
        "智谱 GLM",
        "Moonshot (Kimi)",
    ];

    let provider_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("选择默认 AI 提供商")
        .items(&providers)
        .default(0)
        .interact()?;

    config.default_provider = match provider_idx {
        0 => "openai",
        1 => "anthropic",
        2 => "google",
        3 => "deepseek",
        4 => "qwen",
        5 => "zhipu",
        6 => "moonshot",
        _ => "openai",
    }
    .to_string();

    // 3. API Key 输入
    println!("\n🔑 API 密钥设置\n");

    let key_name = format!("{}_API_KEY", config.default_provider.to_uppercase());
    let key_prompt = format!("请输入 {} API Key (留空跳过)", providers[provider_idx]);

    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt(key_prompt)
        .allow_empty(true)
        .interact()?;

    if !api_key.is_empty() {
        config.api_keys.insert(key_name.clone(), api_key);
    }

    // 快速模式跳过可选步骤
    if !quick {
        // 4. 默认模型
        let models = get_models_for_provider(&config.default_provider);
        let model_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("选择默认模型")
            .items(&models)
            .default(0)
            .interact()?;
        config.default_model = models[model_idx].to_string();

        // 5. 功能选择
        println!("\n⚡ 功能设置\n");

        let features = vec![
            "对话聊天",
            "语音识别 (STT)",
            "语音合成 (TTS)",
            "浏览器控制",
            "实时画布",
            "定时任务",
            "Webhook",
            "Docker 沙箱",
        ];

        let selected = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("选择要启用的功能 (空格选择，回车确认)")
            .items(&features)
            .defaults(&[true])
            .interact()?;

        config.enabled_features = selected.iter().map(|&i| features[i].to_string()).collect();

        // 6. 语音设置
        config.voice_enabled = config.enabled_features.iter().any(|f| f.contains("语音"));

        if config.voice_enabled {
            let voice_providers = vec!["OpenAI Whisper", "本地 Whisper", "Edge TTS"];
            let voice_idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("选择语音服务")
                .items(&voice_providers)
                .default(0)
                .interact()?;
            config.voice_provider = Some(voice_providers[voice_idx].to_string());
        }

        // 7. 浏览器设置
        config.browser_headless = !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("浏览器是否显示窗口？(用于调试)")
            .default(false)
            .interact()?;

        // 8. 沙箱设置
        config.sandbox_enabled = config.enabled_features.iter().any(|f| f.contains("沙箱"));

        // 9. 频道设置
        println!("\n📡 消息频道设置\n");

        let channels = vec!["Telegram", "Discord", "钉钉", "企业微信", "飞书", "Slack"];

        let selected_channels = MultiSelect::with_theme(&ColorfulTheme::default())
            .with_prompt("选择要启用的消息频道 (可选)")
            .items(&channels)
            .interact()?;

        config.channels_enabled = selected_channels
            .iter()
            .map(|&i| channels[i].to_lowercase())
            .collect();
    }

    // 保存配置
    save_config(&config_path, &config)?;

    println!("\n✅ 配置完成！\n");
    println!("配置文件已保存到: {}", config_path.display());
    println!("\n下一步:");
    println!("  • 运行 `openclaw-rust doctor` 检查系统状态");
    println!("  • 运行 `openclaw-rust gateway` 启动服务");
    println!();

    Ok(())
}

/// 获取配置路径
fn get_config_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法获取用户主目录")?;
    Ok(home.join(".openclaw-rust").join("openclaw.json"))
}

/// 保存配置
fn save_config(path: &Path, config: &WizardConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(config)?;
    fs::write(path, content)?;

    Ok(())
}

/// 获取提供商对应的模型列表
fn get_models_for_provider(provider: &str) -> Vec<&'static str> {
    match provider {
        "openai" => vec!["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o3"],
        "anthropic" => vec![
            "claude-4",
            "claude-3.7-sonnet",
            "claude-3.5-sonnet",
            "claude-3-opus",
        ],
        "google" => vec!["gemini-2.0-flash", "gemini-1.5-pro", "gemini-1.5-flash"],
        "deepseek" => vec!["deepseek-chat", "deepseek-coder", "deepseek-reasoner"],
        "qwen" => vec!["qwen-max", "qwen-plus", "qwen-turbo", "qwen-vl"],
        "zhipu" => vec!["glm-4", "glm-4-plus", "glm-3-turbo"],
        "moonshot" => vec!["moonshot-v1-8k", "moonshot-v1-32k", "moonshot-v1-128k"],
        _ => vec!["default"],
    }
}
