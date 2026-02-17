use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicy {
    pub allow_all: bool,
    pub allowed_domains: HashSet<String>,
    pub allowed_ips: HashSet<String>,
    pub allowed_ports: HashSet<u16>,
    pub denied_domains: HashSet<String>,
    pub denied_ips: HashSet<String>,
    pub max_request_size: u64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxNetworkConfig {
    pub enabled: bool,
    pub default_policy: NetworkPolicy,
    pub tool_policies: HashMap<String, NetworkPolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NetworkDecision {
    Allow,
    Deny,
    Limited,
}

pub struct NetworkWhitelist {
    config: Arc<RwLock<SandboxNetworkConfig>>,
}

impl Default for NetworkWhitelist {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkWhitelist {
    pub fn new() -> Self {
        let mut allowed_domains = HashSet::new();
        allowed_domains.insert("api.openai.com".to_string());
        allowed_domains.insert("api.anthropic.com".to_string());
        allowed_domains.insert("api.deepseek.com".to_string());
        allowed_domains.insert("api.moonshot.cn".to_string());
        allowed_domains.insert("api.minimax.chat".to_string());
        allowed_domains.insert("open.bigmodel.cn".to_string());
        allowed_domains.insert("dashscope.aliyuncs.com".to_string());

        let mut allowed_ips = HashSet::new();
        allowed_ips.insert("127.0.0.1".to_string());

        let mut denied_ips = HashSet::new();
        denied_ips.insert("169.254.169.254".to_string());

        let default_policy = NetworkPolicy {
            allow_all: false,
            allowed_domains,
            allowed_ips,
            allowed_ports: vec![80, 443].into_iter().collect(),
            denied_domains: HashSet::new(),
            denied_ips,
            max_request_size: 10 * 1024 * 1024,
            timeout_seconds: 30,
        };

        let mut tool_policies = HashMap::new();

        let browser_policy = NetworkPolicy {
            allow_all: false,
            allowed_domains: vec!["*".to_string()].into_iter().collect(),
            allowed_ips: HashSet::new(),
            allowed_ports: vec![80, 443, 8080, 8443].into_iter().collect(),
            denied_domains: HashSet::new(),
            denied_ips: HashSet::new(),
            max_request_size: 50 * 1024 * 1024,
            timeout_seconds: 60,
        };
        tool_policies.insert("browser_tools".to_string(), browser_policy);

        let http_policy = NetworkPolicy {
            allow_all: false,
            allowed_domains: vec![
                "api.openai.com".to_string(),
                "api.anthropic.com".to_string(),
                "api.deepseek.com".to_string(),
                "*.openai.com".to_string(),
                "*.anthropic.com".to_string(),
            ]
            .into_iter()
            .collect(),
            allowed_ips: HashSet::new(),
            allowed_ports: vec![443].into_iter().collect(),
            denied_domains: HashSet::new(),
            denied_ips: vec![
                "169.254.169.254".to_string(),
                "metadata.google.internal".to_string(),
            ]
            .into_iter()
            .collect(),
            max_request_size: 10 * 1024 * 1024,
            timeout_seconds: 30,
        };
        tool_policies.insert("http_tools".to_string(), http_policy);

        let config = SandboxNetworkConfig {
            enabled: true,
            default_policy,
            tool_policies,
        };

        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub async fn check_request(
        &self,
        tool_id: &str,
        host: &str,
        port: u16,
        _path: Option<&str>,
    ) -> NetworkDecision {
        let config = self.config.read().await;

        if !config.enabled {
            return NetworkDecision::Allow;
        }

        let policy = config
            .tool_policies
            .get(tool_id)
            .unwrap_or(&config.default_policy);

        if policy.allow_all {
            return NetworkDecision::Allow;
        }

        if self.is_denied(host, &policy.denied_domains, &policy.denied_ips) {
            warn!("Network request denied by deny list: {:?}", host);
            return NetworkDecision::Deny;
        }

        if self.is_allowed(
            host,
            port,
            &policy.allowed_domains,
            &policy.allowed_ips,
            &policy.allowed_ports,
        ) {
            debug!("Network request allowed: {}:{}", host, port);
            return NetworkDecision::Allow;
        }

        warn!("Network request not in whitelist: {}:{:?}", host, port);
        NetworkDecision::Deny
    }

    fn is_denied(
        &self,
        host: &str,
        denied_domains: &HashSet<String>,
        denied_ips: &HashSet<String>,
    ) -> bool {
        let host_lower = host.to_lowercase();

        for pattern in denied_domains {
            if pattern.starts_with("*.") {
                let suffix = &pattern[2..];
                if host_lower.ends_with(suffix) || host_lower == suffix {
                    return true;
                }
            } else if host_lower == pattern.to_lowercase() {
                return true;
            }
        }

        for ip in denied_ips {
            if host == ip {
                return true;
            }
        }

        false
    }

    fn is_allowed(
        &self,
        host: &str,
        port: u16,
        allowed_domains: &HashSet<String>,
        allowed_ips: &HashSet<String>,
        allowed_ports: &HashSet<u16>,
    ) -> bool {
        if !allowed_ports.is_empty() && !allowed_ports.contains(&port) {
            return false;
        }

        if allowed_ips.contains(&host.to_string()) {
            return true;
        }

        let host_lower = host.to_lowercase();

        for pattern in allowed_domains {
            if pattern == "*" {
                return true;
            }

            if pattern.starts_with("*.") {
                let suffix = &pattern[2..];
                if host_lower.ends_with(suffix) || host_lower == suffix {
                    return true;
                }
            } else if host_lower == pattern.to_lowercase() {
                return true;
            }
        }

        false
    }

    pub async fn add_allowed_domain(&self, domain: String) {
        let mut config = self.config.write().await;
        config.default_policy.allowed_domains.insert(domain);
    }

    pub async fn add_allowed_ip(&self, ip: String) {
        let mut config = self.config.write().await;
        config.default_policy.allowed_ips.insert(ip);
    }

    pub async fn add_denied_domain(&self, domain: String) {
        let mut config = self.config.write().await;
        config.default_policy.denied_domains.insert(domain);
    }

    pub async fn set_tool_policy(&self, tool_id: String, policy: NetworkPolicy) {
        let mut config = self.config.write().await;
        config.tool_policies.insert(tool_id, policy);
    }

    pub async fn get_policy(&self, tool_id: &str) -> Option<NetworkPolicy> {
        let config = self.config.read().await;
        config.tool_policies.get(tool_id).cloned()
    }

    pub async fn get_default_policy(&self) -> NetworkPolicy {
        let config = self.config.read().await;
        config.default_policy.clone()
    }

    pub async fn enable(&self) {
        let mut config = self.config.write().await;
        config.enabled = true;
    }

    pub async fn disable(&self) {
        let mut config = self.config.write().await;
        config.enabled = false;
    }

    pub async fn is_enabled(&self) -> bool {
        let config = self.config.read().await;
        config.enabled
    }
}

use std::collections::HashMap;
