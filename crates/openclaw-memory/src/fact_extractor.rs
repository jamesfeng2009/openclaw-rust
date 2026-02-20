//! 原子事实提取器
//!
//! 从对话中提取离散的事实条目

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use openclaw_ai::AIProvider;
use openclaw_core::OpenClawError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicFact {
    pub id: String,
    pub content: String,
    pub category: FactCategory,
    pub source_message_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confidence: f32,
    pub is_negative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum FactCategory {
    UserPreference,
    UserBackground,
    UserGoal,
    PersonalInfo,
    WorkInfo,
    ProjectInfo,
    Decision,
    Note,
    Summary,
    Error,
    Action,
    Feedback,
    Other,
}

impl FactCategory {
    pub fn from_text(text: &str) -> Self {
        let lower = text.to_lowercase();
        if lower.contains("喜欢") || lower.contains("偏好") || lower.contains("不爱") || lower.contains("爱") {
            FactCategory::UserPreference
        } else if lower.contains("工作") || lower.contains("公司") || lower.contains("职业") || lower.contains("在") {
            FactCategory::WorkInfo
        } else if lower.contains("目标") || lower.contains("想要") || lower.contains("计划") || lower.contains("想") || lower.contains("学习") {
            FactCategory::UserGoal
        } else if lower.contains("背景") || lower.contains("经历") {
            FactCategory::UserBackground
        } else if lower.contains("决定") || lower.contains("选择") {
            FactCategory::Decision
        } else {
            FactCategory::Other
        }
    }
}

impl AtomicFact {
    pub fn new(content: String, category: FactCategory) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            category,
            source_message_id: None,
            created_at: Utc::now(),
            confidence: 1.0,
            is_negative: false,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_source(mut self, message_id: String) -> Self {
        self.source_message_id = Some(message_id);
        self
    }

    pub fn is_contradicting(&self, other: &AtomicFact) -> bool {
        if self.category != other.category {
            return false;
        }

        let self_lower = self.content.to_lowercase();
        let other_lower = other.content.to_lowercase();

        let negations = [
            ("喜欢", "不喜欢"),
            ("爱", "不爱"),
            ("会", "不会"),
            ("能", "不能"),
            ("有", "没有"),
            ("是", "不是"),
            ("在", "不在"),
        ];

        for (pos, neg) in &negations {
            if self_lower.contains(pos) && other_lower.contains(neg) {
                return true;
            }
            if self_lower.contains(neg) && other_lower.contains(pos) {
                return true;
            }
        }

        false
    }
}

#[async_trait]
pub trait FactExtractor: Send + Sync {
    async fn extract_facts(&self, conversation: &str) -> Result<Vec<AtomicFact>, OpenClawError>;
}

pub struct LLMFactExtractor {
    provider: Arc<dyn AIProvider>,
    model: String,
}

impl LLMFactExtractor {
    pub fn new(provider: Arc<dyn AIProvider>, model: impl Into<String>) -> Self {
        Self {
            provider,
            model: model.into(),
        }
    }
}

#[async_trait]
impl FactExtractor for LLMFactExtractor {
    async fn extract_facts(&self, conversation: &str) -> Result<Vec<AtomicFact>, OpenClawError> {
        use openclaw_ai::ChatRequest;
        use openclaw_core::Message as AiMessage;

        let prompt = format!(
            r#"从以下对话中提取离散的事实条目。每个事实应该是独立的、可验证的信息。

要求：
1. 只提取关于用户的事实，不要提取关于 AI 或系统的事实
2. 每个事实应该是原子化的（不可再分）
3. 包含用户的偏好、背景、目标、决定等信息
4. 用第一人称（用户视角）表述
5. 如果信息是负面的（如"不喜欢"），标记为负面

返回 JSON 数组格式：
[{{"content": "用户喜欢Python编程", "category": "user_preference", "confidence": 0.9}}]

对话内容：
{}"#,
            conversation
        );

        let request = ChatRequest::new(
            self.model.clone(),
            vec![AiMessage::user(prompt)]
        ).with_temperature(0.3).with_max_tokens(4000);

        let response = self.provider.chat(request).await?;
        let content = response.message.text_content().unwrap_or_default().to_string();

        let facts: Vec<AtomicFact> = serde_json::from_str(&content)
            .or_else(|_| {
                serde_json::from_str::<Vec<serde_json::Value>>(&content)
                    .map(|vals| {
                        vals.into_iter()
                            .filter_map(|v| {
                                let content = v.get("content")?.as_str()?.to_string();
                                let category_str = v.get("category").and_then(|c| c.as_str()).unwrap_or("other");
                                let category = match category_str {
                                    "user_preference" => FactCategory::UserPreference,
                                    "user_background" => FactCategory::UserBackground,
                                    "user_goal" => FactCategory::UserGoal,
                                    "personal_info" => FactCategory::PersonalInfo,
                                    "work_info" => FactCategory::WorkInfo,
                                    "project_info" => FactCategory::ProjectInfo,
                                    "decision" => FactCategory::Decision,
                                    "note" => FactCategory::Note,
                                    _ => FactCategory::Other,
                                };
                                let confidence = v.get("confidence").and_then(|c| c.as_f64()).unwrap_or(1.0) as f32;
                                Some(AtomicFact::new(content, category).with_confidence(confidence))
                            })
                            .collect()
                    })
            })
            .map_err(|e| OpenClawError::AIProvider(format!("解析事实提取结果失败: {}", e)))?;

        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_category_from_text() {
        assert_eq!(FactCategory::from_text("用户说他喜欢Python"), FactCategory::UserPreference);
        assert_eq!(FactCategory::from_text("我在Google工作"), FactCategory::WorkInfo);
        assert_eq!(FactCategory::from_text("我想学习Rust"), FactCategory::UserGoal);
    }

    #[test]
    fn test_atomic_fact_creation() {
        let fact = AtomicFact::new("用户喜欢咖啡".to_string(), FactCategory::UserPreference);
        assert_eq!(fact.content, "用户喜欢咖啡");
        assert_eq!(fact.category, FactCategory::UserPreference);
        assert!(!fact.is_negative);
    }

    #[test]
    fn test_contradiction_detection() {
        let fact1 = AtomicFact::new("用户喜欢Python".to_string(), FactCategory::UserPreference);
        let fact2 = AtomicFact::new("用户不喜欢Python".to_string(), FactCategory::UserPreference);
        
        assert!(fact1.is_contradicting(&fact2));
    }

    #[test]
    fn test_non_contradiction() {
        let fact1 = AtomicFact::new("用户喜欢Python".to_string(), FactCategory::UserPreference);
        let fact2 = AtomicFact::new("用户喜欢咖啡".to_string(), FactCategory::UserPreference);
        
        assert!(!fact1.is_contradicting(&fact2));
    }

    #[test]
    fn test_cross_category_no_contradiction() {
        let fact1 = AtomicFact::new("用户喜欢Python".to_string(), FactCategory::UserPreference);
        let fact2 = AtomicFact::new("用户在Google工作".to_string(), FactCategory::WorkInfo);
        
        assert!(!fact1.is_contradicting(&fact2));
    }

    #[test]
    fn test_fact_with_confidence() {
        let fact = AtomicFact::new("用户喜欢咖啡".to_string(), FactCategory::UserPreference)
            .with_confidence(0.8);
        assert!((fact.confidence - 0.8).abs() < 0.001);
    }
}
