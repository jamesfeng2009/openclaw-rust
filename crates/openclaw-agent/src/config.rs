use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    pub list: Vec<AgentInstanceConfig>,
    pub defaults: AgentDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstanceConfig {
    pub id: String,
    pub workspace: PathBuf,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub aieos_path: Option<PathBuf>,
}

impl AgentInstanceConfig {
    pub fn new(id: impl Into<String>, workspace: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            workspace: workspace.into(),
            default: false,
            aieos_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub model: String,
    pub provider: String,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agents_config_default() {
        let config = AgentsConfig::default();
        assert!(config.list.is_empty());
        assert_eq!(config.defaults.model, "gpt-4o");
        assert_eq!(config.defaults.provider, "openai");
    }

    #[test]
    fn test_agent_config_with_values() {
        let config = AgentInstanceConfig {
            id: "test_agent".to_string(),
            workspace: PathBuf::from("/tmp/workspace"),
            default: true,
            aieos_path: Some(PathBuf::from("/path/to/aieos")),
        };
        assert_eq!(config.id, "test_agent");
        assert!(config.default);
        assert!(config.aieos_path.is_some());
    }

    #[test]
    fn test_agent_defaults() {
        let defaults = AgentDefaults::default();
        assert_eq!(defaults.model, "gpt-4o");
        assert_eq!(defaults.provider, "openai");
    }

    #[test]
    fn test_agents_config_serialize_deserialize() {
        let config = AgentsConfig {
            list: vec![AgentInstanceConfig {
                id: "assistant".to_string(),
                workspace: PathBuf::from("./workspace/assistant"),
                default: true,
                aieos_path: None,
            }],
            defaults: AgentDefaults {
                model: "gpt-4o".to_string(),
                provider: "openai".to_string(),
            },
        };

        let json = serde_json::to_string(&config).unwrap();
        let parsed: AgentsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.list.len(), 1);
        assert_eq!(parsed.list[0].id, "assistant");
        assert!(parsed.list[0].default);
        assert_eq!(parsed.defaults.model, "gpt-4o");
    }

    #[test]
    fn test_agent_config_without_optional_fields() {
        let json = r#"{
            "id": "test",
            "workspace": "/path/to/workspace"
        }"#;

        let config: AgentInstanceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.id, "test");
        assert!(!config.default);
        assert!(config.aieos_path.is_none());
    }

    #[test]
    fn test_multiple_agents() {
        let config = AgentsConfig {
            list: vec![
                AgentInstanceConfig {
                    id: "agent1".to_string(),
                    workspace: PathBuf::from("/ws/agent1"),
                    default: true,
                    aieos_path: None,
                },
                AgentInstanceConfig {
                    id: "agent2".to_string(),
                    workspace: PathBuf::from("/ws/agent2"),
                    default: false,
                    aieos_path: Some(PathBuf::from("/aieos")),
                },
            ],
            defaults: AgentDefaults::default(),
        };

        assert_eq!(config.list.len(), 2);
        assert!(config.list[0].default);
        assert!(!config.list[1].default);
    }
}
