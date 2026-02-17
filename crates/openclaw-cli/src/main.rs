//! OpenClaw Rust CLI - 命令行工具

#[macro_use]
extern crate lazy_static;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;
mod api_key_cmd;
mod channel_cmd;
mod voice_cmd;
mod wizard_cmd;
mod doctor_cmd;
mod daemon_cmd;
mod message_cmd;
mod agent_cmd;
mod skill_cmd;

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
        /// Enable Agent service
        #[arg(long)]
        agents: bool,
        /// Enable Channel service
        #[arg(long)]
        channels: bool,
        /// Enable Voice service
        #[arg(long)]
        voice: bool,
        /// Enable Canvas service
        #[arg(long)]
        canvas: bool,
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
    /// Skill marketplace commands
    Skill {
        #[command(subcommand)]
        command: skill_cmd::SkillCommand,
    },
    /// Initialize configuration
    Init {
        /// Configuration file path
        #[arg(short, long, default_value = "~/.openclaw-rust/openclaw.json")]
        config: String,
    },
    /// Interactive setup wizard
    Wizard {
        /// Skip optional steps
        #[arg(short, long)]
        quick: bool,
        /// Force overwrite existing config
        #[arg(long)]
        force: bool,
    },
    /// System health check
    Doctor {
        /// Fix issues automatically
        #[arg(short, long)]
        fix: bool,
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Daemon service management
    Daemon {
        #[command(subcommand)]
        command: daemon_cmd::DaemonCommand,
    },
    /// Send a message to a channel
    Message {
        #[command(subcommand)]
        command: message_cmd::MessageCommand,
    },
    /// Talk to the AI assistant
    Agent {
        /// Agent ID (default: default)
        #[arg(long, default_value = "default")]
        agent: String,
        /// Message to send to the agent
        #[arg(short, long)]
        message: Option<String>,
        /// Thinking mode (low, medium, high)
        #[arg(long, default_value = "medium")]
        thinking: String,
        /// Stream the response
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        stream: bool,
        /// Continue the last conversation
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        continue_conv: bool,
        /// System prompt override
        #[arg(long)]
        system: Option<String>,
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

    openclaw_device::init_device().await?;

    match cli.command {
        Commands::Gateway { port, host, verbose, agents, channels, voice, canvas } => {
            commands::gateway::run(port, host, verbose, agents, channels, voice, canvas).await?;
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
        Commands::Skill { command } => {
            skill_cmd::execute(command).await?;
        }
        Commands::Init { config } => {
            commands::init::run(&config).await?;
        }
        Commands::Wizard { quick, force } => {
            wizard_cmd::run(quick, force).await?;
        }
        Commands::Doctor { fix, verbose } => {
            doctor_cmd::run(fix, verbose).await?;
        }
        Commands::Daemon { command } => {
            daemon_cmd::execute(command).await?;
        }
        Commands::Message { command } => {
            command.execute().await?;
        }
        Commands::Agent { agent, message, thinking, stream, continue_conv, system } => {
            let cli = agent_cmd::AgentCli {
                agent,
                message,
                thinking,
                stream,
                continue_conv,
                system,
            };
            cli.run().await?;
        }
        Commands::Version => {
            println!("OpenClaw Rust {}", env!("CARGO_PKG_VERSION"));
        }
    }

    Ok(())
}
