use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::input_filter::{FilterResult, InputFilter, ThreatLevel};
use crate::permission::{GrantResult, PermissionManager};
use crate::sandbox::{NetworkDecision, NetworkWhitelist};

pub struct SecureToolExecutor<T> {
    inner: T,
    input_filter: InputFilter,
    permission_manager: Arc<PermissionManager>,
    network_whitelist: Arc<NetworkWhitelist>,
    tool_id: String,
}

impl<T> SecureToolExecutor<T> {
    pub fn new(inner: T, tool_id: &str) -> Self {
        Self {
            inner,
            input_filter: InputFilter::new(),
            permission_manager: Arc::new(PermissionManager::new()),
            network_whitelist: Arc::new(NetworkWhitelist::new()),
            tool_id: tool_id.to_string(),
        }
    }

    pub async fn check_input(&self, input: &str) -> Result<FilterResult, String> {
        let result = self.input_filter.check(input).await;
        
        if result.threat_level >= ThreatLevel::Medium {
            warn!(
                "Input filter: threat level {:?} for tool {}",
                result.threat_level, self.tool_id
            );
        }
        
        Ok(result)
    }

    pub async fn check_permission(&self, action: &str, target: &str) -> Result<GrantResult, String> {
        let result = self.permission_manager
            .check_permission(&self.tool_id, action, target)
            .await;
        
        match &result {
            GrantResult::Granted => debug!("Permission granted for {} on {}", action, target),
            GrantResult::Denied => warn!("Permission denied for {} on {}", action, target),
            GrantResult::Limited(_perms) => info!("Permission limited for {} on {}", action, target),
        }
        
        Ok(result)
    }

    pub async fn check_network(&self, host: &str, port: u16) -> Result<NetworkDecision, String> {
        let decision = self.network_whitelist
            .check_request(&self.tool_id, host, port, None)
            .await;
        
        match decision {
            NetworkDecision::Allow => debug!("Network allowed: {}:{}", host, port),
            NetworkDecision::Deny => warn!("Network denied: {}:{}", host, port),
            NetworkDecision::Limited => info!("Network limited: {}:{}", host, port),
        }
        
        Ok(decision)
    }
}

pub struct SecurityMiddleware {
    input_filter: InputFilter,
    permission_manager: Arc<PermissionManager>,
    network_whitelist: Arc<NetworkWhitelist>,
}

impl Default for SecurityMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityMiddleware {
    pub fn new() -> Self {
        Self {
            input_filter: InputFilter::new(),
            permission_manager: Arc::new(PermissionManager::new()),
            network_whitelist: Arc::new(NetworkWhitelist::new()),
        }
    }

    pub async fn check_user_input(&self, input: &str) -> FilterResult {
        self.input_filter.check(input).await
    }

    pub async fn check_tool_permission(&self, tool_id: &str, action: &str, target: &str) -> GrantResult {
        self.permission_manager
            .check_permission(tool_id, action, target)
            .await
    }

    pub async fn check_network_request(&self, tool_id: &str, host: &str, port: u16) -> NetworkDecision {
        self.network_whitelist
            .check_request(tool_id, host, port, None)
            .await
    }

    pub async fn execute_secure<F, R, Fut>(
        &self,
        tool_id: &str,
        action: &str,
        target: &str,
        f: F,
    ) -> Result<R, String>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<R, String>>,
    {
        let perm_result = self.check_tool_permission(tool_id, action, target).await;
        
        match perm_result {
            GrantResult::Granted => {
                f().await
            }
            GrantResult::Denied => {
                Err(format!("Permission denied for {}/{}", tool_id, action))
            }
            GrantResult::Limited(_) => {
                warn!("Tool execution with limited permissions: {}/{}", tool_id, action);
                f().await
            }
        }
    }

    pub fn get_input_filter(&self) -> &InputFilter {
        &self.input_filter
    }

    pub fn get_permission_manager(&self) -> Arc<PermissionManager> {
        Arc::clone(&self.permission_manager)
    }

    pub fn get_network_whitelist(&self) -> Arc<NetworkWhitelist> {
        Arc::clone(&self.network_whitelist)
    }
}
