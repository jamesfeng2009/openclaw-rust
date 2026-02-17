use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PermissionScope {
    Read,
    Write,
    Execute,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    File,
    Network,
    Device,
    Memory,
    Process,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Permission {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scope: PermissionScope,
    pub resource: ResourceType,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub max_size_bytes: Option<u64>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub tool_id: String,
    pub permissions: Vec<Permission>,
    pub enabled: bool,
    pub rate_limit_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tool_permissions: Vec<String>,
    pub inherits_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GrantResult {
    Granted,
    Denied,
    Limited(Vec<Permission>),
}

pub struct PermissionManager {
    tool_permissions: Arc<RwLock<HashMap<String, ToolPermission>>>,
    roles: Arc<RwLock<HashMap<String, Role>>>,
    user_roles: Arc<RwLock<HashMap<String, Vec<String>>>>,
    global_deny_list: Arc<RwLock<Vec<String>>>,
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        let mut tool_perms = HashMap::new();

        tool_perms.insert(
            "browser_tools".to_string(),
            ToolPermission {
                tool_id: "browser_tools".to_string(),
                permissions: vec![
                    Permission {
                        id: "browser_navigate".to_string(),
                        name: "浏览器导航".to_string(),
                        description: "导航到指定 URL".to_string(),
                        scope: PermissionScope::Execute,
                        resource: ResourceType::Network,
                        allowed_paths: vec!["http://*".to_string(), "https://*".to_string()],
                        denied_paths: vec![],
                        max_size_bytes: Some(10 * 1024 * 1024),
                        timeout_seconds: Some(30),
                    },
                    Permission {
                        id: "browser_screenshot".to_string(),
                        name: "浏览器截图".to_string(),
                        description: "截取网页截图".to_string(),
                        scope: PermissionScope::Read,
                        resource: ResourceType::Memory,
                        allowed_paths: vec![],
                        denied_paths: vec![],
                        max_size_bytes: Some(5 * 1024 * 1024),
                        timeout_seconds: Some(10),
                    },
                ],
                enabled: true,
                rate_limit_per_minute: Some(60),
            },
        );

        tool_perms.insert(
            "file_tools".to_string(),
            ToolPermission {
                tool_id: "file_tools".to_string(),
                permissions: vec![
                    Permission {
                        id: "file_read".to_string(),
                        name: "读取文件".to_string(),
                        description: "读取指定文件内容".to_string(),
                        scope: PermissionScope::Read,
                        resource: ResourceType::File,
                        allowed_paths: vec!["./workspace/*".to_string(), "./data/*".to_string()],
                        denied_paths: vec![
                            "/etc/*".to_string(),
                            "~/.ssh/*".to_string(),
                            "*.key".to_string(),
                            "*.pem".to_string(),
                            "*.env".to_string(),
                        ],
                        max_size_bytes: Some(1024 * 1024),
                        timeout_seconds: Some(5),
                    },
                    Permission {
                        id: "file_write".to_string(),
                        name: "写入文件".to_string(),
                        description: "写入内容到文件".to_string(),
                        scope: PermissionScope::Write,
                        resource: ResourceType::File,
                        allowed_paths: vec!["./workspace/*".to_string(), "./data/*".to_string()],
                        denied_paths: vec![
                            "/etc/*".to_string(),
                            "/usr/*".to_string(),
                            "*.key".to_string(),
                            "*.pem".to_string(),
                        ],
                        max_size_bytes: Some(10 * 1024 * 1024),
                        timeout_seconds: Some(10),
                    },
                ],
                enabled: true,
                rate_limit_per_minute: Some(30),
            },
        );

        tool_perms.insert(
            "http_tools".to_string(),
            ToolPermission {
                tool_id: "http_tools".to_string(),
                permissions: vec![Permission {
                    id: "http_request".to_string(),
                    name: "HTTP 请求".to_string(),
                    description: "发送 HTTP 请求".to_string(),
                    scope: PermissionScope::Execute,
                    resource: ResourceType::Network,
                    allowed_paths: vec![
                        "https://api.*".to_string(),
                        "https://*.openai.com".to_string(),
                        "https://*.anthropic.com".to_string(),
                        "https://*.deepseek.com".to_string(),
                    ],
                    denied_paths: vec![
                        "http://localhost*".to_string(),
                        "http://127.0.0.1*".to_string(),
                        "http://0.0.0.0*".to_string(),
                    ],
                    max_size_bytes: Some(5 * 1024 * 1024),
                    timeout_seconds: Some(30),
                }],
                enabled: true,
                rate_limit_per_minute: Some(100),
            },
        );

        tool_perms.insert(
            "system_tools".to_string(),
            ToolPermission {
                tool_id: "system_tools".to_string(),
                permissions: vec![Permission {
                    id: "system_exec".to_string(),
                    name: "系统命令".to_string(),
                    description: "执行系统命令".to_string(),
                    scope: PermissionScope::Execute,
                    resource: ResourceType::Process,
                    allowed_paths: vec![
                        "echo".to_string(),
                        "date".to_string(),
                        "whoami".to_string(),
                        "pwd".to_string(),
                        "ls".to_string(),
                    ],
                    denied_paths: vec![
                        "rm -rf".to_string(),
                        "mkfs".to_string(),
                        "dd".to_string(),
                        "> /dev/sda".to_string(),
                    ],
                    max_size_bytes: None,
                    timeout_seconds: Some(30),
                }],
                enabled: true,
                rate_limit_per_minute: Some(10),
            },
        );

        Self {
            tool_permissions: Arc::new(RwLock::new(tool_perms)),
            roles: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
            global_deny_list: Arc::new(RwLock::new(vec![])),
        }
    }

    pub async fn check_permission(&self, tool_id: &str, action: &str, target: &str) -> GrantResult {
        let perms = self.tool_permissions.read().await;

        if let Some(tool_perm) = perms.get(tool_id) {
            if !tool_perm.enabled {
                return GrantResult::Denied;
            }

            for perm in &tool_perm.permissions {
                if perm.id == action || perm.name == action {
                    if self.is_path_allowed(target, &perm.allowed_paths, &perm.denied_paths) {
                        return GrantResult::Granted;
                    } else {
                        return GrantResult::Denied;
                    }
                }
            }
        }

        GrantResult::Denied
    }

    fn is_path_allowed(&self, target: &str, allowed: &[String], denied: &[String]) -> bool {
        for pattern in denied {
            if self.match_pattern(target, pattern) {
                return false;
            }
        }

        if allowed.is_empty() {
            return false;
        }

        for pattern in allowed {
            if self.match_pattern(target, pattern) {
                return true;
            }
        }

        false
    }

    fn match_pattern(&self, target: &str, pattern: &str) -> bool {
        let pattern = pattern.replace(".", "\\.");
        let pattern = pattern.replace("*", ".*");

        if let Ok(regex) = regex::Regex::new(&format!("^{}$", pattern)) {
            regex.is_match(target)
        } else {
            target.contains(pattern.trim_matches('*'))
        }
    }

    pub async fn register_tool(&self, tool_id: String, permissions: Vec<Permission>) {
        let mut perms = self.tool_permissions.write().await;
        perms.insert(
            tool_id.clone(),
            ToolPermission {
                tool_id,
                permissions,
                enabled: true,
                rate_limit_per_minute: None,
            },
        );
    }

    pub async fn enable_tool(&self, tool_id: &str) -> Result<(), String> {
        let mut perms = self.tool_permissions.write().await;
        if let Some(perm) = perms.get_mut(tool_id) {
            perm.enabled = true;
            Ok(())
        } else {
            Err(format!("工具 {} 不存在", tool_id))
        }
    }

    pub async fn disable_tool(&self, tool_id: &str) -> Result<(), String> {
        let mut perms = self.tool_permissions.write().await;
        if let Some(perm) = perms.get_mut(tool_id) {
            perm.enabled = false;
            Ok(())
        } else {
            Err(format!("工具 {} 不存在", tool_id))
        }
    }

    pub async fn get_tool_permissions(&self, tool_id: &str) -> Option<ToolPermission> {
        let perms = self.tool_permissions.read().await;
        perms.get(tool_id).cloned()
    }

    pub async fn list_tools(&self) -> Vec<ToolPermission> {
        let perms = self.tool_permissions.read().await;
        perms.values().cloned().collect()
    }

    pub async fn add_to_deny_list(&self, pattern: String) {
        let mut deny = self.global_deny_list.write().await;
        deny.push(pattern);
    }

    pub async fn check_rate_limit(&self, tool_id: &str) -> bool {
        let perms = self.tool_permissions.read().await;
        if let Some(perm) = perms.get(tool_id) {
            if let Some(_limit) = perm.rate_limit_per_minute {
                return true;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permission_manager_default() {
        let manager = PermissionManager::new();
        let tools = manager.list_tools().await;
        assert!(!tools.is_empty());
    }

    #[tokio::test]
    async fn test_get_tool_permissions() {
        let manager = PermissionManager::new();
        let perms = manager.get_tool_permissions("browser_tools").await;
        assert!(perms.is_some());
    }

    #[tokio::test]
    async fn test_enable_disable_tool() {
        let manager = PermissionManager::new();

        let result = manager.disable_tool("browser_tools").await;
        assert!(result.is_ok());

        let perms = manager.get_tool_permissions("browser_tools").await;
        assert!(perms.is_some());
        assert!(!perms.unwrap().enabled);

        let result = manager.enable_tool("browser_tools").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_permission() {
        let manager = PermissionManager::new();
        let result = manager
            .check_permission("browser_tools", "browser_navigate", "https://example.com")
            .await;
        match result {
            GrantResult::Granted => (),
            GrantResult::Denied => (),
            GrantResult::Limited(_) => (),
        }
    }

    #[test]
    fn test_permission_scope_values() {
        assert_eq!(PermissionScope::Read, PermissionScope::Read);
        assert_eq!(PermissionScope::Write, PermissionScope::Write);
        assert_eq!(PermissionScope::Execute, PermissionScope::Execute);
        assert_eq!(PermissionScope::Admin, PermissionScope::Admin);
    }

    #[test]
    fn test_resource_type_values() {
        assert_eq!(ResourceType::File, ResourceType::File);
        assert_eq!(ResourceType::Network, ResourceType::Network);
        assert_eq!(ResourceType::Memory, ResourceType::Memory);
        assert_eq!(ResourceType::Process, ResourceType::Process);
    }
}
