//! Gateway 命令

use anyhow::Result;
use openclaw_core::Config;
use openclaw_server::Gateway;

pub async fn run(port: u16, host: String, verbose: bool, agents: bool, channels: bool, voice: bool) -> Result<()> {
    if verbose {
        tracing::info!("Verbose mode enabled");
    }

    let mut config = Config::default();
    config.server.port = port;
    config.server.host = host;
    config.server.enable_agents = agents;
    config.server.enable_channels = channels;
    config.server.enable_voice = voice;

    tracing::info!("Starting OpenClaw Gateway...");
    tracing::info!("Configuration: {:?}", config.server);
    tracing::info!("Services: agents={}, channels={}, voice={}", agents, channels, voice);

    let gateway = Gateway::new(config);
    gateway.start().await?;

    Ok(())
}
