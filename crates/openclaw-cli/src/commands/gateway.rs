//! Gateway 命令

use std::path::PathBuf;
use anyhow::Result;
use openclaw_core::Config;
use openclaw_server::Gateway;

fn get_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|p| p.join(".openclaw-rust").join("openclaw.json"))
}

pub async fn run(
    port: u16,
    host: String,
    verbose: bool,
    agents: bool,
    channels: bool,
    voice: bool,
    canvas: bool,
) -> Result<()> {
    if verbose {
        tracing::info!("Verbose mode enabled");
    }

    // 尝试从配置文件加载
    let mut config = if let Some(config_path) = get_config_path() {
        if config_path.exists() {
            tracing::info!("Loading configuration from: {:?}", config_path);
            match Config::from_file(&config_path) {
                Ok(cfg) => cfg,
                Err(e) => {
                    tracing::warn!("Failed to load config from {:?}: {}, using defaults", config_path, e);
                    Config::default()
                }
            }
        } else {
            tracing::info!("Config file not found at {:?}, using defaults", config_path);
            Config::default()
        }
    } else {
        tracing::info!("Could not determine config directory, using defaults");
        Config::default()
    };

    // CLI 参数覆盖配置
    config.server.port = port;
    config.server.host = host;
    config.server.enable_agents = agents;
    config.server.enable_channels = channels;
    config.server.enable_voice = voice;
    config.server.enable_canvas = canvas;

    tracing::info!("Starting OpenClaw Gateway...");
    tracing::info!("Configuration: {:?}", config.server);
    tracing::info!(
        "Services: agents={}, channels={}, voice={}, canvas={}",
        agents,
        channels,
        voice,
        canvas
    );

    let gateway = Gateway::new(config).await?;
    gateway.start().await?;

    Ok(())
}
