//! Gateway 命令

use anyhow::Result;
use openclaw_core::Config;
use openclaw_server::Gateway;

pub async fn run(port: u16, host: String, verbose: bool) -> Result<()> {
    if verbose {
        tracing::info!("Verbose mode enabled");
    }

    let mut config = Config::default();
    config.server.port = port;
    config.server.host = host;

    tracing::info!("Starting OpenClaw Gateway...");
    tracing::info!("Configuration: {:?}", config.server);

    let gateway = Gateway::new(config);
    gateway.start().await?;

    Ok(())
}
