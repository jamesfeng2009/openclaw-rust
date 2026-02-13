//! Agents 命令

use anyhow::Result;

use crate::AgentCommands;

pub async fn run(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::List => {
            println!("Agents:");
            println!("  - main (default)");
        }
        AgentCommands::Add { id } => {
            println!("Adding agent: {}", id);
            // TODO: 实际添加逻辑
        }
        AgentCommands::Remove { id } => {
            println!("Removing agent: {}", id);
            // TODO: 实际删除逻辑
        }
    }

    Ok(())
}
