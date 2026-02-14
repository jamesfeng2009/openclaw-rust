//! OpenClaw Rust CLI - 命令行工具

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod api_key_cmd;
mod channel_cmd;
mod voice_cmd;

#[derive(Parser)]
#[command(name = "openclaw-rust")]
#[command(about = "OpenClaw Rust - Your personal AI assistant (Rust implementation)", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the gateway server
    Gateway {
        /// Port to listen on
        #[arg(short, long, default_value = "18789")]
        port: u16,
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Manage agents
    Agents {
        #[command(subcommand)]
        command: AgentCommands,
    },
    /// Manage API keys
    ApiKey {
        #[command(subcommand)]
        command: api_key_cmd::ApiKeyCommand,
    },
    /// Manage channel configurations
    Channel {
        #[command(subcommand)]
        command: channel_cmd::ChannelCommand,
    },
    /// Voice commands (STT/TTS/Talk Mode)
    Voice {
        #[command(subcommand)]
        command: voice_cmd::VoiceCommand,
    },
    /// Initialize configuration
    Init {
        /// Configuration file path
        #[arg(short, long, default_value = "~/.openclaw/openclaw.json")]
        config: String,
    },
    /// Show version info
    Version,
}

#[derive(Subcommand)]
enum AgentCommands {
    /// List all agents
    List,
    /// Add a new agent
    Add {
        /// Agent ID
        id: String,
    },
    /// Remove an agent
    Remove {
        /// Agent ID
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "openclaw=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Gateway { port, host, verbose } => {
            commands::gateway::run(port, host, verbose).await?;
        }
        Commands::Agents { command } => {
            commands::agents::run(command).await?;
        }
        Commands::ApiKey { command } => {
            command.execute().await?;
        }
        Commands::Channel { command } => {
            command.execute().await?;
        }
        Commands::Voice { command } => {
            command.execute().await?;
        }
        Commands::Init { config } => {
            commands::init::run(&config).await?;
        }
        Commands::Version => {
            println!("OpenClaw Rust {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
