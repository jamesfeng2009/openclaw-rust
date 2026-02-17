//! iMessage 通道实现 (macOS 专用)
//!
//! 通过 AppleScript 和 macOS 消息应用发送 iMessage
//! 仅支持 macOS 系统

use async_trait::async_trait;
use chrono::Utc;
use openclaw_core::{OpenClawError, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{debug, error, info, warn};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// iMessage 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageConfig {
    /// 发送者 Apple ID
    pub apple_id: Option<String>,
    /// 是否启用短信回退
    pub enable_sms_fallback: bool,
    /// 消息超时 (秒)
    pub timeout: u64,
}

impl Default for IMessageConfig {
    fn default() -> Self {
        Self {
            apple_id: None,
            enable_sms_fallback: true,
            timeout: 30,
        }
    }
}

/// iMessage 通道 (macOS 专用)
pub struct IMessageChannel {
    config: IMessageConfig,
    running: bool,
}

impl IMessageChannel {
    pub fn new(config: IMessageConfig) -> Self {
        Self {
            config,
            running: false,
        }
    }

    /// 检查是否在 macOS 上运行
    fn check_macos() -> bool {
        #[cfg(target_os = "macos")]
        {
            true
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// 通过 AppleScript 发送 iMessage
    #[cfg(target_os = "macos")]
    fn send_applescript(&self, recipient: &str, message: &str) -> Result<String> {
        let script = format!(
            r#"
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "{}" of targetService
    send "{}" to targetBuddy
    return "sent"
end tell
"#,
            recipient.replace("\"", "\\\""),
            message.replace("\"", "\\\"").replace("\\", "\\\\")
        );

        debug!("执行 AppleScript: 发送消息到 {}", recipient);

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| OpenClawError::Execution(format!("执行 AppleScript 失败: {}", e)))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout).to_string();
            info!("iMessage 发送成功: {}", recipient);
            Ok(result)
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
            error!("iMessage 发送失败: {}", error_msg);

            // 如果 iMessage 失败，尝试短信回退
            if self.config.enable_sms_fallback {
                warn!("尝试短信回退...");
                return self.send_sms_fallback(recipient, message);
            }

            Err(OpenClawError::Execution(format!(
                "AppleScript 错误: {}",
                error_msg
            )))
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn send_applescript(&self, _recipient: &str, _message: &str) -> Result<String> {
        Err(OpenClawError::Platform(
            "iMessage 仅支持 macOS 系统".to_string(),
        ))
    }

    /// 短信回退 (使用 macOS 短信服务)
    #[cfg(target_os = "macos")]
    fn send_sms_fallback(&self, phone: &str, message: &str) -> Result<String> {
        let script = format!(
            r#"
tell application "Messages"
    set targetService to 1st service whose service type = SMS
    set targetBuddy to buddy "{}" of targetService
    send "{}" to targetBuddy
    return "sent"
end tell
"#,
            phone.replace("\"", "\\\""),
            message.replace("\"", "\\\"").replace("\\", "\\\\")
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| OpenClawError::Execution(format!("执行 AppleScript 失败: {}", e)))?;

        if output.status.success() {
            info!("短信发送成功: {}", phone);
            Ok("sent via SMS".to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            Err(OpenClawError::Execution(format!("短信发送失败: {}", error)))
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn send_sms_fallback(&self, _phone: &str, _message: &str) -> Result<String> {
        Err(OpenClawError::Platform("短信仅支持 macOS 系统".to_string()))
    }

    /// 获取最近的 iMessage 消息
    #[cfg(target_os = "macos")]
    fn get_recent_messages(&self) -> Result<Vec<IMessageData>> {
        let script = r#"
tell application "Messages"
    set output to ""
    set targetChat to chat 1
    set messageList to messages of targetChat
    repeat with msg in messageList
        set msgText to content of msg
        set msgSender to sender of msg
        set msgDate to date of msg
        set output to output & msgSender & "|" & msgText & "|" & msgDate & "
"
    end repeat
    return output
end tell
"#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| OpenClawError::Execution(format!("获取消息失败: {}", e)))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            let messages: Vec<IMessageData> = result
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(3, '|').collect();
                    if parts.len() >= 3 {
                        Some(IMessageData {
                            sender: parts[0].to_string(),
                            content: parts[1].to_string(),
                            date: parts[2].to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            Ok(messages)
        } else {
            Ok(Vec::new())
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn get_recent_messages(&self) -> Result<Vec<IMessageData>> {
        Ok(Vec::new())
    }
}

/// iMessage 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageData {
    pub sender: String,
    pub content: String,
    pub date: String,
}

/// iMessage 联系人
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IMessageContact {
    pub name: String,
    pub phone: String,
    pub email: Option<String>,
}

#[async_trait]
impl Channel for IMessageChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::IMessage
    }

    fn name(&self) -> &str {
        "imessage"
    }

    async fn start(&mut self) -> Result<()> {
        if !Self::check_macos() {
            warn!("iMessage 通道仅支持 macOS 系统，当前系统不支持");
            return Err(OpenClawError::Platform("iMessage 仅支持 macOS".to_string()));
        }

        info!("启动 iMessage 通道");

        // 检查 Messages 应用是否可用
        #[cfg(target_os = "macos")]
        {
            let script = r#"
tell application "System Events"
    return exists application process "Messages"
end tell
"#;
            let output = Command::new("osascript").arg("-e").arg(script).output();

            match output {
                Ok(o) if o.status.success() => {
                    let result = String::from_utf8_lossy(&o.stdout);
                    if result.contains("true") {
                        info!("Messages 应用可用");
                    } else {
                        warn!("Messages 应用未运行，将在发送时自动启动");
                    }
                }
                _ => {
                    warn!("无法检查 Messages 应用状态");
                }
            }
        }

        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        info!("停止 iMessage 通道");
        self.running = false;
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        debug!("发送 iMessage 到: {}", message.chat_id);

        let result = self.send_applescript(&message.chat_id, &message.content)?;

        Ok(ChannelMessage {
            id: chrono::Utc::now().timestamp_millis().to_string(),
            channel_type: ChannelType::IMessage,
            chat_id: message.chat_id,
            user_id: self.config.apple_id.clone().unwrap_or_default(),
            content: message.content,
            timestamp: Utc::now(),
            metadata: Some(serde_json::json!({
                "method": "imessage",
                "result": result,
            })),
        })
    }

    async fn health_check(&self) -> Result<bool> {
        if !Self::check_macos() {
            return Ok(false);
        }

        #[cfg(target_os = "macos")]
        {
            let script = r#"
tell application "System Events"
    return exists application process "Messages"
end tell
"#;
            let output = Command::new("osascript").arg("-e").arg(script).output();

            match output {
                Ok(o) => Ok(o.status.success()),
                Err(_) => Ok(false),
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            Ok(false)
        }
    }
}

// ============ 扩展功能 ============

impl IMessageChannel {
    /// 发送富文本消息 (带附件)
    #[cfg(target_os = "macos")]
    pub async fn send_with_attachment(
        &self,
        recipient: &str,
        message: &str,
        attachment_path: &str,
    ) -> Result<ChannelMessage> {
        let script = format!(
            r#"
tell application "Messages"
    set targetService to 1st service whose service type = iMessage
    set targetBuddy to buddy "{}" of targetService
    send POSIX file "{}" to targetBuddy
    send "{}" to targetBuddy
end tell
"#,
            recipient.replace("\"", "\\\""),
            attachment_path.replace("\"", "\\\""),
            message.replace("\"", "\\\"")
        );

        let output = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| OpenClawError::Execution(format!("发送附件失败: {}", e)))?;

        if output.status.success() {
            info!("iMessage 附件发送成功");
            Ok(ChannelMessage {
                id: chrono::Utc::now().timestamp_millis().to_string(),
                channel_type: ChannelType::IMessage,
                chat_id: recipient.to_string(),
                user_id: self.config.apple_id.clone().unwrap_or_default(),
                content: message.to_string(),
                timestamp: Utc::now(),
                metadata: Some(serde_json::json!({
                    "has_attachment": true,
                    "attachment_path": attachment_path,
                })),
            })
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(OpenClawError::Execution(format!("发送附件失败: {}", error)))
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub async fn send_with_attachment(
        &self,
        _recipient: &str,
        _message: &str,
        _attachment_path: &str,
    ) -> Result<ChannelMessage> {
        Err(OpenClawError::Platform("iMessage 仅支持 macOS".to_string()))
    }

    /// 发送 Tapback (表情回应)
    #[cfg(target_os = "macos")]
    pub async fn send_tapback(&self, message_id: &str, tapback_type: TapbackType) -> Result<()> {
        let tapback_code = match tapback_type {
            TapbackType::Heart => "heart",
            TapbackType::ThumbsUp => "like",
            TapbackType::ThumbsDown => "dislike",
            TapbackType::HaHa => "laugh",
            TapbackType::Exclamation => "exclaim",
            TapbackType::Question => "question",
        };

        debug!("发送 Tapback: {} for message {}", tapback_code, message_id);
        // 注意: Tapback 实现需要 UI 脚本，这里提供框架
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub async fn send_tapback(&self, _message_id: &str, _tapback_type: TapbackType) -> Result<()> {
        Err(OpenClawError::Platform("iMessage 仅支持 macOS".to_string()))
    }
}

/// Tapback 类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TapbackType {
    Heart,
    ThumbsUp,
    ThumbsDown,
    HaHa,
    Exclamation,
    Question,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imessage_config_default() {
        let config = IMessageConfig::default();
        assert!(config.enable_sms_fallback);
        assert_eq!(config.timeout, 30);
    }

    #[test]
    fn test_channel_name() {
        let config = IMessageConfig::default();
        let channel = IMessageChannel::new(config);
        assert_eq!(channel.name(), "imessage");
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn test_non_macos() {
        let config = IMessageConfig::default();
        let mut channel = IMessageChannel::new(config);

        // 在非 macOS 上应该返回错误
        let result = channel.start().await;
        assert!(result.is_err());
    }
}
