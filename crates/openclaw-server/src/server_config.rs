use openclaw_agent::AgentsConfig;
use openclaw_device::DevicesConfig;
use openclaw_memory::workspace_config::WorkspacesConfig;
use openclaw_core::Config as CoreConfig;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(flatten)]
    pub core: CoreConfig,
    pub agents: AgentsConfig,
    pub devices: DevicesConfig,
    pub workspaces: WorkspacesConfig,
}

impl ServerConfig {
    pub fn from_core(core: CoreConfig) -> Self {
        Self {
            core,
            agents: AgentsConfig::default(),
            devices: DevicesConfig::default(),
            workspaces: WorkspacesConfig::default(),
        }
    }

    pub fn load(config_dir: &Path) -> std::io::Result<Self> {
        let core = CoreConfig::from_file(&config_dir.join("config.yaml"))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        let agents = load_yaml_config(config_dir.join("agents.yaml"))
            .unwrap_or_default();
        let devices = load_yaml_config(config_dir.join("devices.yaml"))
            .unwrap_or_default();
        let workspaces = load_yaml_config(config_dir.join("workspaces.yaml"))
            .unwrap_or_default();

        Ok(Self {
            core,
            agents,
            devices,
            workspaces,
        })
    }

    pub fn load_or_default(config_dir: &Path) -> Self {
        Self::load(config_dir).unwrap_or_else(|_| Self::default())
    }
}

fn load_yaml_config<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    if !path.exists() {
        return None;
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|content| serde_yaml::from_str(&content).ok())
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            agents: AgentsConfig::default(),
            devices: DevicesConfig::default(),
            workspaces: WorkspacesConfig::default(),
        }
    }
}

impl std::ops::Deref for ServerConfig {
    type Target = CoreConfig;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl std::ops::DerefMut for ServerConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert!(config.agents.list.is_empty());
        assert!(!config.devices.enabled);
        assert!(config.workspaces.workspaces.is_empty());
    }

    #[test]
    fn test_server_config_from_core() {
        let core = CoreConfig::default();
        let config = ServerConfig::from_core(core);
        assert!(config.agents.list.is_empty());
    }

    #[test]
    fn test_load_yaml_config_not_exists() {
        let result: Option<AgentsConfig> = load_yaml_config(PathBuf::from("/nonexistent.yaml"));
        assert!(result.is_none());
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_load_with_valid_config() {
        let temp_dir = TempDir::new().unwrap();
        
        let config_yaml = r#"
server:
  host: "0.0.0.0"
  port: 8080
"#;
        fs::write(temp_dir.path().join("config.yaml"), config_yaml).unwrap();

        let agents_yaml = r#"
list:
  - id: "agent1"
    name: "Agent 1"
defaults:
  provider: "openai"
  model: "gpt-4"
"#;
        fs::write(temp_dir.path().join("agents.yaml"), agents_yaml).unwrap();

        let config = ServerConfig::load(temp_dir.path()).unwrap();
        
        assert_eq!(config.core.server.port, 8080);
        assert_eq!(config.agents.list.len(), 1);
        assert_eq!(config.agents.list[0].id, "agent1");
        assert_eq!(config.agents.defaults.provider, "openai");
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_load_with_missing_files() {
        let temp_dir = TempDir::new().unwrap();
        
        let config_yaml = r#"
server:
  host: "0.0.0.0"
  port: 9090
"#;
        fs::write(temp_dir.path().join("config.yaml"), config_yaml).unwrap();

        let config = ServerConfig::load(temp_dir.path()).unwrap();
        
        assert_eq!(config.core.server.port, 9090);
        assert!(config.agents.list.is_empty());
        assert!(!config.devices.enabled);
    }

    #[test]
    #[cfg(feature = "testing")]
    fn test_load_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        
        let config_yaml = r#"
server:
  host: "0.0.0.0"
"#;
        fs::write(temp_dir.path().join("config.yaml"), config_yaml).unwrap();
        
        let invalid_yaml = "invalid: yaml: content:";
        fs::write(temp_dir.path().join("agents.yaml"), invalid_yaml).unwrap();

        let config = ServerConfig::load(temp_dir.path()).unwrap();
        
        assert_eq!(config.core.server.host, "0.0.0.0");
        assert!(config.agents.list.is_empty());
    }

    #[test]
    fn test_load_or_default_with_invalid_dir() {
        let config = ServerConfig::load_or_default(Path::new("/nonexistent"));
        
        assert_eq!(config.core.server.port, 18789);
    }
}
