//! Matrix 通道实现
//!
//! Matrix 是一个开放的去中心化通信协议

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Matrix 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixConfig {
    /// Matrix 服务器 URL (例如 https://matrix.org)
    pub homeserver: String,
    /// 访问令牌
    pub access_token: Option<String>,
    /// 用户 ID (例如 @user:matrix.org)
    pub user_id: Option<String>,
    /// 设备 ID
    pub device_id: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// Matrix 房间成员
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixRoomMember {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

/// Matrix 消息事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixRoomEvent {
    pub event_id: String,
    pub sender: String,
    pub room_id: String,
    pub origin_server_ts: u64,
    pub content: MatrixEventContent,
    pub unsigned: Option<serde_json::Value>,
}

/// Matrix 事件内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "msgtype", rename_all = "snake_case")]
pub enum MatrixEventContent {
    #[serde(rename = "m.text")]
    Text {
        body: String,
        format: Option<String>,
        formatted_body: Option<String>,
    },
    #[serde(rename = "m.image")]
    Image {
        url: String,
        info: Option<serde_json::Value>,
        thumbnail_url: Option<String>,
        thumbnail_info: Option<serde_json::Value>,
    },
    #[serde(rename = "m.file")]
    File {
        url: String,
        info: Option<serde_json::Value>,
        filename: String,
    },
    Unknown,
}

/// Matrix 客户端
pub struct MatrixClient {
    config: MatrixConfig,
    client: reqwest::Client,
    next_batch: std::sync::RwLock<Option<String>>,
    running: std::sync::RwLock<bool>,
}

impl MatrixClient {
    pub fn new(config: MatrixConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            next_batch: std::sync::RwLock::new(None),
            running: std::sync::RwLock::new(false),
            config,
        }
    }

    /// 获取 API URL
    fn get_api_url(&self, endpoint: &str) -> String {
        let homeserver = self.config.homeserver.trim_end_matches('/');
        format!("{}{}", homeserver, endpoint)
    }

    /// 发送请求（带认证）
    async fn request<T: for<'de> Deserialize<'de>>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T> {
        let url = self.get_api_url(endpoint);

        let mut request = self.client.request(method, &url);

        if let Some(token) = &self.config.access_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Matrix API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Matrix API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Matrix 响应失败: {}", e)))
    }

    /// 登录
    pub async fn login(&mut self, username: &str, password: &str) -> Result<MatrixLoginResponse> {
        let body = serde_json::json!({
            "type": "m.login.password",
            "identifier": {
                "type": "m.id.user",
                "user": username
            },
            "password": password,
            "initial_device_display_name": "OpenClaw Rust"
        });

        let response: MatrixLoginResponse = self
            .request(
                reqwest::Method::POST,
                "/_matrix/client/r0/login",
                Some(body),
            )
            .await?;

        self.config.access_token = Some(response.access_token.clone());
        self.config.user_id = Some(response.user_id.clone());
        self.config.device_id = Some(response.device_id.clone());

        Ok(response)
    }

    /// 发送房间消息
    pub async fn send_message(
        &self,
        room_id: &str,
        message_type: &str,
        body: &str,
    ) -> Result<MatrixSendResponse> {
        let txn_id = format!("m{}", uuid::Uuid::new_v4());

        let content = match message_type {
            "text" | "markdown" => {
                serde_json::json!({
                    "msgtype": "m.text",
                    "body": body
                })
            }
            "image" => {
                serde_json::json!({
                    "msgtype": "m.image",
                    "body": body,
                    "url": body
                })
            }
            _ => {
                serde_json::json!({
                    "msgtype": "m.text",
                    "body": body
                })
            }
        };

        let endpoint = format!(
            "/_matrix/client/r0/rooms/{}/send/m.room.message/{}",
            room_id, txn_id
        );

        self.request(reqwest::Method::PUT, &endpoint, Some(content))
            .await
    }

    /// 获取房间列表
    pub async fn get_joined_rooms(&self) -> Result<MatrixRoomsResponse> {
        self.request(
            reqwest::Method::GET,
            "/_matrix/client/r0/joined_rooms",
            None,
        )
        .await
    }

    /// 获取房间成员
    pub async fn get_room_members(&self, room_id: &str) -> Result<Vec<MatrixRoomMember>> {
        let endpoint = format!("/_matrix/client/r0/rooms/{}/members", room_id);

        #[derive(Deserialize)]
        struct MembersResponse {
            members: Vec<MatrixRoomMember>,
        }

        let response: MembersResponse = self.request(reqwest::Method::GET, &endpoint, None).await?;

        Ok(response.members)
    }

    /// 同步最新消息
    pub async fn sync(&self, timeout: u64) -> Result<MatrixSyncResponse> {
        let mut params = vec![("timeout", timeout.to_string())];

        if let Some(batch) = self.next_batch.read().unwrap().as_ref() {
            params.push(("since", batch.clone()));
        }

        let query = serde_qs::to_string(&params)
            .map_err(|e| OpenClawError::Config(format!("构建查询失败: {}", e)))?;

        let endpoint = format!("/_matrix/client/r0/sync?{}", query);

        let response: MatrixSyncResponse =
            self.request(reqwest::Method::GET, &endpoint, None).await?;

        if let Some(next_batch) = response.next_batch.clone() {
            *self.next_batch.write().unwrap() = Some(next_batch);
        }

        Ok(response)
    }

    /// 加入房间
    pub async fn join_room(&self, room_id_or_alias: &str) -> Result<MatrixJoinResponse> {
        let endpoint = format!(
            "/_matrix/client/r0/join/{}",
            urlencoding::encode(room_id_or_alias)
        );

        self.request(
            reqwest::Method::POST,
            &endpoint,
            Some(serde_json::json!({})),
        )
        .await
    }

    /// 创建房间
    pub async fn create_room(
        &self,
        name: Option<&str>,
        topic: Option<&str>,
        is_direct: bool,
    ) -> Result<MatrixCreateRoomResponse> {
        let mut body = serde_json::json!({
            "visibility": "public",
            "is_direct": is_direct,
        });

        if let Some(name) = name {
            body["name"] = serde_json::json!(name);
        }

        if let Some(topic) = topic {
            body["topic"] = serde_json::json!(topic);
        }

        self.request(
            reqwest::Method::POST,
            "/_matrix/client/r0/createRoom",
            Some(body),
        )
        .await
    }
}

#[derive(Debug, Deserialize)]
pub struct MatrixLoginResponse {
    pub access_token: String,
    pub device_id: String,
    pub user_id: String,
    pub home_server: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixSendResponse {
    pub event_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MatrixRoomsResponse {
    pub joined_rooms: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixJoinResponse {
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MatrixCreateRoomResponse {
    pub room_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MatrixSyncResponse {
    #[serde(default)]
    pub next_batch: Option<String>,
    pub rooms: Option<MatrixSyncRooms>,
    pub presence: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct MatrixSyncRooms {
    pub join: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub invite: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub leave: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[async_trait]
impl Channel for MatrixClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Matrix
    }

    fn name(&self) -> &str {
        "matrix"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.access_token.is_none() {
            return Err(OpenClawError::Channel(
                "Matrix access token 未配置，请先登录".into(),
            ));
        }

        *self.running.write().unwrap() = true;
        tracing::info!(
            "Matrix 客户端已启动，Homeserver: {}",
            self.config.homeserver
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("Matrix 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self
            .send_message(&message.chat_id, &message.message_type, &message.content)
            .await?;

        Ok(ChannelMessage {
            id: response.event_id,
            channel_type: ChannelType::Matrix,
            chat_id: message.chat_id,
            user_id: self.config.user_id.clone().unwrap_or_default(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        if self.config.access_token.is_none() {
            return Ok(false);
        }

        match self.get_joined_rooms().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Matrix 消息解析辅助函数
pub fn parse_matrix_message(event: &serde_json::Value) -> Option<ChannelMessage> {
    let event_type = event.get("type")?.as_str()?;
    if event_type != "m.room.message" {
        return None;
    }

    let content = event.get("content")?;
    let msgtype = content.get("msgtype")?.as_str()?;
    if msgtype != "m.text" {
        return None;
    }

    let body = content.get("body")?.as_str()?;

    Some(ChannelMessage {
        id: event.get("event_id")?.as_str()?.to_string(),
        channel_type: ChannelType::Matrix,
        chat_id: event.get("room_id")?.as_str()?.to_string(),
        user_id: event.get("sender")?.as_str()?.to_string(),
        content: body.to_string(),
        timestamp: chrono::Utc::now(),
        metadata: Some(event.clone()),
    })
}
