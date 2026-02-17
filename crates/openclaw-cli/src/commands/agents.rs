//! Agents 命令

use anyhow::Result;

use crate::AgentCommands;

pub async fn run(command: AgentCommands) -> Result<()> {
    match command {
        AgentCommands::List => {
            list_agents().await?;
        }
        AgentCommands::Add { id } => {
            add_agent(&id).await?;
        }
        AgentCommands::Remove { id } => {
            remove_agent(&id).await?;
        }
    }

    Ok(())
}

async fn list_agents() -> Result<()> {
    use std::path::PathBuf;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openclaw");

    let agents_file = config_dir.join("agents.json");

    println!("🤖 已配置的 Agent:");
    println!();

    if agents_file.exists() {
        let content = std::fs::read_to_string(&agents_file)?;
        let agents: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(arr) = agents.as_array() {
            for (i, agent) in arr.iter().enumerate() {
                let name = agent
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let id = agent
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let description = agent
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                println!("   {}. {}", i + 1, name);
                println!("      ID: {}", id);
                if !description.is_empty() {
                    println!("      描述: {}", description);
                }
                println!();
            }
        }
    } else {
        println!("   (暂无配置)");
        println!();
        println!("默认 Agent:");
        println!("   - default (默认助手)");
    }

    Ok(())
}

async fn add_agent(id: &str) -> Result<()> {
    use std::path::PathBuf;

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openclaw");

    std::fs::create_dir_all(&config_dir)?;

    let agents_file = config_dir.join("agents.json");

    let mut agents: Vec<serde_json::Value> = if agents_file.exists() {
        let content = std::fs::read_to_string(&agents_file)?;
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    if agents
        .iter()
        .any(|a| a.get("id").and_then(|v| v.as_str()) == Some(id))
    {
        println!("⚠️  Agent '{}' 已存在", id);
        return Ok(());
    }

    let new_agent = serde_json::json!({
        "id": id,
        "name": id,
        "description": format!("用户创建的 Agent: {}", id),
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    agents.push(new_agent);

    let content = serde_json::to_string_pretty(&agents)?;
    std::fs::write(&agents_file, content)?;

    println!("✅ Agent '{}' 已添加", id);

    Ok(())
}

async fn remove_agent(id: &str) -> Result<()> {
    use std::path::PathBuf;

    if id == "default" {
        println!("⚠️  不能删除默认 Agent 'default'");
        return Ok(());
    }

    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openclaw");

    let agents_file = config_dir.join("agents.json");

    if !agents_file.exists() {
        println!("⚠️  Agent '{}' 不存在", id);
        return Ok(());
    }

    let content = std::fs::read_to_string(&agents_file)?;
    let mut agents: Vec<serde_json::Value> = serde_json::from_str(&content)?;

    let original_len = agents.len();
    agents.retain(|a| a.get("id").and_then(|v| v.as_str()) != Some(id));

    if agents.len() == original_len {
        println!("⚠️  Agent '{}' 不存在", id);
        return Ok(());
    }

    let content = serde_json::to_string_pretty(&agents)?;
    std::fs::write(&agents_file, content)?;

    println!("✅ Agent '{}' 已删除", id);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_agents_empty() {
        let result = list_agents().await;
        assert!(result.is_ok());
    }
}
