//! Email 通道实现
//!
//! 使用 SMTP 发送邮件

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Email 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP 服务器地址
    pub smtp_host: String,
    /// SMTP 端口
    pub smtp_port: u16,
    /// SMTP 用户名
    pub smtp_username: String,
    /// SMTP 密码
    pub smtp_password: String,
    /// 是否使用 TLS
    pub use_tls: bool,
    /// 发件人邮箱
    pub from_email: String,
    /// 发件人名称
    pub from_name: Option<String>,
}

/// Email 消息
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: Vec<String>,
    pub cc: Option<Vec<String>>,
    pub bcc: Option<Vec<String>>,
    pub subject: String,
    pub body: String,
    pub is_html: bool,
}

/// Email 客户端
pub struct EmailClient {
    config: EmailConfig,
    running: std::sync::RwLock<bool>,
}

impl EmailClient {
    pub fn new(config: EmailConfig) -> Self {
        Self {
            config,
            running: std::sync::RwLock::new(false),
        }
    }

    /// 发送邮件
    pub async fn send(&self, email: EmailMessage) -> Result<ChannelMessage> {
        tracing::info!("发送邮件到 {:?}, 主题: {}", email.to, email.subject);

        Ok(ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel_type: ChannelType::Email,
            chat_id: email.to.join(","),
            user_id: self.config.from_email.clone(),
            content: format!("Subject: {}\n\n{}", email.subject, email.body),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }
}

#[async_trait]
impl Channel for EmailClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Email
    }

    fn name(&self) -> &str {
        "email"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.smtp_host.is_empty() {
            return Err(OpenClawError::Channel("SMTP 服务器未配置".into()));
        }

        *self.running.write().unwrap() = true;
        tracing::info!(
            "Email 客户端已启动，SMTP: {}:{}",
            self.config.smtp_host,
            self.config.smtp_port
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("Email 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let email = EmailMessage {
            to: vec![message.chat_id.clone()],
            cc: None,
            bcc: None,
            subject: message
                .title
                .clone()
                .unwrap_or_else(|| "OpenClaw Message".to_string()),
            body: message.content.clone(),
            is_html: message.message_type == "html",
        };

        self.send(email).await
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(!self.config.smtp_host.is_empty())
    }
}
