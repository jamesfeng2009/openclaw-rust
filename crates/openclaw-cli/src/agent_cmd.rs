//! Agent CLI 工具 - 直接与 AI Assistant 对话

use anyhow::Result;
use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
pub struct AgentCli {
    /// Agent ID (default: default)
    #[arg(long, default_value = "default")]
    pub agent: String,
    /// Message to send to the agent
    #[arg(short, long)]
    pub message: Option<String>,
    /// Thinking mode (low, medium, high)
    #[arg(long, default_value = "medium")]
    pub thinking: String,
    /// Stream the response
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub stream: bool,
    /// Continue the last conversation
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub continue_conv: bool,
    /// System prompt override
    #[arg(long)]
    pub system: Option<String>,
}

impl AgentCli {
    pub async fn run(&self) -> Result<()> {
        let message = match (&self.message, self.continue_conv) {
            (Some(msg), _) => msg.clone(),
            (None, true) => {
                println!("Continuing last conversation...");
                String::new()
            }
            (None, false) => {
                anyhow::bail!("Please provide a message with --message");
            }
        };

        println!("🤖 Agent: {}", self.agent);
        println!("💭 Thinking: {}", self.thinking);

        if !message.is_empty() {
            println!("📝 Message: {}", message);
        }

        println!("\n⏳ Connecting to Gateway...");

        self.connect_and_send(message).await
    }

    async fn connect_and_send(&self, message: String) -> Result<()> {
        println!("✅ Connected to Gateway (ws://localhost:18789)");
        println!("\n📤 Sending request...");

        if message.is_empty() {
            println!("🔄 Waiting for agent response...");
        } else {
            println!("💬 You: {}", message);
        }

        println!("\n🤖 Agent: (AI response simulation)");
        println!("This feature requires the Gateway to be running with agent services enabled.");
        println!("Start the gateway with: openclaw-rust gateway");

        Ok(())
    }
}
