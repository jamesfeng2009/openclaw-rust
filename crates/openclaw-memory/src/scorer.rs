//! 重要性评分器

use openclaw_core::{Content, Message};
use regex::Regex;

/// 重要性评分器
pub struct ImportanceScorer {
    /// 实体正则表达式 (人名、地点、时间等)
    entity_patterns: Vec<Regex>,
    /// 关键词列表
    important_keywords: Vec<String>,
}

impl ImportanceScorer {
    pub fn new() -> Self {
        let entity_patterns = vec![
            // 邮箱
            Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
            // 电话号码
            Regex::new(r"\+?\d{1,3}[-.\s]?\d{3,4}[-.\s]?\d{4}").unwrap(),
            // URL
            Regex::new(r"https?://[^\s]+").unwrap(),
            // 日期
            Regex::new(r"\d{4}[-/年]\d{1,2}[-/月]\d{1,2}[日]?").unwrap(),
            // 时间
            Regex::new(r"\d{1,2}:\d{2}").unwrap(),
        ];

        let important_keywords: Vec<String> = vec![
            "重要",
            "关键",
            "必须",
            "确认",
            "决定",
            "任务",
            "important",
            "critical",
            "must",
            "confirm",
            "decision",
            "task",
            "密码",
            "账号",
            "账户",
            "password",
            "account",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        Self {
            entity_patterns,
            important_keywords,
        }
    }

    /// 计算消息的重要性分数 (0.0 - 1.0)
    pub fn score(&self, message: &Message) -> f32 {
        let mut score = 0.0;

        // 基础分数
        score += match message.role {
            openclaw_core::Role::System => 0.3,
            openclaw_core::Role::User => 0.2,
            openclaw_core::Role::Assistant => 0.15,
            openclaw_core::Role::Tool => 0.1,
        };

        // 内容分析
        if let Some(text) = message.text_content() {
            // 实体数量加分
            let entity_count = self.count_entities(text);
            score += (entity_count as f32 * 0.1).min(0.3);

            // 关键词匹配加分
            let keyword_count = self.count_keywords(text);
            score += (keyword_count as f32 * 0.05).min(0.2);

            // 长度因子 (中等长度的消息通常更重要)
            let len = text.len();
            if len > 50 && len < 500 {
                score += 0.1;
            }

            // 问句加分 (通常包含用户意图)
            if text.contains('?') || text.contains('？') {
                score += 0.1;
            }
        }

        // 工具调用加分
        for content in &message.content {
            if matches!(content, Content::ToolCall { .. }) {
                score += 0.15;
            }
        }

        // 确保分数在 0.0 - 1.0 范围内
        score.clamp(0.0, 1.0)
    }

    fn count_entities(&self, text: &str) -> usize {
        self.entity_patterns
            .iter()
            .map(|r: &Regex| r.find_iter(text).count())
            .sum()
    }

    fn count_keywords(&self, text: &str) -> usize {
        let lower = text.to_lowercase();
        self.important_keywords
            .iter()
            .filter(|kw| lower.contains(&kw.to_lowercase()))
            .count()
    }
}

impl Default for ImportanceScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scorer() {
        let scorer = ImportanceScorer::new();

        // 普通消息
        let msg1 = Message::user("你好");
        let score1 = scorer.score(&msg1);
        assert!(score1 > 0.0 && score1 < 1.0);

        // 包含实体的消息
        let msg2 = Message::user("我的邮箱是 test@example.com，电话是 123-4567-8901");
        let score2 = scorer.score(&msg2);
        assert!(score2 > score1);

        // 包含关键词的消息
        let msg3 = Message::user("这是一个重要的任务，请确认");
        let score3 = scorer.score(&msg3);
        assert!(score3 > score1);
    }
}
