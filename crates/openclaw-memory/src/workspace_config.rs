use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspacesConfig {
    #[serde(default)]
    pub workspaces: Vec<WorkspaceConfig>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub channels: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub agent_ids: Vec<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl WorkspaceConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            path: path.into(),
            channels: HashMap::new(),
            agent_ids: Vec::new(),
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspaces_config_default() {
        let config = WorkspacesConfig::default();
        assert!(config.workspaces.is_empty());
        assert!(config.default.is_none());
    }

    #[test]
    fn test_workspace_config_new() {
        let ws = WorkspaceConfig::new("test-ws", "Test Workspace", "/tmp/workspace");
        assert_eq!(ws.id, "test-ws");
        assert_eq!(ws.name, "Test Workspace");
        assert_eq!(ws.path, PathBuf::from("/tmp/workspace"));
        assert!(ws.enabled);
        assert!(ws.channels.is_empty());
        assert!(ws.agent_ids.is_empty());
    }

    #[test]
    fn test_workspace_config_with_values() {
        let ws = WorkspaceConfig {
            id: "ws1".to_string(),
            name: "Workspace 1".to_string(),
            path: PathBuf::from("/data/ws1"),
            channels: HashMap::new(),
            agent_ids: vec!["agent1".to_string()],
            enabled: false,
        };
        assert_eq!(ws.id, "ws1");
        assert!(!ws.enabled);
        assert_eq!(ws.agent_ids.len(), 1);
    }

    #[test]
    fn test_workspaces_config_serialize_deserialize() {
        let config = WorkspacesConfig {
            workspaces: vec![WorkspaceConfig::new("ws1", "Workspace 1", "/data/ws1")],
            default: Some("ws1".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: WorkspacesConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.workspaces.len(), 1);
        assert_eq!(parsed.default, Some("ws1".to_string()));
    }

    #[test]
    fn test_workspace_config_without_optional_fields() {
        let json = r#"{
            "id": "test",
            "name": "Test",
            "path": "/path/to/workspace"
        }"#;

        let config: WorkspaceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "test");
        assert!(config.enabled);
        assert!(config.channels.is_empty());
        assert!(config.agent_ids.is_empty());
    }
}
