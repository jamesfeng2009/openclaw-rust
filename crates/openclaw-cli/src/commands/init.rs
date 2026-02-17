//! Init 命令

use anyhow::Result;
use std::path::PathBuf;

pub async fn run(config_path: &str) -> Result<()> {
    // 展开路径
    let path = shellexpand::tilde(config_path).to_string();
    let path = PathBuf::from(path);

    // 创建目录
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 创建默认配置
    let config = openclaw_core::Config::default();
    let content = serde_json::to_string_pretty(&config)?;

    // 写入文件
    tokio::fs::write(&path, content).await?;

    println!("Configuration initialized at: {}", path.display());
    println!("\nDefault configuration:");
    println!(
        "  Server: http://{}:{}",
        config.server.host, config.server.port
    );
    println!("  Default AI Provider: {}", config.ai.default_provider);
    println!("\nEdit the configuration file to add your API keys.");

    Ok(())
}
