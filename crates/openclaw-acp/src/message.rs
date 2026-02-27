//! ACP Message Types
//!
//! Request, Response, and Event message types.

use serde::{Deserialize, Serialize};

/// ACP Request - request from one agent to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpRequest {
    /// Action name
    pub action: String,

    /// Request parameters
    pub params: serde_json::Value,

    /// Callback ID (for async responses)
    pub callback_id: Option<String>,

    /// Request timeout (seconds)
    pub timeout: Option<u64>,
}

impl AcpRequest {
    /// Create a new request
    pub fn new(action: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            action: action.into(),
            params,
            callback_id: None,
            timeout: None,
        }
    }

    /// Create a request with callback
    pub fn with_callback(mut self, callback_id: impl Into<String>) -> Self {
        self.callback_id = Some(callback_id.into());
        self
    }

    /// Create a request with timeout
    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// ACP Response - response to a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpResponse {
    /// Original request ID
    pub request_id: String,

    /// Success flag
    pub success: bool,

    /// Response data
    pub data: Option<serde_json::Value>,

    /// Error message (if failed)
    pub error: Option<String>,
}

impl AcpResponse {
    /// Create a successful response
    pub fn success(request_id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            request_id: request_id.into(),
            success: true,
            data: Some(data),
            error: None,
        }
    }

    /// Create a failed response
    pub fn error(request_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            success: false,
            data: None,
            error: Some(error.into()),
        }
    }
}

/// ACP Event - event notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpEvent {
    /// Event type
    pub event_type: String,

    /// Event data
    pub data: serde_json::Value,

    /// Source (channel, agent, etc.)
    pub source: Option<String>,
}

impl AcpEvent {
    /// Create a new event
    pub fn new(event_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            event_type: event_type.into(),
            data,
            source: None,
        }
    }

    /// Create an event with source
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Common event types
pub mod events {
    use super::*;

    /// Message received event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MessageReceived {
        pub message_id: String,
        pub chat_id: String,
        pub user_id: String,
        pub content: String,
        pub mentions: Vec<String>,
    }

    /// Agent joined event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentJoined {
        pub agent_id: String,
        pub agent_name: String,
        pub chat_id: String,
    }

    /// Agent left event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentLeft {
        pub agent_id: String,
        pub chat_id: String,
    }

    /// Task completed event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TaskCompleted {
        pub task_id: String,
        pub agent_id: String,
        pub result: serde_json::Value,
    }

    /// Context updated event
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ContextUpdated {
        pub context_id: String,
        pub updated_by: String,
        pub changes: serde_json::Value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let request = AcpRequest::new("execute", serde_json::json!({ "code": "print('hello')" }));
        assert_eq!(request.action, "execute");
    }

    #[test]
    fn test_response_creation() {
        let response = AcpResponse::success("req-123", serde_json::json!({ "result": "ok" }));
        assert!(response.success);
        assert_eq!(response.request_id, "req-123");
    }
}
