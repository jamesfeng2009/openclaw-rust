//! MQTT 接口
//!
//! MQTT (Message Queuing Telemetry Transport) 协议接口

use crate::framework::{FrameworkConfig, FrameworkModule, FrameworkResult};
use crate::platform::Platform;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type MqttResult<T> = crate::framework::FrameworkResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum MqttError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Broker not found: {0}")]
    BrokerNotFound(String),
    #[error("Authentication failed")]
    AuthFailed,
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    #[error("Message too large: {0}")]
    MessageTooLarge(usize),
    #[error("Invalid topic: {0}")]
    InvalidTopic(String),
    #[error("QoS not supported: {0}")]
    QosNotSupported(u8),
    #[error("TLS error: {0}")]
    TlsError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttMessage {
    pub topic: String,
    pub payload: Vec<u8>,
    pub qos: MqttQos,
    pub retain: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl Default for MqttQos {
    fn default() -> Self {
        Self::AtMostOnce
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSubscription {
    pub topic: String,
    pub qos: MqttQos,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    pub broker: String,
    pub port: u16,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub use_tls: bool,
    pub keep_alive_secs: u16,
    pub clean_session: bool,
    pub last_will: Option<MqttLastWill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttLastWill {
    pub topic: String,
    pub message: Vec<u8>,
    pub qos: MqttQos,
    pub retain: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: "localhost".to_string(),
            port: 1883,
            client_id: None,
            username: None,
            password: None,
            use_tls: false,
            keep_alive_secs: 60,
            clean_session: true,
            last_will: None,
        }
    }
}

pub trait MqttClient: FrameworkModule {
    fn client_id(&self) -> &str;

    fn is_connected(&self) -> bool;

    fn subscribe(&self, topics: &[MqttSubscription]) -> MqttResult<()>;

    fn unsubscribe(&self, topics: &[String]) -> MqttResult<()>;

    fn publish(&self, message: MqttMessage) -> MqttResult<()>;

    fn set_message_callback<F>(&self, callback: F)
    where
        F: Fn(MqttMessage) + Send + Sync + 'static;

    fn pending_messages(&self) -> usize;
}
