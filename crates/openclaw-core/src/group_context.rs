//! 群聊上下文管理
//!
//! 提供群聊场景下的上下文注入功能：
//! - 每轮对话注入群聊上下文
//! - 防止模型丢失群组意识
//! - 支持上下文压缩和摘要

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{Message, Result};

/// 群聊配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupChatConfig {
    /// 是否启用群聊上下文注入
    pub enabled: bool,
    /// 上下文注入模式
    pub injection_mode: ContextInjectionMode,
    /// 最大上下文消息数
    pub max_context_messages: usize,
    /// 是否包含发送者信息
    pub include_sender_info: bool,
    /// 是否生成群聊摘要
    pub generate_summary: bool,
    /// 摘要更新间隔 (消息数)
    pub summary_update_interval: usize,
}

impl Default for GroupChatConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            injection_mode: ContextInjectionMode::Prepend,
            max_context_messages: 20,
            include_sender_info: true,
            generate_summary: true,
            summary_update_interval: 10,
        }
    }
}

/// 上下文注入模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionMode {
    /// 在消息前添加上下文
    Prepend,
    /// 在消息后添加上下文
    Append,
    /// 作为系统消息注入
    SystemMessage,
    /// 融合到用户消息中
    Inline,
}

/// 群聊成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub role: GroupMemberRole,
    pub joined_at: Option<DateTime<Utc>>,
    pub last_active: Option<DateTime<Utc>>,
    pub message_count: usize,
}

/// 群成员角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GroupMemberRole {
    Admin,
    Moderator,
    Member,
    Guest,
}

/// 群聊信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub member_count: usize,
    pub created_at: DateTime<Utc>,
    pub topic: Option<String>,
}

/// 群聊上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupContext {
    /// 群组信息
    pub group_info: GroupInfo,
    /// 成员列表
    pub members: HashMap<String, GroupMember>,
    /// 最近消息
    pub recent_messages: Vec<GroupMessage>,
    /// 群聊摘要
    pub summary: Option<String>,
    /// 摘要更新时间
    pub summary_updated_at: Option<DateTime<Utc>>,
    /// 上下文消息计数
    pub message_count_since_summary: usize,
    /// 活跃话题
    pub active_topics: Vec<String>,
}

impl GroupContext {
    /// 创建新的群聊上下文
    pub fn new(group_info: GroupInfo) -> Self {
        Self {
            group_info,
            members: HashMap::new(),
            recent_messages: Vec::new(),
            summary: None,
            summary_updated_at: None,
            message_count_since_summary: 0,
            active_topics: Vec::new(),
        }
    }

    /// 添加成员
    pub fn add_member(&mut self, member: GroupMember) {
        self.members.insert(member.id.clone(), member);
        self.group_info.member_count = self.members.len();
    }

    /// 移除成员
    pub fn remove_member(&mut self, member_id: &str) {
        self.members.remove(member_id);
        self.group_info.member_count = self.members.len();
    }

    /// 记录消息
    pub fn record_message(&mut self, message: GroupMessage) {
        // 更新成员活跃状态
        if let Some(member) = self.members.get_mut(&message.sender_id) {
            member.last_active = Some(message.timestamp);
            member.message_count += 1;
        }

        // 添加到最近消息
        self.recent_messages.push(message);
        self.message_count_since_summary += 1;

        // 保持消息数量限制
        if self.recent_messages.len() > 50 {
            self.recent_messages.remove(0);
        }
    }

    /// 更新摘要
    pub fn update_summary(&mut self, summary: String) {
        self.summary = Some(summary);
        self.summary_updated_at = Some(Utc::now());
        self.message_count_since_summary = 0;
    }

    /// 需要更新摘要
    pub fn needs_summary_update(&self, interval: usize) -> bool {
        self.message_count_since_summary >= interval
    }

    /// 获取成员显示名
    pub fn get_member_display_name(&self, member_id: &str) -> String {
        self.members
            .get(member_id)
            .map(|m| m.display_name.clone().unwrap_or_else(|| m.name.clone()))
            .unwrap_or_else(|| "未知用户".to_string())
    }
}

/// 群聊消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMessage {
    pub id: String,
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub reply_to: Option<String>,
    pub mentions: Vec<String>,
}

/// 群聊上下文注入器
pub struct GroupContextInjector {
    config: GroupChatConfig,
}

impl GroupContextInjector {
    pub fn new(config: GroupChatConfig) -> Self {
        Self { config }
    }

    /// 注入群聊上下文到消息
    pub fn inject_context(&self, message: Message, context: &GroupContext) -> Result<Message> {
        if !self.config.enabled {
            return Ok(message);
        }

        let context_text = self.build_context_text(context);

        let injected_content = match self.config.injection_mode {
            ContextInjectionMode::Prepend => {
                format!(
                    "{}\n\n{}",
                    context_text,
                    message.text_content().unwrap_or_default()
                )
            }
            ContextInjectionMode::Append => {
                format!(
                    "{}\n\n{}",
                    message.text_content().unwrap_or_default(),
                    context_text
                )
            }
            ContextInjectionMode::SystemMessage => {
                // 返回原消息，系统消息需要单独处理
                return Ok(message);
            }
            ContextInjectionMode::Inline => {
                format!(
                    "[群聊上下文] {}\n{}",
                    context_text,
                    message.text_content().unwrap_or_default()
                )
            }
        };

        Ok(Message::user(injected_content))
    }

    /// 构建上下文文本
    fn build_context_text(&self, context: &GroupContext) -> String {
        let mut parts = Vec::new();

        // 群组信息
        parts.push(format!(
            "【群组】{} ({} 名成员)",
            context.group_info.name, context.group_info.member_count
        ));

        // 群聊摘要
        if let Some(ref summary) = context.summary {
            parts.push(format!("【最近话题】{}", summary));
        }

        // 最近消息上下文
        if !context.recent_messages.is_empty() {
            let recent: Vec<String> = context
                .recent_messages
                .iter()
                .rev()
                .take(self.config.max_context_messages)
                .rev()
                .map(|msg| {
                    if self.config.include_sender_info {
                        format!("{}: {}", msg.sender_name, msg.content)
                    } else {
                        msg.content.clone()
                    }
                })
                .collect();

            parts.push(format!("【最近消息】\n{}", recent.join("\n")));
        }

        parts.join("\n\n")
    }

    /// 生成系统消息
    pub fn generate_system_message(&self, context: &GroupContext) -> Option<Message> {
        if !self.config.enabled || self.config.injection_mode != ContextInjectionMode::SystemMessage
        {
            return None;
        }

        let context_text = self.build_context_text(context);
        Some(Message::system(format!(
            "你正在群聊「{}」中回复消息。以下是群聊上下文：\n\n{}",
            context.group_info.name, context_text
        )))
    }

    /// 生成群聊摘要
    pub fn generate_summary(&self, context: &GroupContext) -> String {
        let mut summary_parts = Vec::new();

        // 活跃成员
        let active_members: Vec<_> = context
            .members
            .values()
            .filter(|m| m.message_count > 0)
            .take(5)
            .collect();

        if !active_members.is_empty() {
            let names: Vec<_> = active_members.iter().map(|m| m.name.as_str()).collect();
            summary_parts.push(format!("活跃成员: {}", names.join(", ")));
        }

        // 话题关键词
        if !context.active_topics.is_empty() {
            summary_parts.push(format!("讨论话题: {}", context.active_topics.join(", ")));
        }

        // 消息统计
        summary_parts.push(format!(
            "最近 {} 条消息",
            context.recent_messages.len().min(50)
        ));

        summary_parts.join(" | ")
    }
}

impl Default for GroupContextInjector {
    fn default() -> Self {
        Self::new(GroupChatConfig::default())
    }
}

/// 群聊上下文管理器
pub struct GroupContextManager {
    contexts: HashMap<String, GroupContext>,
    injector: GroupContextInjector,
    config: GroupChatConfig,
}

impl GroupContextManager {
    pub fn new(config: GroupChatConfig) -> Self {
        let injector = GroupContextInjector::new(config.clone());
        Self {
            contexts: HashMap::new(),
            injector,
            config,
        }
    }

    /// 获取或创建群聊上下文
    pub fn get_or_create(
        &mut self,
        group_id: &str,
        group_info: Option<GroupInfo>,
    ) -> &mut GroupContext {
        self.contexts
            .entry(group_id.to_string())
            .or_insert_with(|| {
                let info = group_info.unwrap_or_else(|| GroupInfo {
                    id: group_id.to_string(),
                    name: format!("群组 {}", group_id),
                    description: None,
                    member_count: 0,
                    created_at: Utc::now(),
                    topic: None,
                });
                GroupContext::new(info)
            })
    }

    /// 记录消息到群聊上下文
    pub fn record_message(
        &mut self,
        group_id: &str,
        sender_id: &str,
        sender_name: &str,
        content: &str,
    ) {
        if let Some(context) = self.contexts.get_mut(group_id) {
            let message = GroupMessage {
                id: uuid::Uuid::new_v4().to_string(),
                sender_id: sender_id.to_string(),
                sender_name: sender_name.to_string(),
                content: content.to_string(),
                timestamp: Utc::now(),
                reply_to: None,
                mentions: Vec::new(),
            };
            context.record_message(message);
        }
    }

    /// 注入上下文
    pub fn inject(&self, group_id: &str, message: Message) -> Result<Message> {
        if let Some(context) = self.contexts.get(group_id) {
            self.injector.inject_context(message, context)
        } else {
            Ok(message)
        }
    }

    /// 获取系统消息
    pub fn get_system_message(&self, group_id: &str) -> Option<Message> {
        self.contexts
            .get(group_id)
            .and_then(|ctx| self.injector.generate_system_message(ctx))
    }

    /// 更新群聊摘要
    pub fn update_summary(&mut self, group_id: &str) {
        if let Some(context) = self.contexts.get(group_id)
            && context.needs_summary_update(self.config.summary_update_interval)
        {
            let summary = self.injector.generate_summary(context);
            if let Some(ctx) = self.contexts.get_mut(group_id) {
                ctx.update_summary(summary);
            }
        }
    }

    /// 清理过期上下文
    pub fn cleanup_stale(&mut self, max_age_hours: i64) {
        let now = Utc::now();
        self.contexts.retain(|_, ctx| {
            ctx.recent_messages
                .last()
                .map(|msg| {
                    let age = now - msg.timestamp;
                    age.num_hours() < max_age_hours
                })
                .unwrap_or(false)
        });
    }

    /// 获取群聊统计
    pub fn get_stats(&self) -> GroupContextStats {
        GroupContextStats {
            total_groups: self.contexts.len(),
            total_messages: self
                .contexts
                .values()
                .map(|c| c.recent_messages.len())
                .sum(),
            total_members: self.contexts.values().map(|c| c.members.len()).sum(),
        }
    }
}

impl Default for GroupContextManager {
    fn default() -> Self {
        Self::new(GroupChatConfig::default())
    }
}

/// 群聊上下文统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupContextStats {
    pub total_groups: usize,
    pub total_messages: usize,
    pub total_members: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_context_creation() {
        let group_info = GroupInfo {
            id: "test-group".to_string(),
            name: "测试群".to_string(),
            description: None,
            member_count: 0,
            created_at: Utc::now(),
            topic: None,
        };

        let context = GroupContext::new(group_info);
        assert_eq!(context.group_info.name, "测试群");
        assert!(context.recent_messages.is_empty());
    }

    #[test]
    fn test_member_management() {
        let group_info = GroupInfo {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: None,
            member_count: 0,
            created_at: Utc::now(),
            topic: None,
        };

        let mut context = GroupContext::new(group_info);

        let member = GroupMember {
            id: "user-1".to_string(),
            name: "测试用户".to_string(),
            display_name: Some("小测试".to_string()),
            role: GroupMemberRole::Member,
            joined_at: None,
            last_active: None,
            message_count: 0,
        };

        context.add_member(member);
        assert_eq!(context.members.len(), 1);
        assert_eq!(context.group_info.member_count, 1);
    }

    #[test]
    fn test_context_injection() {
        let config = GroupChatConfig::default();
        let injector = GroupContextInjector::new(config);

        let group_info = GroupInfo {
            id: "test".to_string(),
            name: "测试群".to_string(),
            description: None,
            member_count: 3,
            created_at: Utc::now(),
            topic: None,
        };

        let context = GroupContext::new(group_info);
        let message = Message::user("你好");

        let injected = injector.inject_context(message, &context).unwrap();
        assert!(injected.text_content().unwrap().contains("测试群"));
    }
}
