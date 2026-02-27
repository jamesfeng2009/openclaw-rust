//! ACP Message Router
//!
//! Routes messages to appropriate agents based on @mentions and rules.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::context::ContextManager;
use crate::registry::AgentRegistry;

/// Route rule
#[derive(Debug, Clone)]
pub struct RouteRule {
    pub pattern: Regex,
    pub target_agent: String,
    pub priority: i32,
}

/// Router result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterResult {
    pub target_agent: String,
    pub cleaned_content: String,
    pub matched_rule: Option<String>,
    pub context_id: Option<String>,
}

/// Message Router
#[derive(Clone)]
pub struct Router {
    rules: Vec<RouteRule>,
    agent_registry: Arc<AgentRegistry>,
    context_manager: Arc<ContextManager>,
    default_agent: Option<String>,
}

impl Router {
    pub fn new(agent_registry: Arc<AgentRegistry>, context_manager: Arc<ContextManager>) -> Self {
        Self {
            rules: Vec::new(),
            agent_registry,
            context_manager,
            default_agent: None,
        }
    }

    pub fn with_default_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.default_agent = Some(agent_id.into());
        self
    }

    pub fn add_rule(&mut self, pattern: impl Into<String>, target_agent: impl Into<String>, priority: i32) -> Result<(), regex::Error> {
        let rule = RouteRule {
            pattern: Regex::new(&pattern.into())?,
            target_agent: target_agent.into(),
            priority,
        };
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// Parse @mentions from content
    pub fn parse_mentions(&self, content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        
        // 飞书风格: @机器人名称
        let feishu_pattern = Regex::new(r"@([\w\u4e00-\u9fa5]+)").unwrap();
        for cap in feishu_pattern.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                mentions.push(name.as_str().to_string());
            }
        }

        // Discord 风格: <@!123456789>
        let discord_pattern = Regex::new(r"<@!?(\d+)>").unwrap();
        for _ in discord_pattern.captures_iter(content) {
            mentions.push("discord_mention".to_string());
        }

        mentions
    }

    /// Clean content by removing @mentions
    pub fn clean_content(&self, content: &str) -> String {
        let mut cleaned = content.to_string();
        
        // Remove 飞书 style mentions
        let feishu_pattern = Regex::new(r"@[\w\u4e00-\u9fa5]+").unwrap();
        cleaned = feishu_pattern.replace_all(&cleaned, "").to_string();
        
        // Remove Discord style mentions
        let discord_pattern = Regex::new(r"<@!?\d+>").unwrap();
        cleaned = discord_pattern.replace_all(&cleaned, "").to_string();
        
        cleaned.trim().to_string()
    }

    /// Route a message
    pub async fn route(&self, content: &str, _conversation_id: &str) -> RouterResult {
        let mentions = self.parse_mentions(content);

        // 1. If there's an explicit @mention, route to that agent
        if !mentions.is_empty() {
            for mention in &mentions {
                // Try to find agent by name
                if let Some(agent) = self.agent_registry.get_by_name(mention).await {
                    return RouterResult {
                        target_agent: agent.id,
                        cleaned_content: self.clean_content(content),
                        matched_rule: Some(format!("mention:{}", mention)),
                        context_id: None,
                    };
                }
            }
            
            // If found by mention but not registered, use the mention as target
            return RouterResult {
                target_agent: mentions[0].clone(),
                cleaned_content: self.clean_content(content),
                matched_rule: Some("mention".to_string()),
                context_id: None,
            };
        }

        // 2. Try to match rules
        for rule in &self.rules {
            if rule.pattern.is_match(content) {
                return RouterResult {
                    target_agent: rule.target_agent.clone(),
                    cleaned_content: self.clean_content(content),
                    matched_rule: Some(rule.target_agent.clone()),
                    context_id: None,
                };
            }
        }

        // 3. Use default agent
        if let Some(default) = &self.default_agent {
            return RouterResult {
                target_agent: default.clone(),
                cleaned_content: self.clean_content(content),
                matched_rule: Some("default".to_string()),
                context_id: None,
            };
        }

        // 4. No route found
        RouterResult {
            target_agent: String::new(),
            cleaned_content: self.clean_content(content),
            matched_rule: None,
            context_id: None,
        }
    }

    /// Broadcast message to all agents
    pub async fn broadcast(&self, content: &str) -> Vec<String> {
        let agents = self.agent_registry.list().await;
        let mut results = Vec::new();
        
        for agent in agents {
            results.push(agent.id);
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mentions() {
        let agent_registry = Arc::new(AgentRegistry::new());
        let context_manager = Arc::new(ContextManager::new());
        let router = Router::new(agent_registry, context_manager);

        let mentions = router.parse_mentions("@goclaw 帮我写个函数");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0], "goclaw");
    }

    #[test]
    fn test_clean_content() {
        let agent_registry = Arc::new(AgentRegistry::new());
        let context_manager = Arc::new(ContextManager::new());
        let router = Router::new(agent_registry, context_manager);

        let cleaned = router.clean_content("@goclaw 帮我写个排序算法");
        assert_eq!(cleaned, "帮我写个排序算法");
    }
}
