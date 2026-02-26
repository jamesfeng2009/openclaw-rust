//! Memory Compression - 上下文压缩
//!
//! 提供上下文压缩和摘要功能：
//! - ContextCompactor: 上下文压缩器
//! - MemoryCleanupPolicy: 内存清理策略

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub max_tokens: usize,
    pub min_tokens_to_compress: usize,
    pub summary_ratio: f32,
    pub preserve_recent: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8000,
            min_tokens_to_compress: 2000,
            summary_ratio: 0.3,
            preserve_recent: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: usize,
}

impl ContextMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let tokens = estimate_tokens(&content);
        Self {
            role: role.into(),
            content,
            timestamp: Utc::now(),
            tokens,
        }
    }

    pub fn from_message(role: impl Into<String>, content: impl Into<String>, timestamp: DateTime<Utc>) -> Self {
        let content = content.into();
        let tokens = estimate_tokens(&content);
        Self {
            role: role.into(),
            content,
            timestamp,
            tokens,
        }
    }
}

fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

pub struct ContextCompactor {
    config: CompressionConfig,
}

impl ContextCompactor {
    pub fn new(config: CompressionConfig) -> Self {
        Self { config }
    }

    pub fn with_default() -> Self {
        Self::new(CompressionConfig::default())
    }

    pub fn should_compress(&self, messages: &[ContextMessage]) -> bool {
        let total_tokens: usize = messages.iter().map(|m| m.tokens).sum();
        total_tokens >= self.config.min_tokens_to_compress
    }

    pub fn compress(&self, messages: &[ContextMessage]) -> Vec<ContextMessage> {
        if messages.is_empty() {
            return Vec::new();
        }

        let total_tokens: usize = messages.iter().map(|m| m.tokens).sum();
        if total_tokens < self.config.min_tokens_to_compress {
            return messages.to_vec();
        }

        let preserve_count = self.config.preserve_recent.min(messages.len());
        let (recent, to_compress) = messages.split_at(messages.len() - preserve_count);

        let compressed = self.summarize(to_compress);

        let mut result = compressed;
        result.extend(recent.iter().cloned());

        result
    }

    pub fn summarize(&self, messages: &[ContextMessage]) -> Vec<ContextMessage> {
        if messages.is_empty() {
            return Vec::new();
        }

        let total_tokens: usize = messages.iter().map(|m| m.tokens).sum();
        let target_tokens = (total_tokens as f32 * self.config.summary_ratio) as usize;

        let mut summary_parts = Vec::new();
        let mut current_tokens = 0;

        for msg in messages.iter().rev() {
            if current_tokens >= target_tokens {
                break;
            }
            summary_parts.push(format!("[{}]: {}", msg.role, truncate(&msg.content, 100)));
            current_tokens += msg.tokens;
        }

        summary_parts.reverse();

        let summary_content = if summary_parts.len() > 3 {
            format!(
                "... ({} messages summarized) ...",
                messages.len() - summary_parts.len()
            )
        } else {
            summary_parts.join("\n")
        };

        vec![ContextMessage::from_message(
            "system",
            format!("Previous context summary:\n{}", summary_content),
            Utc::now(),
        )]
    }

    pub fn compress_deep_path(&self, messages: &[ContextMessage], max_depth: usize) -> Vec<ContextMessage> {
        if messages.len() <= max_depth {
            return messages.to_vec();
        }

        let keep_recent = max_depth / 2;
        let keep_first = max_depth - keep_recent;

        let (first, middle) = messages.split_at(keep_first);
        let (_recent, middle) = middle.split_at(middle.len() - keep_recent);

        let compressed = self.summarize(middle);

        let mut result = first.to_vec();
        result.extend(compressed);
        result.extend(_recent.iter().cloned());

        result
    }
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPolicy {
    pub max_branches: usize,
    pub branch_ttl_seconds: i64,
    pub compress_deep_paths: bool,
    pub deep_path_threshold: usize,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            max_branches: 10,
            branch_ttl_seconds: 3600,
            compress_deep_paths: true,
            deep_path_threshold: 20,
        }
    }
}

pub struct MemoryCleanupPolicy {
    config: CleanupPolicy,
}

impl MemoryCleanupPolicy {
    pub fn new(config: CleanupPolicy) -> Self {
        Self { config }
    }

    pub fn with_default() -> Self {
        Self::new(CleanupPolicy::default())
    }

    pub fn cleanup_expired_branches(
        &self,
        nodes: &mut Vec<(String, DateTime<Utc>)>,
    ) -> Vec<String> {
        let now = Utc::now();
        let ttl = chrono::Duration::seconds(self.config.branch_ttl_seconds);

        let expired: Vec<String> = nodes
            .iter()
            .filter(|(_, timestamp)| now.signed_duration_since(*timestamp) > ttl)
            .map(|(id, _)| id.clone())
            .collect();

        nodes.retain(|(_, timestamp)| {
            !(now.signed_duration_since(*timestamp) > ttl)
        });

        expired
    }

    pub fn limit_branches(&self, nodes: &mut Vec<(String, DateTime<Utc>)>) -> Vec<String> {
        if nodes.len() <= self.config.max_branches {
            return Vec::new();
        }

        nodes.sort_by(|a, b| b.1.cmp(&a.1));

        let removed: Vec<String> = nodes
            .split_off(self.config.max_branches)
            .into_iter()
            .map(|(id, _)| id)
            .collect();

        removed
    }

    pub fn should_compress_path(&self, depth: usize) -> bool {
        self.config.compress_deep_paths && depth >= self.config.deep_path_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert!(estimate_tokens("hello") >= 1);
        assert!(estimate_tokens("hello world") >= 2);
    }

    #[tokio::test]
    async fn test_should_compress() {
        let compactor = ContextCompactor::with_default();
        
        let messages = vec![
            ContextMessage::new("user", "hello"),
            ContextMessage::new("assistant", "hi"),
        ];
        
        assert!(!compactor.should_compress(&messages));
    }

    #[tokio::test]
    async fn test_compress() {
        let compactor = ContextCompactor::with_default();
        
        let messages = vec![
            ContextMessage::new("user", "a".repeat(1000)),
            ContextMessage::new("assistant", "b".repeat(1000)),
            ContextMessage::new("user", "c".repeat(1000)),
            ContextMessage::new("assistant", "recent message"),
        ];
        
        let result = compactor.compress(&messages);
        assert!(result.len() <= messages.len());
    }

    #[tokio::test]
    async fn test_compress_deep_path() {
        let compactor = ContextCompactor::with_default();
        
        let messages: Vec<ContextMessage> = (0..25)
            .map(|i| ContextMessage::new("user", format!("message {}", i)))
            .collect();
        
        let result = compactor.compress_deep_path(&messages, 10);
        assert!(result.len() < messages.len());
    }

    #[tokio::test]
    async fn test_cleanup_expired_branches() {
        let policy = MemoryCleanupPolicy::with_default();
        
        let now = Utc::now();
        let old = now - chrono::Duration::seconds(4000);
        
        let mut nodes = vec![
            ("branch1".to_string(), now),
            ("branch2".to_string(), old),
        ];
        
        let expired = policy.cleanup_expired_branches(&mut nodes);
        
        assert_eq!(expired.len(), 1);
        assert_eq!(nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_limit_branches() {
        let policy = MemoryCleanupPolicy::new(CleanupPolicy {
            max_branches: 2,
            ..Default::default()
        });
        
        let now = Utc::now();
        let mut nodes = vec![
            ("branch1".to_string(), now),
            ("branch2".to_string(), now),
            ("branch3".to_string(), now),
        ];
        
        let removed = policy.limit_branches(&mut nodes);
        
        assert_eq!(nodes.len(), 2);
        assert_eq!(removed.len(), 1);
    }
}
