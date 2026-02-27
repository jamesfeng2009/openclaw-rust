//! 飞书通道实现
//!
//! 支持飞书机器人（Webhook 和 Bot API）

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// 飞书 @消息提及
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuMention {
    /// 提及类型: user, room, tenant
    pub mention_type: String,
    /// 提及的 ID
    pub key: String,
    /// 提及的名称
    pub name: String,
}

/// 飞书消息提及解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuMessageParseResult {
    /// 原始消息内容
    pub raw_content: String,
    /// 清理后的文本内容（移除 @机器人 部分）
    pub clean_content: String,
    /// 解析出的 @提及列表
    pub mentions: Vec<FeishuMention>,
    /// 群/聊天 ID
    pub chat_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
    /// 发送者 ID
    pub user_id: Option<String>,
    /// 发送者名称
    pub user_name: Option<String>,
}

/// 飞书群信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuGroupInfo {
    /// 群 ID
    pub chat_id: String,
    /// 群名称
    pub name: String,
    /// 群头像
    pub avatar: Option<String>,
    /// 群描述
    pub description: Option<String>,
    /// 群成员数
    pub member_count: Option<i32>,
    /// 群所有者 ID
    pub owner_id: Option<String>,
}

/// 飞书群成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuGroupMember {
    /// 用户 ID (open_id 或 user_id)
    pub member_id: String,
    /// 成员类型: user, bot
    pub member_type: String,
    /// 成员名称
    pub name: String,
    /// 是否是机器人
    pub is_bot: bool,
}

/// 飞书消息发送响应
#[derive(Debug, Deserialize)]
struct FeishuSendMessageResponse {
    code: i32,
    msg: String,
    data: Option<FeishuMessageData>,
}

#[derive(Debug, Deserialize)]
struct FeishuMessageData {
    message_id: String,
}

/// 飞书群列表响应
#[derive(Debug, Deserialize)]
struct FeishuListChatsResponse {
    code: i32,
    msg: String,
    data: Option<FeishuListChatsData>,
}

#[derive(Debug, Deserialize)]
struct FeishuListChatsData {
    items: Option<Vec<FeishuChatItem>>,
    page_token: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct FeishuChatItem {
    chat_id: String,
    name: String,
    avatar: Option<String>,
    description: Option<String>,
    owner_id: Option<String>,
    owner_id_type: Option<String>,
    external: Option<bool>,
    tenant_key: Option<String>,
}

/// 飞书群成员响应
#[derive(Debug, Deserialize)]
struct FeishuListMembersResponse {
    code: i32,
    msg: String,
    data: Option<FeishuListMembersData>,
}

#[derive(Debug, Deserialize)]
struct FeishuListMembersData {
    items: Option<Vec<FeishuMemberItem>>,
    page_token: Option<String>,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct FeishuMemberItem {
    member_id: String,
    member_id_type: String,
    name: String,
    avatar: Option<String>,
    type_: i32,
}

/// 飞书通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID
    pub app_id: String,
    /// App Secret
    pub app_secret: String,
    /// Webhook 地址（可选，用于简单场景）
    pub webhook: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 飞书机器人
pub struct FeishuChannel {
    config: FeishuConfig,
    client: Client,
    access_token: Option<String>,
}

impl FeishuChannel {
    /// 创建新的飞书通道
    pub fn new(config: FeishuConfig) -> Self {
        let client = Client::new();
        Self {
            config,
            client,
            access_token: None,
        }
    }

    /// 获取 tenant_access_token
    pub async fn get_access_token(&mut self) -> Result<String> {
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }

        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

        let body = json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret
        });

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuTokenResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        self.access_token = Some(result.tenant_access_token.clone());
        Ok(result.tenant_access_token)
    }

    /// 解析飞书 @消息
    ///
    /// 飞书 @消息格式:
    /// - `@{机器人名称}` 或 `@机器人名称`
    /// - 示例: `@OpenClaw 你好`, `@goclaw 帮我写个函数`
    pub fn parse_mentions(&self, content: &str) -> FeishuMessageParseResult {
        let mut mentions = Vec::new();
        let mut clean_content = content.to_string();

        let at_pattern = Regex::new(r"@([\w\u4e00-\u9fa5]+)").unwrap();

        for cap in at_pattern.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let mention_name = name.as_str();
                if !mention_name.is_empty() {
                    mentions.push(FeishuMention {
                        mention_type: "user".to_string(),
                        key: mention_name.to_string(),
                        name: mention_name.to_string(),
                    });

                    let mention_str = format!("@{}", mention_name);
                    // Replace all occurrences and trim once at the end
                    while clean_content.contains(&mention_str) {
                        clean_content = clean_content.replace(&mention_str, "");
                    }
                }
            }
        }

        clean_content = clean_content.trim().to_string();

        FeishuMessageParseResult {
            raw_content: content.to_string(),
            clean_content,
            mentions,
            chat_id: None,
            message_id: None,
            user_id: None,
            user_name: None,
        }
    }

    /// 解析飞书事件消息 (JSON 格式)
    ///
    /// 处理飞书回调事件中的消息内容
    pub fn parse_event_message(&self, event_json: &serde_json::Value) -> FeishuMessageParseResult {
        let mut result = FeishuMessageParseResult {
            raw_content: String::new(),
            clean_content: String::new(),
            mentions: Vec::new(),
            chat_id: None,
            message_id: None,
            user_id: None,
            user_name: None,
        };

        if let Some(message) = event_json.get("message") {
            if let Some(msg_type) = message.get("message_type").and_then(|v| v.as_str()) {
                if msg_type == "text" {
                    if let Some(body) = message.get("body") {
                        if let Some(content) = body.get("content").and_then(|v| v.as_str()) {
                            let parsed = self.parse_mentions(content);
                            result.raw_content = parsed.raw_content;
                            result.clean_content = parsed.clean_content;
                            result.mentions = parsed.mentions;
                        }
                    }
                }
            }

            if let Some(chat_id) = message.get("chat_id").and_then(|v| v.as_str()) {
                result.chat_id = Some(chat_id.to_string());
            }
            if let Some(message_id) = message.get("message_id").and_then(|v| v.as_str()) {
                result.message_id = Some(message_id.to_string());
            }
            if let Some(user_id) = message.get("sender_id").and_then(|v| v.get("open_id")).and_then(|v| v.as_str()) {
                result.user_id = Some(user_id.to_string());
            }
            if let Some(user_name) = message.get("sender").and_then(|v| v.get("name")).and_then(|v| v.as_str()) {
                result.user_name = Some(user_name.to_string());
            }
        }

        result
    }

    /// 发送文本消息（Webhook 方式）
    pub async fn send_text_webhook(&self, content: &str) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "text",
            "content": {
                "text": content
            }
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送富文本消息
    pub async fn send_post(&self, title: &str, content: Vec<PostContent>) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": title,
                        "content": content
                    }
                }
            }
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送交互式卡片消息
    pub async fn send_interactive(&self, card: CardContent) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "interactive",
            "card": card
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送消息到群聊 (Bot API)
    ///
    /// 使用飞书 Bot API 发送消息到指定群聊
    pub async fn send_to_chat(&mut self, chat_id: &str, content: &str) -> Result<String> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/messages",
            chat_id
        );

        let body = json!({
            "msg_type": "text",
            "content": {
                "text": content
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuSendMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        Ok(result.data.map(|d| d.message_id).unwrap_or_default())
    }

    /// 发送富文本消息到群聊 (Bot API)
    pub async fn send_post_to_chat(
        &mut self,
        chat_id: &str,
        title: &str,
        content: Vec<PostContent>,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/messages",
            chat_id
        );

        let body = json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": title,
                        "content": content
                    }
                }
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuSendMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        Ok(result.data.map(|d| d.message_id).unwrap_or_default())
    }

    /// 发送卡片消息到群聊 (Bot API)
    pub async fn send_card_to_chat(
        &mut self,
        chat_id: &str,
        card: CardContent,
    ) -> Result<String> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/messages",
            chat_id
        );

        let body = json!({
            "msg_type": "interactive",
            "card": card
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuSendMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        Ok(result.data.map(|d| d.message_id).unwrap_or_default())
    }

    /// 处理飞书事件回调
    ///
    /// 解析飞书推送的事件，如消息事件、群组事件等
    pub fn handle_event(&self, event_json: serde_json::Value) -> Result<FeishuMessageParseResult> {
        let event_type = event_json
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match event_type {
            "im.message" => {
                let message = event_json.get("message").ok_or_else(|| {
                    OpenClawError::AIProvider("无效的消息事件".to_string())
                })?;
                Ok(self.parse_event_message(&serde_json::json!({ "message": message })))
            }
            _ => Err(OpenClawError::AIProvider(format!(
                "未支持的事件类型: {}",
                event_type
            ))),
        }
    }

    /// 获取群聊列表
    pub async fn list_chats(&mut self) -> Result<Vec<FeishuGroupInfo>> {
        let token = self.get_access_token().await?;

        let url = "https://open.feishu.cn/open-apis/im/v1/chats";

        let response = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuListChatsResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        let items = result.data.map(|d| d.items).unwrap_or_default();
        Ok(items
            .unwrap_or_default()
            .into_iter()
            .map(|item| FeishuGroupInfo {
                chat_id: item.chat_id,
                name: item.name,
                avatar: item.avatar,
                description: item.description,
                member_count: None,
                owner_id: item.owner_id,
            })
            .collect())
    }

    /// 获取群成员列表
    pub async fn list_chat_members(&mut self, chat_id: &str) -> Result<Vec<FeishuGroupMember>> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/members",
            chat_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuListMembersResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        let items = result.data.map(|d| d.items).unwrap_or_default();
        Ok(items
            .unwrap_or_default()
            .into_iter()
            .map(|item| FeishuGroupMember {
                member_id: item.member_id,
                member_type: item.member_id_type,
                name: item.name,
                is_bot: item.type_ == 2,
            })
            .collect())
    }

    /// 添加群成员
    pub async fn add_chat_member(
        &mut self,
        chat_id: &str,
        member_id: &str,
        member_type: &str,
    ) -> Result<()> {
        let token = self.get_access_token().await?;

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/members",
            chat_id
        );

        let member_id_type = match member_type {
            "open_id" | "user_id" | "union_id" | "chat_id" => member_type,
            _ => "open_id",
        };

        let body = json!({
            "member_id_type": member_id_type,
            "id_list": [member_id]
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if let Some(code) = result.get("code").and_then(|v| v.as_i64()) {
            if code != 0 {
                let msg = result.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                return Err(OpenClawError::AIProvider(format!(
                    "飞书 API 返回错误: {} - {}",
                    code, msg
                )));
            }
        }

        Ok(())
    }

    /// 移除群成员
    pub async fn remove_chat_member(
        &mut self,
        chat_id: &str,
        member_id: &str,
        member_type: &str,
    ) -> Result<()> {
        let token = self.get_access_token().await?;

        let member_id_type = match member_type {
            "open_id" | "user_id" | "union_id" | "chat_id" => member_type,
            _ => "open_id",
        };

        let url = format!(
            "https://open.feishu.cn/open-apis/im/v1/chats/{}/members/{}?member_id_type={}",
            chat_id, member_id, member_id_type
        );

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if let Some(code) = result.get("code").and_then(|v| v.as_i64()) {
            if code != 0 {
                let msg = result.get("msg").and_then(|v| v.as_str()).unwrap_or("");
                return Err(OpenClawError::AIProvider(format!(
                    "飞书 API 返回错误: {} - {}",
                    code, msg
                )));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Feishu
    }

    fn name(&self) -> &str {
        "feishu"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("飞书通道未启用".to_string()));
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let message_id = uuid::Uuid::new_v4().to_string();

        // 优先使用 Webhook
        if self.config.webhook.is_some() {
            match message.message_type.as_str() {
                "text" => {
                    self.send_text_webhook(&message.content).await?;
                }
                "post" | "markdown" => {
                    let title = message.title.as_deref().unwrap_or("消息");
                    let content = vec![PostContent {
                        tag: "text".to_string(),
                        text: Some(message.content.clone()),
                        ..Default::default()
                    }];
                    self.send_post(title, content).await?;
                }
                "interactive" | "card" => {
                    let card = CardContent {
                        elements: vec![CardContentElement {
                            tag: "div".to_string(),
                            text: Some(CardText {
                                tag: "plain_text".to_string(),
                                content: message.content.clone(),
                            }),
                        }],
                    };
                    self.send_interactive(card).await?;
                }
                _ => {
                    self.send_text_webhook(&message.content).await?;
                }
            }
        } else {
            return Err(OpenClawError::Config(
                "飞书通道需要配置 webhook 或 app_id/app_secret".to_string(),
            ));
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::Feishu,
            chat_id: "feishu".to_string(),
            user_id: "bot".to_string(),
            content: message.content.clone(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.enabled)
    }
}

/// 飞书 Token 响应
#[derive(Debug, Deserialize)]
struct FeishuTokenResponse {
    code: i32,
    msg: String,
    tenant_access_token: String,
    expire: i32,
}

/// 富文本内容
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostContent {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// 卡片内容
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardContent {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<CardContentElement>,
}

/// 卡片内容元素
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardContentElement {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<CardText>,
}

/// 卡片文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardText {
    pub tag: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_channel_creation() {
        let config = FeishuConfig {
            app_id: "cli_xxx".to_string(),
            app_secret: "secret".to_string(),
            webhook: Some("https://open.feishu.cn/open-apis/bot/v2/hook/xxx".to_string()),
            enabled: true,
        };
        let channel = FeishuChannel::new(config);
        assert_eq!(channel.name(), "feishu");
    }
}
