use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::classifier::PromptCategory;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuditEventType {
    InputReceived,
    InputFiltered,
    LlmClassified,
    OutputValidated,
    ToolExecuted,
    PermissionChecked,
    NetworkAllowed,
    NetworkDenied,
    SessionStarted,
    SessionEnded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub tool_id: Option<String>,
    pub action: Option<String>,
    pub target: Option<String>,
    pub input_preview: Option<String>,
    pub output_preview: Option<String>,
    pub metadata: HashMap<String, String>,
    pub result: String,
    pub duration_ms: Option<u64>,
}

impl AuditEvent {
    pub fn new(event_type: AuditEventType, severity: AuditSeverity) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            severity,
            session_id: None,
            user_id: None,
            tool_id: None,
            action: None,
            target: None,
            input_preview: None,
            output_preview: None,
            metadata: HashMap::new(),
            result: String::new(),
            duration_ms: None,
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    pub fn with_user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    pub fn with_tool(mut self, tool_id: &str) -> Self {
        self.tool_id = Some(tool_id.to_string());
        self
    }

    pub fn with_action(mut self, action: &str) -> Self {
        self.action = Some(action.to_string());
        self
    }

    pub fn with_target(mut self, target: &str) -> Self {
        self.target = Some(target.to_string());
        self
    }

    pub fn with_input(mut self, input: &str) -> Self {
        let preview = if input.len() > 200 {
            format!("{}...", &input[..200])
        } else {
            input.to_string()
        };
        self.input_preview = Some(preview);
        self
    }

    pub fn with_output(mut self, output: &str) -> Self {
        let preview = if output.len() > 200 {
            format!("{}...", &output[..200])
        } else {
            output.to_string()
        };
        self.output_preview = Some(preview);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_result(mut self, result: &str) -> Self {
        self.result = result.to_string();
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }
}

pub struct AuditLogger {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    stats: Arc<RwLock<HashMap<AuditEventType, u32>>>,
    session_stats: Arc<RwLock<HashMap<String, u32>>>,
    max_events: usize,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            session_stats: Arc::new(RwLock::new(HashMap::new())),
            max_events: 10000,
        }
    }

    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    pub async fn log(&self, event: AuditEvent) {
        let event_type = event.event_type.clone();
        let session_id = event.session_id.clone();

        {
            let mut events = self.events.write().await;
            if events.len() >= self.max_events {
                events.remove(0);
            }
            events.push(event.clone());
        }

        {
            let mut stats = self.stats.write().await;
            *stats.entry(event_type).or_insert(0) += 1;
        }

        if let Some(sid) = session_id {
            let mut session_stats = self.session_stats.write().await;
            *session_stats.entry(sid).or_insert(0) += 1;
        }

        match event.severity {
            AuditSeverity::Critical => {
                error!("Audit (CRITICAL): {:?}", event);
            }
            AuditSeverity::Error => {
                error!("Audit: {:?}", event);
            }
            AuditSeverity::Warning => {
                warn!("Audit: {:?}", event);
            }
            AuditSeverity::Info => {
                info!("Audit: {:?}", event);
            }
            AuditSeverity::Debug => {
                debug!("Audit: {:?}", event);
            }
        }
    }

    pub async fn log_input(&self, session_id: &str, input: &str) {
        let event = AuditEvent::new(AuditEventType::InputReceived, AuditSeverity::Debug)
            .with_session(session_id)
            .with_input(input);
        self.log(event).await;
    }

    pub async fn log_filtered(&self, session_id: &str, input: &str, reason: &str) {
        let event = AuditEvent::new(AuditEventType::InputFiltered, AuditSeverity::Warning)
            .with_session(session_id)
            .with_input(input)
            .with_metadata("reason", reason);
        self.log(event).await;
    }

    pub async fn log_classification(
        &self,
        session_id: &str,
        category: &PromptCategory,
        risk_score: f32,
    ) {
        let event = AuditEvent::new(AuditEventType::LlmClassified, AuditSeverity::Info)
            .with_session(session_id)
            .with_metadata("category", &format!("{:?}", category))
            .with_metadata("risk_score", &risk_score.to_string());
        self.log(event).await;
    }

    pub async fn log_validation(&self, session_id: &str, sensitive_count: usize, blocked: bool) {
        let severity = if blocked {
            AuditSeverity::Warning
        } else if sensitive_count > 0 {
            AuditSeverity::Info
        } else {
            AuditSeverity::Debug
        };

        let event = AuditEvent::new(AuditEventType::OutputValidated, severity)
            .with_session(session_id)
            .with_metadata("sensitive_count", &sensitive_count.to_string())
            .with_metadata("blocked", &blocked.to_string());
        self.log(event).await;
    }

    pub async fn log_tool_execution(
        &self,
        session_id: &str,
        tool_id: &str,
        action: &str,
        target: &str,
        result: &str,
        duration_ms: u64,
    ) {
        let event = AuditEvent::new(AuditEventType::ToolExecuted, AuditSeverity::Info)
            .with_session(session_id)
            .with_tool(tool_id)
            .with_action(action)
            .with_target(target)
            .with_result(result)
            .with_duration(duration_ms);
        self.log(event).await;
    }

    pub async fn log_permission(
        &self,
        session_id: &str,
        tool_id: &str,
        action: &str,
        granted: bool,
    ) {
        let severity = if granted {
            AuditSeverity::Debug
        } else {
            AuditSeverity::Warning
        };

        let event = AuditEvent::new(AuditEventType::PermissionChecked, severity)
            .with_session(session_id)
            .with_tool(tool_id)
            .with_action(action)
            .with_result(if granted { "granted" } else { "denied" });
        self.log(event).await;
    }

    pub async fn log_network(&self, session_id: &str, host: &str, port: u16, allowed: bool) {
        let event_type = if allowed {
            AuditEventType::NetworkAllowed
        } else {
            AuditEventType::NetworkDenied
        };
        let severity = if allowed {
            AuditSeverity::Debug
        } else {
            AuditSeverity::Warning
        };

        let event = AuditEvent::new(event_type, severity)
            .with_session(session_id)
            .with_target(&format!("{}:{}", host, port))
            .with_result(if allowed { "allowed" } else { "denied" });
        self.log(event).await;
    }

    pub async fn log_error(&self, session_id: &str, error: &str) {
        let event = AuditEvent::new(AuditEventType::Error, AuditSeverity::Error)
            .with_session(session_id)
            .with_result(error);
        self.log(event).await;
    }

    pub async fn get_events(
        &self,
        session_id: Option<&str>,
        limit: Option<usize>,
    ) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        let limit = limit.unwrap_or(100);

        if let Some(sid) = session_id {
            events
                .iter()
                .filter(|e| e.session_id.as_deref() == Some(sid))
                .rev()
                .take(limit)
                .cloned()
                .collect()
        } else {
            events.iter().rev().take(limit).cloned().collect()
        }
    }

    pub async fn get_stats(&self) -> HashMap<AuditEventType, u32> {
        let stats = self.stats.read().await;
        stats.clone()
    }

    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();

        let mut stats = self.stats.write().await;
        stats.clear();

        let mut session_stats = self.session_stats.write().await;
        session_stats.clear();
    }
}
