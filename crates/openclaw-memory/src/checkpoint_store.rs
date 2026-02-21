use std::path::PathBuf;
use tokio::sync::RwLock;
use std::sync::Arc;

use super::checkpoint::{AgentState, Checkpoint};

pub struct CheckpointStore {
    checkpoints: Arc<RwLock<Vec<Checkpoint>>>,
    storage_path: PathBuf,
}

impl CheckpointStore {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            storage_path,
        }
    }

    pub async fn save_checkpoint(&self, checkpoint: Checkpoint) -> Result<(), String> {
        let mut checkpoints = self.checkpoints.write().await;
        
        if let Some(existing) = checkpoints.iter_mut().find(|c| c.agent_id == checkpoint.agent_id && c.session_id == checkpoint.session_id) {
            if checkpoint.sequence_number > existing.sequence_number {
                *existing = checkpoint;
            }
        } else {
            checkpoints.push(checkpoint);
        }
        
        self.persist_to_disk(&checkpoints).await?;
        Ok(())
    }

    pub async fn load_checkpoint(&self, agent_id: &str, session_id: &str) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints
            .iter()
            .find(|c| c.agent_id == agent_id && c.session_id == session_id)
            .cloned()
    }

    pub async fn get_latest_checkpoint(&self, agent_id: &str) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints
            .iter()
            .filter(|c| c.agent_id == agent_id)
            .max_by_key(|c| c.sequence_number)
            .cloned()
    }

    pub async fn get_checkpoints_by_session(&self, session_id: &str) -> Vec<Checkpoint> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints
            .iter()
            .filter(|c| c.session_id == session_id)
            .cloned()
            .collect()
    }

    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<(), String> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.retain(|c| c.id != checkpoint_id);
        self.persist_to_disk(&checkpoints).await?;
        Ok(())
    }

    pub async fn clear_agent_checkpoints(&self, agent_id: &str) -> Result<(), String> {
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.retain(|c| c.agent_id != agent_id);
        self.persist_to_disk(&checkpoints).await?;
        Ok(())
    }

    async fn persist_to_disk(&self, checkpoints: &[Checkpoint]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(checkpoints)
            .map_err(|e| format!("Failed to serialize checkpoints: {}", e))?;
        
        tokio::fs::write(&self.storage_path, json)
            .await
            .map_err(|e| format!("Failed to write checkpoints to disk: {}", e))?;
        
        Ok(())
    }

    pub async fn load_from_disk(&self) -> Result<(), String> {
        if !self.storage_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.storage_path)
            .await
            .map_err(|e| format!("Failed to read checkpoints from disk: {}", e))?;

        let loaded: Vec<Checkpoint> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse checkpoints: {}", e))?;

        let mut checkpoints = self.checkpoints.write().await;
        *checkpoints = loaded;

        Ok(())
    }

    pub async fn list_agents(&self) -> Vec<String> {
        let checkpoints = self.checkpoints.read().await;
        let mut agents: Vec<String> = checkpoints
            .iter()
            .map(|c| c.agent_id.clone())
            .collect();
        agents.sort();
        agents.dedup();
        agents
    }

    pub async fn list_sessions(&self, agent_id: &str) -> Vec<String> {
        let checkpoints = self.checkpoints.read().await;
        let mut sessions: Vec<String> = checkpoints
            .iter()
            .filter(|c| c.agent_id == agent_id)
            .map(|c| c.session_id.clone())
            .collect();
        sessions.sort();
        sessions.dedup();
        sessions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkpoint::AgentState;

    #[tokio::test]
    async fn test_save_and_load_checkpoint() {
        let store = CheckpointStore::new(std::path::PathBuf::from("/tmp/test_checkpoints.json"));
        
        let state = AgentState::new("agent-1".to_string());
        let checkpoint = Checkpoint::new(
            "agent-1".to_string(),
            "session-1".to_string(),
            state,
            1,
        );
        
        store.save_checkpoint(checkpoint.clone()).await.unwrap();
        
        let loaded = store.load_checkpoint("agent-1", "session-1").await;
        assert!(loaded.is_some());
    }

    #[tokio::test]
    async fn test_get_latest_checkpoint() {
        let store = CheckpointStore::new(std::path::PathBuf::from("/tmp/test_checkpoints2.json"));
        
        let state1 = AgentState::new("agent-1".to_string());
        let checkpoint1 = Checkpoint::new("agent-1".to_string(), "session-1".to_string(), state1, 1);
        store.save_checkpoint(checkpoint1).await.unwrap();
        
        let state2 = AgentState::new("agent-1".to_string());
        let checkpoint2 = Checkpoint::new("agent-1".to_string(), "session-1".to_string(), state2, 2);
        store.save_checkpoint(checkpoint2).await.unwrap();
        
        let latest = store.get_latest_checkpoint("agent-1").await;
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().sequence_number, 2);
    }

    #[tokio::test]
    async fn test_list_agents() {
        let store = CheckpointStore::new(std::path::PathBuf::from("/tmp/test_checkpoints3.json"));
        
        let state1 = AgentState::new("agent-1".to_string());
        let checkpoint1 = Checkpoint::new("agent-1".to_string(), "session-1".to_string(), state1, 1);
        store.save_checkpoint(checkpoint1).await.unwrap();
        
        let state2 = AgentState::new("agent-2".to_string());
        let checkpoint2 = Checkpoint::new("agent-2".to_string(), "session-1".to_string(), state2, 1);
        store.save_checkpoint(checkpoint2).await.unwrap();
        
        let agents = store.list_agents().await;
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_checkpoint() {
        let store = CheckpointStore::new(std::path::PathBuf::from("/tmp/test_checkpoints4.json"));
        
        let state = AgentState::new("agent-1".to_string());
        let checkpoint = Checkpoint::new("agent-1".to_string(), "session-1".to_string(), state, 1);
        let checkpoint_id = checkpoint.id.clone();
        store.save_checkpoint(checkpoint).await.unwrap();
        
        store.delete_checkpoint(&checkpoint_id).await.unwrap();
        
        let loaded = store.load_checkpoint("agent-1", "session-1").await;
        assert!(loaded.is_none());
    }
}
