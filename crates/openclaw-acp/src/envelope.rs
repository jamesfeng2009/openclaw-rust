//! ACP Message Envelope
//!
//! The envelope is the top-level structure for all ACP messages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Envelope type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeType {
    Request,
    Response,
    Event,
    Heartbeat,
}

/// ACP Envelope - top-level message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpEnvelope {
    /// Protocol version (semver)
    pub version: String,

    /// Unique message ID (UUID)
    pub msg_id: String,

    /// Timestamp (ISO8601)
    pub timestamp: DateTime<Utc>,

    /// Envelope type
    pub envelope_type: EnvelopeType,

    /// Sender Agent ID
    pub from: String,

    /// Receiver Agent ID (optional, for broadcast)
    pub to: Option<String>,

    /// Conversation/Context ID
    pub conversation_id: Option<String>,

    /// Message payload
    pub payload: serde_json::Value,

    /// Extensions (for future extension)
    #[serde(default)]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl AcpEnvelope {
    /// Create a new envelope
    pub fn new(
        envelope_type: EnvelopeType,
        from: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            version: "1.0.0".to_string(),
            msg_id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            envelope_type,
            from,
            to: None,
            conversation_id: None,
            payload,
            extensions: HashMap::new(),
        }
    }

    /// Create a request envelope
    pub fn request(from: String, payload: serde_json::Value) -> Self {
        Self::new(EnvelopeType::Request, from, payload)
    }

    /// Create a response envelope
    pub fn response(from: String, to: String, payload: serde_json::Value) -> Self {
        let mut envelope = Self::new(EnvelopeType::Response, from, payload);
        envelope.to = Some(to);
        envelope
    }

    /// Create an event envelope
    pub fn event(from: String, payload: serde_json::Value) -> Self {
        Self::new(EnvelopeType::Event, from, payload)
    }

    /// Create a heartbeat envelope
    pub fn heartbeat(from: String) -> Self {
        Self::new(EnvelopeType::Heartbeat, from, serde_json::json!({ "type": "heartbeat" }))
    }

    /// Set conversation ID
    pub fn with_conversation(mut self, conversation_id: String) -> Self {
        self.conversation_id = Some(conversation_id);
        self
    }

    /// Set receiver
    pub fn with_receiver(mut self, to: String) -> Self {
        self.to = Some(to);
        self
    }

    /// Add extension
    pub fn with_extension(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extensions.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let envelope = AcpEnvelope::request(
            "openclaw".to_string(),
            serde_json::json!({ "action": "test" }),
        );

        assert_eq!(envelope.version, "1.0.0");
        assert_eq!(envelope.envelope_type, EnvelopeType::Request);
        assert_eq!(envelope.from, "openclaw");
    }

    #[test]
    fn test_envelope_serialization() {
        let envelope = AcpEnvelope::request(
            "openclaw".to_string(),
            serde_json::json!({ "action": "test" }),
        );

        let json = serde_json::to_string(&envelope).unwrap();
        let decoded: AcpEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.from, "openclaw");
    }
}
