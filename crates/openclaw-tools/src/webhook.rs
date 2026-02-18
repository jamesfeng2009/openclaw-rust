//! Webhook 系统模块

use crate::types::*;
use chrono::Utc;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Webhook 错误
#[derive(Debug, Error)]
pub enum WebhookError {
    #[error("Webhook 不存在: {0}")]
    WebhookNotFound(WebhookId),

    #[error("发送失败: {0}")]
    SendFailed(String),

    #[error("签名验证失败")]
    SignatureInvalid,

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Webhook 管理器
pub struct WebhookManager {
    webhooks: Arc<RwLock<HashMap<WebhookId, WebhookConfig>>>,
    triggers: Arc<RwLock<Vec<WebhookTrigger>>>,
    client: Client,
}

impl WebhookManager {
    /// 创建新的 Webhook 管理器
    pub fn new() -> Self {
        Self {
            webhooks: Arc::new(RwLock::new(HashMap::new())),
            triggers: Arc::new(RwLock::new(Vec::new())),
            client: Client::new(),
        }
    }

    /// 创建 Webhook
    pub async fn create_webhook(
        &self,
        name: String,
        url: String,
        events: Vec<WebhookEvent>,
        secret: Option<String>,
    ) -> WebhookId {
        let webhook = WebhookConfig {
            id: Uuid::new_v4().to_string(),
            name,
            url,
            secret,
            events,
            headers: HashMap::new(),
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_triggered: None,
            trigger_count: 0,
        };

        let id = webhook.id.clone();

        let mut webhooks = self.webhooks.write().await;
        webhooks.insert(id.clone(), webhook);

        info!("创建 Webhook: {}", id);
        id
    }

    /// 更新 Webhook
    pub async fn update_webhook(
        &self,
        webhook_id: &WebhookId,
        updates: WebhookUpdates,
    ) -> Result<(), WebhookError> {
        let mut webhooks = self.webhooks.write().await;

        let webhook = webhooks
            .get_mut(webhook_id)
            .ok_or_else(|| WebhookError::WebhookNotFound(webhook_id.clone()))?;

        if let Some(name) = updates.name {
            webhook.name = name;
        }
        if let Some(url) = updates.url {
            webhook.url = url;
        }
        if let Some(events) = updates.events {
            webhook.events = events;
        }
        if let Some(enabled) = updates.enabled {
            webhook.enabled = enabled;
        }
        webhook.updated_at = Utc::now();

        info!("更新 Webhook: {}", webhook_id);
        Ok(())
    }

    /// 删除 Webhook
    pub async fn delete_webhook(&self, webhook_id: &WebhookId) -> Result<(), WebhookError> {
        let mut webhooks = self.webhooks.write().await;

        if webhooks.remove(webhook_id).is_some() {
            info!("删除 Webhook: {}", webhook_id);
            Ok(())
        } else {
            Err(WebhookError::WebhookNotFound(webhook_id.clone()))
        }
    }

    /// 获取 Webhook
    pub async fn get_webhook(&self, webhook_id: &WebhookId) -> Option<WebhookConfig> {
        let webhooks = self.webhooks.read().await;
        webhooks.get(webhook_id).cloned()
    }

    /// 列出所有 Webhooks
    pub async fn list_webhooks(&self) -> Vec<WebhookConfig> {
        let webhooks = self.webhooks.read().await;
        webhooks.values().cloned().collect()
    }

    /// 触发事件
    pub async fn trigger_event(
        &self,
        event: WebhookEvent,
        payload: serde_json::Value,
    ) -> Vec<WebhookTrigger> {
        let webhooks = self.webhooks.read().await;
        let mut results = vec![];

        for webhook in webhooks.values() {
            if !webhook.enabled {
                continue;
            }

            // 检查事件是否匹配
            let matches = webhook.events.iter().any(|e| match (e, &event) {
                (WebhookEvent::SandboxCreated, WebhookEvent::SandboxCreated) => true,
                (WebhookEvent::SandboxStarted, WebhookEvent::SandboxStarted) => true,
                (WebhookEvent::SandboxStopped, WebhookEvent::SandboxStopped) => true,
                (WebhookEvent::CanvasCreated, WebhookEvent::CanvasCreated) => true,
                (WebhookEvent::CanvasUpdated, WebhookEvent::CanvasUpdated) => true,
                (WebhookEvent::CanvasDeleted, WebhookEvent::CanvasDeleted) => true,
                (WebhookEvent::ToolExecuted, WebhookEvent::ToolExecuted) => true,
                (WebhookEvent::TaskCompleted, WebhookEvent::TaskCompleted) => true,
                (WebhookEvent::TaskFailed, WebhookEvent::TaskFailed) => true,
                (WebhookEvent::SystemStarted, WebhookEvent::SystemStarted) => true,
                (WebhookEvent::SystemStopped, WebhookEvent::SystemStopped) => true,
                (WebhookEvent::Custom { name: n1 }, WebhookEvent::Custom { name: n2 }) => n1 == n2,
                _ => false,
            });

            if !matches {
                continue;
            }

            // 发送 Webhook
            let trigger = self.send_webhook(webhook, &event, payload.clone()).await;
            results.push(trigger);
        }

        results
    }

    /// 发送 Webhook
    async fn send_webhook(
        &self,
        webhook: &WebhookConfig,
        event: &WebhookEvent,
        payload: serde_json::Value,
    ) -> WebhookTrigger {
        let start = std::time::Instant::now();
        let trigger_id = Uuid::new_v4().to_string();

        let body = serde_json::json!({
            "event": event,
            "timestamp": Utc::now().to_rfc3339(),
            "data": payload,
        });

        let body_str = serde_json::to_string(&body).unwrap_or_default();

        // 计算签名
        let signature = if let Some(ref secret) = webhook.secret {
            let mut hasher = Sha256::new();
            hasher.update(secret.as_bytes());
            hasher.update(body_str.as_bytes());
            Some(format!("sha256={}", hex::encode(hasher.finalize())))
        } else {
            None
        };

        // 构建请求
        let mut request = self
            .client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header(
                "X-Webhook-Event",
                serde_json::to_string(event).unwrap_or_default(),
            )
            .header("X-Webhook-ID", &trigger_id)
            .body(body_str.clone());

        if let Some(ref sig) = signature {
            request = request.header("X-Webhook-Signature", sig);
        }

        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        // 发送请求
        let result = request.send().await;

        let (status, response_code, response_body, error) = match result {
            Ok(response) => {
                let code = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                if (200..300).contains(&code) {
                    (TriggerStatus::Success, Some(code), Some(body.clone()), None)
                } else {
                    (
                        TriggerStatus::Failed,
                        Some(code),
                        Some(body),
                        Some(format!("HTTP {}", code)),
                    )
                }
            }
            Err(e) => (TriggerStatus::Failed, None, None, Some(e.to_string())),
        };

        let duration = start.elapsed().as_millis() as u64;

        let trigger = WebhookTrigger {
            id: trigger_id,
            webhook_id: webhook.id.clone(),
            event: event.clone(),
            payload,
            status,
            response_code,
            response_body,
            error: error.clone(),
            triggered_at: Utc::now(),
            duration_ms: duration,
        };

        // 记录触发
        {
            let mut triggers = self.triggers.write().await;
            triggers.push(trigger.clone());
        }

        // 更新 Webhook 统计
        {
            let mut webhooks = self.webhooks.write().await;
            if let Some(wh) = webhooks.get_mut(&webhook.id) {
                wh.trigger_count += 1;
                wh.last_triggered = Some(Utc::now());
            }
        }

        if status == TriggerStatus::Success {
            info!("Webhook {} 触发成功 ({}ms)", webhook.id, duration);
        } else if let Some(err) = &error {
            warn!("Webhook {} 触发失败: {:?}", webhook.id, err);
        }

        trigger
    }

    /// 验证签名
    pub fn verify_signature(&self, secret: &str, body: &[u8], signature: &str) -> bool {
        let expected = signature.trim_start_matches("sha256=");

        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(body);
        let computed = hex::encode(hasher.finalize());

        computed == expected
    }

    /// 获取触发历史
    pub async fn get_trigger_history(&self, webhook_id: Option<&WebhookId>) -> Vec<WebhookTrigger> {
        let triggers = self.triggers.read().await;

        match webhook_id {
            Some(id) => triggers
                .iter()
                .filter(|t| &t.webhook_id == id)
                .cloned()
                .collect(),
            None => triggers.clone(),
        }
    }
}

impl Default for WebhookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Webhook 更新字段
pub struct WebhookUpdates {
    pub name: Option<String>,
    pub url: Option<String>,
    pub events: Option<Vec<WebhookEvent>>,
    pub enabled: Option<bool>,
}
