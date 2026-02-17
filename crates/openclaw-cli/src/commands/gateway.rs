//! Gateway 命令

use anyhow::Result;
use openclaw_core::Config;
use openclaw_server::Gateway;

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

    let mut config = Config::default();
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

    let gateway = Gateway::new(config);
    gateway.start().await?;

    Ok(())
}
