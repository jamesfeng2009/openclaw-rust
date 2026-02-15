use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub input_filter_enabled: bool,
    pub permission_check_enabled: bool,
    pub network_whitelist_enabled: bool,
    pub log_enabled: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            input_filter_enabled: true,
            permission_check_enabled: true,
            network_whitelist_enabled: true,
            log_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: i64,
    pub event_type: String,
    pub tool_id: Option<String>,
    pub user_id: Option<String>,
    pub details: String,
    pub result: String,
}
