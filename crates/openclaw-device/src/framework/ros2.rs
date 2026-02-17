//! ROS2 接口
//!
//! ROS2 (Robot Operating System 2) 集成接口

use crate::framework::{FrameworkConfig, FrameworkModule, FrameworkResult};
use crate::platform::Platform;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

pub type Ros2Result<T> = crate::framework::FrameworkResult<T>;

#[derive(Debug, thiserror::Error)]
pub enum Ros2Error {
    #[error("Node not found: {0}")]
    NodeNotFound(String),
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
    #[error("Action not found: {0}")]
    ActionNotFound(String),
    #[error("Publish failed: {0}")]
    PublishFailed(String),
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),
    #[error("Service call failed: {0}")]
    ServiceCallFailed(String),
    #[error("Action failed: {0}")]
    ActionFailed(String),
    #[error("ROS2 not installed")]
    NotInstalled,
    #[error("Workspace not found: {0}")]
    WorkspaceNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ros2TopicInfo {
    pub name: String,
    pub msg_type: String,
    pub is_publisher: bool,
    pub is_subscriber: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ros2ServiceInfo {
    pub name: String,
    pub srv_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ros2NodeInfo {
    pub name: String,
    pub namespace: String,
    pub topics: Vec<Ros2TopicInfo>,
    pub services: Vec<Ros2ServiceInfo>,
}

pub type Ros2SubscriberCallback = Box<dyn Fn(Vec<u8>) + Send + Sync>;

#[async_trait]
pub trait Ros2Topic: Send + Sync {
    fn name(&self) -> &str;

    fn msg_type(&self) -> &str;

    async fn publish(&self, data: &[u8]) -> Ros2Result<()>;

    async fn subscribe(&self) -> Ros2Result<Receiver<Vec<u8>>>;

    fn set_callback(&self, callback: Ros2SubscriberCallback);
}

#[async_trait]
pub trait Ros2Service: Send + Sync {
    fn name(&self) -> &str;

    fn srv_type(&self) -> &str;

    async fn call(&self, request: &[u8]) -> Ros2Result<Vec<u8>>;
}

#[async_trait]
pub trait Ros2Action: Send + Sync {
    fn name(&self) -> &str;

    fn action_type(&self) -> &str;

    async fn send_goal(&self, goal: &[u8]) -> Ros2Result<String>;

    async fn get_result(&self, goal_id: &str) -> Ros2Result<Vec<u8>>;

    async fn cancel_goal(&self, goal_id: &str) -> Ros2Result<()>;
}

#[async_trait]
pub trait Ros2Client: Send + Sync {
    fn name(&self) -> &str;

    fn supported_platforms(&self) -> &[Platform];

    fn is_connected(&self) -> bool;

    async fn connect(&self, config: &FrameworkConfig) -> FrameworkResult<()>;

    async fn disconnect(&self) -> FrameworkResult<()>;

    async fn health_check(&self) -> FrameworkResult<bool>;

    fn node_name(&self) -> &str;

    fn namespace(&self) -> &str;

    fn list_topics(&self) -> Ros2Result<Vec<Ros2TopicInfo>>;

    fn list_services(&self) -> Ros2Result<Vec<Ros2ServiceInfo>>;

    async fn get_topic(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Topic>>;

    async fn get_service(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Service>>;

    async fn get_action(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Action>>;

    async fn create_publisher(&self, topic: &str, msg_type: &str)
    -> Ros2Result<Arc<dyn Ros2Topic>>;

    async fn create_subscriber(
        &self,
        topic: &str,
        msg_type: &str,
    ) -> Ros2Result<Arc<dyn Ros2Topic>>;

    async fn create_service(&self, name: &str, srv_type: &str) -> Ros2Result<Arc<dyn Ros2Service>>;

    async fn create_action_client(
        &self,
        name: &str,
        action_type: &str,
    ) -> Ros2Result<Arc<dyn Ros2Action>>;
}
