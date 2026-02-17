//! Signal 通道实现
//!
//! 使用 Signal CLI REST API 进行消息收发
//! 隐私优先的即时通讯平台

use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Signal 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Signal CLI REST API 地址
    pub api_url: String,
    /// 电话号码 (含国家码)
    pub phone_number: String,
    /// 是否自动接收消息
    pub auto_receive: bool,
    /// 消息轮询间隔 (秒)
    pub poll_interval: u64,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:8080".to_string(),
            phone_number: String::new(),
            auto_receive: true,
            poll_interval: 5,
        }
    }
}

/// Signal 通道
pub struct SignalChannel {
    config: SignalConfig,
    client: Client,
    message_tx: Option<mpsc::Sender<ChannelMessage>>,
    running: bool,
}

impl SignalChannel {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            config,
            client: Client::new(),
            message_tx: None,
            running: false,
        }
    }

    /// 获取 API 端点
    fn endpoint(&self, path: &str) -> String {
        format!("{}/v1{}", self.config.api_url, path)
    }

    /// 发送文本消息
    async fn send_text(&self, to: &str, message: &str) -> Result<SignalMessageResponse> {
        let url = self.endpoint(&format!("/send/{}", self.config.phone_number));

        let body = SendMessageRequest {
            message: message.to_string(),
            number: to.to_string(),
            recipients: None,
        };

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(format!("Signal API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Api(format!(
                "Signal 发送失败: {}",
                error_text
            )));
        }

        response
            .json::<SignalMessageResponse>()
            .await
            .map_err(|e| OpenClawError::Parse(format!("解析响应失败: {}", e)))
    }

    /// 获取消息列表
    async fn get_messages(&self) -> Result<Vec<SignalMessage>> {
        let url = self.endpoint(&format!("/messages/{}", self.config.phone_number));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(format!("Signal API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            return Ok(Vec::new());
        }

        response
            .json::<Vec<SignalMessage>>()
            .await
            .map_err(|e| OpenClawError::Parse(format!("解析消息失败: {}", e)))
    }

    /// 将 Signal 消息转换为通道消息
    fn convert_message(&self, msg: SignalMessage) -> ChannelMessage {
        ChannelMessage {
            id: msg.timestamp.to_string(),
            channel_type: ChannelType::Signal,
            chat_id: msg.source.clone(),
            user_id: msg.source.clone(),
            content: msg.message.clone().unwrap_or_default(),
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "source": msg.source,
                "source_device": msg.source_device,
                "timestamp": msg.timestamp,
                "is_read": msg.is_read,
                "attachments": msg.attachments,
                "group_id": msg.group_id,
            })),
        }
    }
}

#[async_trait]
impl Channel for SignalChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Signal
    }

    fn name(&self) -> &str {
        "signal"
    }

    async fn start(&mut self) -> Result<()> {
        info!("启动 Signal 通道: {}", self.config.phone_number);

        // 检查 API 连接
        let health_url = self.endpoint("/health");
        match self.client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                info!("Signal CLI API 连接成功");
            }
            Ok(resp) => {
                warn!("Signal CLI API 返回错误状态: {}", resp.status());
            }
            Err(e) => {
                warn!(
                    "无法连接 Signal CLI API: {}。请确保 signal-cli-rest-api 服务已启动",
                    e
                );
            }
        }

        // 启动消息接收循环
        if self.config.auto_receive {
            let poll_interval = self.config.poll_interval;
            let api_url = self.config.api_url.clone();
            let phone_number = self.config.phone_number.clone();
            let client = self.client.clone();

            tokio::spawn(async move {
                info!("Signal 消息轮询已启动");
                // 消息通过轮询获取，不使用 channel
                loop {
                    // 获取消息
                    let url = format!("{}/v1/messages/{}", api_url, phone_number);
                    match client.get(&url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            // 消息处理逻辑
                            let _ = resp.json::<Vec<SignalMessage>>().await;
                        }
                        _ => {}
                    }

                    tokio::time::sleep(tokio::time::Duration::from_secs(poll_interval)).await;
                }
            });
        }

        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("停止 Signal 通道");
        self.running = false;
        self.message_tx = None;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        debug!("发送 Signal 消息到: {}", message.chat_id);

        let response = self.send_text(&message.chat_id, &message.content).await?;

        Ok(ChannelMessage {
            id: response.timestamp.to_string(),
            channel_type: ChannelType::Signal,
            chat_id: message.chat_id,
            user_id: self.config.phone_number.clone(),
            content: message.content,
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "message_id": response.id,
                "timestamp": response.timestamp,
            })),
        })
    }

    fn messages(&self) -> Option<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>> {
        // Signal 使用轮询模式，返回空流
        None
    }

    async fn health_check(&self) -> Result<bool> {
        let url = self.endpoint("/health");

        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

// ============ API 类型定义 ============

/// 发送消息请求
#[derive(Debug, Serialize)]
struct SendMessageRequest {
    message: String,
    number: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    recipients: Option<Vec<String>>,
}

/// Signal 消息响应
#[derive(Debug, Deserialize)]
struct SignalMessageResponse {
    id: String,
    timestamp: i64,
}

/// Signal 消息
#[derive(Debug, Clone, Deserialize)]
struct SignalMessage {
    /// 发送者号码
    source: String,
    /// 发送者设备
    source_device: Option<i32>,
    /// 时间戳
    timestamp: i64,
    /// 消息内容
    message: Option<String>,
    /// 是否已读
    is_read: Option<bool>,
    /// 附件列表
    attachments: Option<Vec<SignalAttachment>>,
    /// 群组 ID
    group_id: Option<String>,
}

/// Signal 附件
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignalAttachment {
    content_type: String,
    filename: Option<String>,
    id: Option<String>,
    size: Option<i64>,
}

/// Signal 群组信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalGroup {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
    pub avatar: Option<String>,
}

// ============ 群组管理功能 ============

impl SignalChannel {
    /// 获取群组列表
    pub async fn list_groups(&self) -> Result<Vec<SignalGroup>> {
        let url = self.endpoint(&format!("/groups/{}", self.config.phone_number));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(format!("获取群组失败: {}", e)))?;

        response
            .json::<Vec<SignalGroup>>()
            .await
            .map_err(|e| OpenClawError::Parse(format!("解析群组失败: {}", e)))
    }

    /// 创建群组
    pub async fn create_group(&self, name: &str, members: &[String]) -> Result<SignalGroup> {
        let url = self.endpoint(&format!("/groups/{}", self.config.phone_number));

        let body = serde_json::json!({
            "name": name,
            "members": members,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(format!("创建群组失败: {}", e)))?;

        response
            .json::<SignalGroup>()
            .await
            .map_err(|e| OpenClawError::Parse(format!("解析群组失败: {}", e)))
    }

    /// 发送群组消息
    pub async fn send_group_message(
        &self,
        group_id: &str,
        message: &str,
    ) -> Result<ChannelMessage> {
        let url = self.endpoint(&format!("/send/{}", self.config.phone_number));

        let body = serde_json::json!({
            "message": message,
            "recipients": [group_id],
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(format!("发送群组消息失败: {}", e)))?;

        let resp = response
            .json::<SignalMessageResponse>()
            .await
            .map_err(|e| OpenClawError::Parse(format!("解析响应失败: {}", e)))?;

        Ok(ChannelMessage {
            id: resp.timestamp.to_string(),
            channel_type: ChannelType::Signal,
            chat_id: group_id.to_string(),
            user_id: self.config.phone_number.clone(),
            content: message.to_string(),
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "message_id": resp.id,
                "is_group": true,
            })),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_config_default() {
        let config = SignalConfig::default();
        assert_eq!(config.api_url, "http://localhost:8080");
        assert!(config.auto_receive);
    }

    #[test]
    fn test_channel_type() {
        let config = SignalConfig::default();
        let channel = SignalChannel::new(config);
        assert_eq!(channel.channel_type(), ChannelType::Signal);
    }
}
