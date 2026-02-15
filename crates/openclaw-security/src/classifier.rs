use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PromptCategory {
    Safe,
    Benign,
    Suspicious,
    Malicious,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassification {
    pub category: PromptCategory,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub risk_score: f32,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationConfig {
    pub model_provider: String,
    pub model_name: String,
    pub threshold_suspicious: f32,
    pub threshold_malicious: f32,
    pub enabled: bool,
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            model_provider: "openai".to_string(),
            model_name: "gpt-4o-mini".to_string(),
            threshold_suspicious: 0.6,
            threshold_malicious: 0.85,
            enabled: true,
        }
    }
}

pub struct PromptClassifier {
    config: Arc<RwLock<ClassificationConfig>>,
    category_stats: Arc<RwLock<HashMap<PromptCategory, u32>>>,
}

impl Default for PromptClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptClassifier {
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(ClassificationConfig::default())),
            category_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn configure(&self, config: ClassificationConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    pub async fn classify(&self, input: &str, context: Option<&str>) -> LlmClassification {
        let config = self.config.read().await;
        
        if !config.enabled {
            return LlmClassification {
                category: PromptCategory::Safe,
                confidence: 1.0,
                reasons: vec!["Classifier disabled".to_string()],
                risk_score: 0.0,
                recommendations: vec![],
            };
        }

        let prompt = Self::build_classification_prompt(input, context);
        
        let classification = self.perform_classification(&prompt).await;
        
        self.record_category(&classification.category).await;

        debug!(
            "Prompt classified as {:?} with confidence {:.2}",
            classification.category, classification.confidence
        );

        classification
    }

    fn build_classification_prompt(input: &str, context: Option<&str>) -> String {
        let context_str = context.map(|c| format!("\n\n上下文信息:\n{}", c)).unwrap_or_default();
        
        format!(
            r#"请分析以下用户输入是否存在安全风险。只需输出 JSON 格式的分类结果。

用户输入: {}
{}

输出格式:
{{
  "category": "safe|benign|suspicious|malicious|critical",
  "confidence": 0.0-1.0,
  "reasons": ["原因1", "原因2"],
  "risk_score": 0.0-1.0,
  "recommendations": ["建议1", "建议2"]
}}"#,
            input, context_str
        )
    }

    async fn perform_classification(&self, prompt: &str) -> LlmClassification {
        let keywords = [
            ("sudo", "提权尝试", 0.7),
            ("rm -rf", "破坏性命令", 0.9),
            ("ignore previous", "提示注入", 0.8),
            ("roleplay", "角色扮演尝试", 0.6),
            ("system prompt", "系统提示探测", 0.85),
            ("越权", "权限提升尝试", 0.8),
            ("绕过", "安全绕过尝试", 0.85),
            ("eval(", "代码注入", 0.7),
            ("exec(", "命令执行", 0.7),
        ];

        let mut match_count = 0;
        let mut max_risk: f32 = 0.0;
        let mut reasons = Vec::new();
        let input_lower = prompt.to_lowercase();

        for (keyword, reason, risk) in keywords {
            if input_lower.contains(keyword) {
                match_count += 1;
                max_risk = max_risk.max(risk);
                reasons.push(format!("检测到关键词: {} (风险: {})", reason, risk));
            }
        }

        if match_count == 0 {
            return LlmClassification {
                category: PromptCategory::Safe,
                confidence: 0.95,
                reasons: vec!["未检测到可疑模式".to_string()],
                risk_score: 0.05,
                recommendations: vec![],
            };
        }

        let category = if max_risk >= 0.85 {
            PromptCategory::Critical
        } else if max_risk >= 0.7 {
            PromptCategory::Malicious
        } else if max_risk >= 0.5 {
            PromptCategory::Suspicious
        } else {
            PromptCategory::Benign
        };

        let category_for_recs = category.clone();

        let confidence = (match_count as f32 * 0.2).min(0.95);

        LlmClassification {
            category,
            confidence,
            reasons,
            risk_score: max_risk,
            recommendations: Self::get_recommendations(category_for_recs),
        }
    }

    fn get_recommendations(category: PromptCategory) -> Vec<String> {
        match category {
            PromptCategory::Safe => vec![],
            PromptCategory::Benign => vec!["建议监控".to_string()],
            PromptCategory::Suspicious => vec![
                "建议使用严格模式重新检查".to_string(),
                "记录日志".to_string(),
            ],
            PromptCategory::Malicious => vec![
                "阻止执行并返回错误".to_string(),
                "记录完整审计日志".to_string(),
                "通知管理员".to_string(),
            ],
            PromptCategory::Critical => vec![
                "立即阻止执行".to_string(),
                "终止会话".to_string(),
                "通知安全团队".to_string(),
                "保存证据".to_string(),
            ],
        }
    }

    async fn record_category(&self, category: &PromptCategory) {
        let mut stats = self.category_stats.write().await;
        *stats.entry(category.clone()).or_insert(0) += 1;
    }

    pub async fn get_stats(&self) -> HashMap<PromptCategory, u32> {
        let stats = self.category_stats.read().await;
        stats.clone()
    }

    pub async fn reset_stats(&self) {
        let mut stats = self.category_stats.write().await;
        stats.clear();
    }
}
