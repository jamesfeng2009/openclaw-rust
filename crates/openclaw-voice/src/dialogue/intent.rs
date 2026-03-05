//! 意图识别模块

use std::collections::HashMap;
use async_trait::async_trait;
use openclaw_core::Result;

use crate::dialogue::{DialogueContext, Intent, SlotValue, Entity};

#[derive(Debug, Clone)]
pub struct IntentRecognitionResult {
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub raw_response: String,
}

#[async_trait]
pub trait IntentRecognizer: Send + Sync {
    async fn recognize(&self, text: &str, context: &DialogueContext) -> Result<IntentRecognitionResult>;
}

pub struct KeywordIntentRecognizer {
    intent_map: HashMap<String, String>,
}

impl KeywordIntentRecognizer {
    pub fn new() -> Self {
        let mut intent_map = HashMap::new();
        
        intent_map.insert("hello".to_string(), "greeting".to_string());
        intent_map.insert("hi".to_string(), "greeting".to_string());
        intent_map.insert("hey".to_string(), "greeting".to_string());
        
        intent_map.insert("weather".to_string(), "weather_query".to_string());
        intent_map.insert("temperature".to_string(), "weather_query".to_string());
        
        intent_map.insert("time".to_string(), "time_query".to_string());
        intent_map.insert("date".to_string(), "date_query".to_string());
        
        intent_map.insert("help".to_string(), "help_request".to_string());
        intent_map.insert("?".to_string(), "help_request".to_string());
        
        intent_map.insert("bye".to_string(), "farewell".to_string());
        intent_map.insert("goodbye".to_string(), "farewell".to_string());
        
        Self { intent_map }
    }

    pub fn register_intent(mut self, keyword: &str, intent: &str) -> Self {
        self.intent_map.insert(keyword.to_lowercase(), intent.to_string());
        self
    }
}

impl Default for KeywordIntentRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl IntentRecognizer for KeywordIntentRecognizer {
    async fn recognize(&self, text: &str, _context: &DialogueContext) -> Result<IntentRecognitionResult> {
        let text_lower = text.to_lowercase();
        
        let mut best_match: Option<(&String, &String)> = None;
        
        for (keyword, intent) in &self.intent_map {
            if text_lower.contains(keyword) {
                if best_match.is_none() || keyword.len() > best_match.unwrap().0.len() {
                    best_match = Some((keyword, intent));
                }
            }
        }

        let (intent_name, confidence) = match best_match {
            Some((_, intent)) => (intent.clone(), 0.8),
            None => ("unknown".to_string(), 0.0),
        };

        Ok(IntentRecognitionResult {
            intent: Intent::new(intent_name, confidence),
            entities: Vec::new(),
            raw_response: text.to_string(),
        })
    }
}

pub struct RuleBasedIntentRecognizer {
    rules: Vec<IntentRule>,
}

#[derive(Debug, Clone)]
pub struct IntentRule {
    pub keywords: Vec<String>,
    pub intent_name: String,
}

impl RuleBasedIntentRecognizer {
    pub fn new() -> Self {
        Self {
            rules: vec![],
        }
    }

    pub fn add_rule(mut self, rule: IntentRule) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn with_default_rules() -> Self {
        Self::new()
            .add_rule(IntentRule {
                keywords: vec!["weather".to_string(), "temperature".to_string()],
                intent_name: "weather_query".to_string(),
            })
            .add_rule(IntentRule {
                keywords: vec!["time".to_string(), "what time".to_string()],
                intent_name: "time_query".to_string(),
            })
            .add_rule(IntentRule {
                keywords: vec!["help".to_string(), "?".to_string()],
                intent_name: "help_request".to_string(),
            })
    }
}

impl Default for RuleBasedIntentRecognizer {
    fn default() -> Self {
        Self::with_default_rules()
    }
}

#[async_trait]
impl IntentRecognizer for RuleBasedIntentRecognizer {
    async fn recognize(&self, text: &str, _context: &DialogueContext) -> Result<IntentRecognitionResult> {
        let text_lower = text.to_lowercase();
        
        for rule in &self.rules {
            for keyword in &rule.keywords {
                if text_lower.contains(&keyword.to_lowercase()) {
                    return Ok(IntentRecognitionResult {
                        intent: Intent::new(rule.intent_name.clone(), 0.9),
                        entities: Vec::new(),
                        raw_response: text.to_string(),
                    });
                }
            }
        }

        Ok(IntentRecognitionResult {
            intent: Intent::new("unknown".to_string(), 0.0),
            entities: Vec::new(),
            raw_response: text.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_intent_recognizer_new() {
        let recognizer = KeywordIntentRecognizer::new();
        assert!(!recognizer.intent_map.is_empty());
    }

    #[test]
    fn test_keyword_intent_recognizer_register() {
        let recognizer = KeywordIntentRecognizer::new()
            .register_intent("custom", "custom_intent");
        
        assert!(recognizer.intent_map.contains_key("custom"));
    }

    #[tokio::test]
    async fn test_recognize_greeting() {
        let recognizer = KeywordIntentRecognizer::new();
        let context = DialogueContext::new("test".to_string());
        
        let result = recognizer.recognize("hello there", &context).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.intent.name, "greeting");
    }

    #[tokio::test]
    async fn test_recognize_unknown() {
        let recognizer = KeywordIntentRecognizer::new();
        let context = DialogueContext::new("test".to_string());
        
        let result = recognizer.recognize("xyz123 random", &context).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.intent.name, "unknown");
    }

    #[tokio::test]
    async fn test_rule_based_recognizer() {
        let recognizer = RuleBasedIntentRecognizer::with_default_rules();
        let context = DialogueContext::new("test".to_string());
        
        let result = recognizer.recognize("what's the weather today", &context).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.intent.name, "weather_query");
    }
}
