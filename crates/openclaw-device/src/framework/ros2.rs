//! ROS2 接口
//!
//! ROS2 (Robot Operating System 2) 集成接口

use crate::framework::{FrameworkConfig, FrameworkError, FrameworkResult};
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
    #[error("Service error: {0}")]
    ServiceError(String),
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

#[cfg(feature = "ros2_mock")]
pub mod mock_impl {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::sync::mpsc::Sender;
    use async_trait::async_trait;

    pub struct MockRos2Node {
        name: String,
        namespace: String,
        connected: Arc<Mutex<bool>>,
        topics: Arc<Mutex<HashMap<String, (String, Sender<Vec<u8>>)>>>,
        services: Arc<Mutex<HashMap<String, String>>>,
        actions: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockRos2Node {
        pub fn new(name: &str, namespace: &str) -> Self {
            Self {
                name: name.to_string(),
                namespace: namespace.to_string(),
                connected: Arc::new(Mutex::new(false)),
                topics: Arc::new(Mutex::new(HashMap::new())),
                services: Arc::new(Mutex::new(HashMap::new())),
                actions: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl Ros2Client for MockRos2Node {
        fn name(&self) -> &str {
            "MockRos2Node"
        }

        fn supported_platforms(&self) -> &[Platform] {
            &[
                Platform::LinuxDesktop,
                Platform::LinuxServer,
                Platform::LinuxEmbedded,
                Platform::Windows,
                Platform::MacOSIntel,
                Platform::MacOSAppleSilicon,
            ]
        }

        fn is_connected(&self) -> bool {
            *self.connected.lock().unwrap()
        }

        async fn connect(&self, _config: &FrameworkConfig) -> FrameworkResult<()> {
            let mut conn = self.connected.lock().unwrap();
            *conn = true;
            Ok(())
        }

        async fn disconnect(&self) -> FrameworkResult<()> {
            let mut conn = self.connected.lock().unwrap();
            *conn = false;
            Ok(())
        }

        async fn health_check(&self) -> FrameworkResult<bool> {
            Ok(self.is_connected())
        }

        fn node_name(&self) -> &str {
            &self.name
        }

        fn namespace(&self) -> &str {
            &self.namespace
        }

        fn list_topics(&self) -> Ros2Result<Vec<Ros2TopicInfo>> {
            let topics = self.topics.lock().unwrap();
            Ok(topics
                .iter()
                .map(|(name, (msg_type, _))| Ros2TopicInfo {
                    name: name.clone(),
                    msg_type: msg_type.clone(),
                    is_publisher: true,
                    is_subscriber: true,
                })
                .collect())
        }

        fn list_services(&self) -> Ros2Result<Vec<Ros2ServiceInfo>> {
            let services = self.services.lock().unwrap();
            Ok(services
                .iter()
                .map(|(name, srv_type)| Ros2ServiceInfo {
                    name: name.clone(),
                    srv_type: srv_type.clone(),
                })
                .collect())
        }

        async fn get_topic(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Topic>> {
            let topics = self.topics.lock().unwrap();
            if topics.contains_key(name) {
                Ok(Arc::new(MockTopic::new(
                    name.to_string(),
                    topics.get(name).unwrap().0.clone(),
                )))
            } else {
                Err(Ros2Error::TopicNotFound(name.to_string()))
                    .map_err(|e| FrameworkError::MessageError(e.to_string()))
            }
        }

        async fn get_service(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Service>> {
            let services = self.services.lock().unwrap();
            if services.contains_key(name) {
                Ok(Arc::new(MockService::new(
                    name.to_string(),
                    services.get(name).unwrap().clone(),
                )))
            } else {
                Err(Ros2Error::ServiceNotFound(name.to_string()))
                    .map_err(|e| FrameworkError::MessageError(e.to_string()))
            }
        }

        async fn get_action(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Action>> {
            let actions = self.actions.lock().unwrap();
            if actions.contains_key(name) {
                Ok(Arc::new(MockAction::new(
                    name.to_string(),
                    actions.get(name).unwrap().clone(),
                )))
            } else {
                Err(Ros2Error::ActionNotFound(name.to_string()))
                    .map_err(|e| FrameworkError::MessageError(e.to_string()))
            }
        }

        async fn create_publisher(
            &self,
            topic: &str,
            msg_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Topic>> {
            let (tx, _rx) = std::sync::mpsc::channel();
            let mut topics = self.topics.lock().unwrap();
            topics.insert(topic.to_string(), (msg_type.to_string(), tx));
            Ok(Arc::new(MockTopic::new(topic.to_string(), msg_type.to_string())))
        }

        async fn create_subscriber(
            &self,
            topic: &str,
            msg_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Topic>> {
            let (tx, _rx) = std::sync::mpsc::channel();
            let mut topics = self.topics.lock().unwrap();
            topics.insert(topic.to_string(), (msg_type.to_string(), tx));
            Ok(Arc::new(MockTopic::new(topic.to_string(), msg_type.to_string())))
        }

        async fn create_service(&self, name: &str, srv_type: &str) -> Ros2Result<Arc<dyn Ros2Service>> {
            let mut services = self.services.lock().unwrap();
            services.insert(name.to_string(), srv_type.to_string());
            Ok(Arc::new(MockService::new(name.to_string(), srv_type.to_string())))
        }

        async fn create_action_client(
            &self,
            name: &str,
            action_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Action>> {
            let mut actions = self.actions.lock().unwrap();
            actions.insert(name.to_string(), action_type.to_string());
            Ok(Arc::new(MockAction::new(name.to_string(), action_type.to_string())))
        }
    }

    struct MockTopic {
        name: String,
        msg_type: String,
        callback: Arc<Mutex<Option<Ros2SubscriberCallback>>>,
    }

    impl MockTopic {
        fn new(name: String, msg_type: String) -> Self {
            Self {
                name,
                msg_type,
                callback: Arc::new(Mutex::new(None)),
            }
        }
    }

    #[async_trait]
    impl Ros2Topic for MockTopic {
        fn name(&self) -> &str {
            &self.name
        }

        fn msg_type(&self) -> &str {
            &self.msg_type
        }

        async fn publish(&self, _data: &[u8]) -> Ros2Result<()> {
            Ok(())
        }

        async fn subscribe(&self) -> Ros2Result<Receiver<Vec<u8>>> {
            let (_tx, rx) = std::sync::mpsc::channel();
            Ok(rx)
        }

        fn set_callback(&self, callback: Ros2SubscriberCallback) {
            let mut cb = self.callback.lock().unwrap();
            *cb = Some(callback);
        }
    }

    struct MockService {
        name: String,
        srv_type: String,
    }

    impl MockService {
        fn new(name: String, srv_type: String) -> Self {
            Self { name, srv_type }
        }
    }

    #[async_trait]
    impl Ros2Service for MockService {
        fn name(&self) -> &str {
            &self.name
        }

        fn srv_type(&self) -> &str {
            &self.srv_type
        }

        async fn call(&self, _request: &[u8]) -> Ros2Result<Vec<u8>> {
            Ok(vec![])
        }
    }

    struct MockAction {
        name: String,
        action_type: String,
    }

    impl MockAction {
        fn new(name: String, action_type: String) -> Self {
            Self { name, action_type }
        }
    }

    #[async_trait]
    impl Ros2Action for MockAction {
        fn name(&self) -> &str {
            &self.name
        }

        fn action_type(&self) -> &str {
            &self.action_type
        }

        async fn send_goal(&self, _goal: &[u8]) -> Ros2Result<String> {
            Ok("mock_goal_id".to_string())
        }

        async fn get_result(&self, _goal_id: &str) -> Ros2Result<Vec<u8>> {
            Ok(vec![])
        }

        async fn cancel_goal(&self, _goal_id: &str) -> Ros2Result<()> {
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_ros2_node_creation() {
            let node = MockRos2Node::new("/test_node", "/ns");
            assert_eq!(node.node_name(), "/test_node");
            assert_eq!(node.namespace(), "/ns");
        }

        #[test]
        fn test_mock_ros2_connect() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let node = MockRos2Node::new("/test_node", "/ns");
            
            rt.block_on(async {
                node.connect(&FrameworkConfig::default()).await.unwrap();
                assert!(node.is_connected());
                node.disconnect().await.unwrap();
                assert!(!node.is_connected());
            });
        }

        #[test]
        fn test_mock_ros2_publisher() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let node = MockRos2Node::new("/test_node", "/ns");
            
            rt.block_on(async {
                node.connect(&FrameworkConfig::default()).await.unwrap();
                let publisher = node
                    .create_publisher("/chatter", "std_msgs/String")
                    .await
                    .unwrap();
                assert_eq!(publisher.name(), "/chatter");
                assert_eq!(publisher.msg_type(), "std_msgs/String");
                publisher.publish(b"Hello").await.unwrap();
            });
        }

        #[test]
        fn test_mock_ros2_service() {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let node = MockRos2Node::new("/test_node", "/ns");
            
            rt.block_on(async {
                node.connect(&FrameworkConfig::default()).await.unwrap();
                let _service = node
                    .create_service("/add_two_ints", "example_interfaces/AddTwoInts")
                    .await
                    .unwrap();
                let services = node.list_services().unwrap();
                assert!(services.iter().any(|s| s.name == "/add_two_ints"));
            });
        }
    }
}

#[cfg(feature = "ros2_cli")]
pub mod cli_impl {
    use super::*;
    use std::collections::HashMap;
    use std::process::Command;
    use std::sync::{Arc, Mutex};
    use async_trait::async_trait;

    pub struct CliRos2Node {
        name: String,
        namespace: String,
        connected: Arc<Mutex<bool>>,
        workspace: Option<String>,
    }

    impl CliRos2Node {
        pub fn new(name: &str, namespace: &str, workspace: Option<&str>) -> Self {
            Self {
                name: name.to_string(),
                namespace: namespace.to_string(),
                connected: Arc::new(Mutex::new(false)),
                workspace: workspace.map(|s| s.to_string()),
            }
        }

        fn run_ros2_command(&self, args: &[&str]) -> FrameworkResult<String> {
            let output = Command::new("ros2")
                .args(args)
                .output()
                .map_err(|e| FrameworkError::ServiceError(e.to_string()))?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(FrameworkError::ServiceError(stderr.to_string()))
            }
        }

        fn topic_name(&self, topic: &str) -> String {
            if topic.starts_with('/') {
                topic.to_string()
            } else {
                format!("{}/{}", self.namespace, topic)
            }
        }

        fn service_name(&self, service: &str) -> String {
            if service.starts_with('/') {
                service.to_string()
            } else {
                format!("{}/{}", self.namespace, service)
            }
        }
    }

    #[async_trait]
    impl Ros2Client for CliRos2Node {
        fn name(&self) -> &str {
            "CliRos2Node"
        }

        fn supported_platforms(&self) -> &[Platform] {
            &[
                Platform::LinuxDesktop,
                Platform::LinuxServer,
                Platform::LinuxEmbedded,
                Platform::Windows,
                Platform::MacOSIntel,
                Platform::MacOSAppleSilicon,
            ]
        }

        fn is_connected(&self) -> bool {
            *self.connected.lock().unwrap()
        }

        async fn connect(&self, _config: &FrameworkConfig) -> FrameworkResult<()> {
            let result = self.run_ros2_command(&["topic", "list"]);
            match result {
                Ok(_) => {
                    let mut conn = self.connected.lock().unwrap();
                    *conn = true;
                    Ok(())
                }
                Err(_) => Err(FrameworkError::ServiceError("ROS2 not installed".to_string())),
            }
        }

        async fn disconnect(&self) -> FrameworkResult<()> {
            let mut conn = self.connected.lock().unwrap();
            *conn = false;
            Ok(())
        }

        async fn health_check(&self) -> FrameworkResult<bool> {
            Ok(self.is_connected())
        }

        fn node_name(&self) -> &str {
            &self.name
        }

        fn namespace(&self) -> &str {
            &self.namespace
        }

        fn list_topics(&self) -> Ros2Result<Vec<Ros2TopicInfo>> {
            let output = self.run_ros2_command(&["topic", "list"])?;
            let topics: Vec<Ros2TopicInfo> = output
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let name = parts.get(0).unwrap_or(&"");
                    let msg_type = parts.get(1).unwrap_or(&"");
                    Ros2TopicInfo {
                        name: name.to_string(),
                        msg_type: msg_type.to_string(),
                        is_publisher: true,
                        is_subscriber: true,
                    }
                })
                .collect();
            Ok(topics)
        }

        fn list_services(&self) -> Ros2Result<Vec<Ros2ServiceInfo>> {
            let output = self.run_ros2_command(&["service", "list"])?;
            let services: Vec<Ros2ServiceInfo> = output
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let name = parts.get(0).unwrap_or(&"");
                    let srv_type = parts.get(1).unwrap_or(&"");
                    Ros2ServiceInfo {
                        name: name.to_string(),
                        srv_type: srv_type.to_string(),
                    }
                })
                .collect();
            Ok(services)
        }

        async fn get_topic(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Topic>> {
            Ok(Arc::new(CliTopic::new(
                self.topic_name(name),
                "unknown".to_string(),
            )))
        }

        async fn get_service(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Service>> {
            Ok(Arc::new(CliService::new(
                self.service_name(name),
                "unknown".to_string(),
            )))
        }

        async fn get_action(&self, name: &str) -> Ros2Result<Arc<dyn Ros2Action>> {
            Ok(Arc::new(CliAction::new(
                name.to_string(),
                "unknown".to_string(),
            )))
        }

        async fn create_publisher(
            &self,
            topic: &str,
            msg_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Topic>> {
            let full_topic = self.topic_name(topic);
            Ok(Arc::new(CliTopic::new(full_topic, msg_type.to_string())))
        }

        async fn create_subscriber(
            &self,
            topic: &str,
            msg_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Topic>> {
            let full_topic = self.topic_name(topic);
            Ok(Arc::new(CliTopic::new(full_topic, msg_type.to_string())))
        }

        async fn create_service(&self, name: &str, srv_type: &str) -> Ros2Result<Arc<dyn Ros2Service>> {
            let full_name = self.service_name(name);
            Ok(Arc::new(CliService::new(full_name, srv_type.to_string())))
        }

        async fn create_action_client(
            &self,
            name: &str,
            action_type: &str,
        ) -> Ros2Result<Arc<dyn Ros2Action>> {
            Ok(Arc::new(CliAction::new(name.to_string(), action_type.to_string())))
        }
    }

    struct CliTopic {
        name: String,
        msg_type: String,
        callback: Arc<Mutex<Option<Ros2SubscriberCallback>>>,
    }

    impl CliTopic {
        fn new(name: String, msg_type: String) -> Self {
            Self {
                name,
                msg_type,
                callback: Arc::new(Mutex::new(None)),
            }
        }
    }

    #[async_trait]
    impl Ros2Topic for CliTopic {
        fn name(&self) -> &str {
            &self.name
        }

        fn msg_type(&self) -> &str {
            &self.msg_type
        }

        async fn publish(&self, data: &[u8]) -> Ros2Result<()> {
            let output = Command::new("ros2")
                .args(&["topic", "pub", &self.name, &self.msg_type])
                .arg(format!("{{data: {}}}", String::from_utf8_lossy(data)))
                .output()
                .map_err(|e| FrameworkError::MessageError(e.to_string()))?;

            if output.status.success() {
                Ok(())
            } else {
                Err(FrameworkError::MessageError("Command failed".to_string()))
            }
        }

        async fn subscribe(&self) -> Ros2Result<Receiver<Vec<u8>>> {
            let (tx, rx) = std::sync::mpsc::channel();
            let name = self.name.clone();
            
            std::thread::spawn(move || {
                let output = Command::new("ros2")
                    .args(&["topic", "echo", &name])
                    .output();
                
                if let Ok(output) = output {
                    if output.status.success() {
                        let _ = tx.send(output.stdout);
                    }
                }
            });

            Ok(rx)
        }

        fn set_callback(&self, callback: Ros2SubscriberCallback) {
            let mut cb = self.callback.lock().unwrap();
            *cb = Some(callback);
        }
    }

    struct CliService {
        name: String,
        srv_type: String,
    }

    impl CliService {
        fn new(name: String, srv_type: String) -> Self {
            Self { name, srv_type }
        }
    }

    #[async_trait]
    impl Ros2Service for CliService {
        fn name(&self) -> &str {
            &self.name
        }

        fn srv_type(&self) -> &str {
            &self.srv_type
        }

        async fn call(&self, request: &[u8]) -> Ros2Result<Vec<u8>> {
            let req_str = String::from_utf8_lossy(request).to_string();
            let output = Command::new("ros2")
                .args(&["service", "call", &self.name, &self.srv_type])
                .arg(&req_str)
                .output()
                .map_err(|e| FrameworkError::ServiceError(e.to_string()))?;

            if output.status.success() {
                Ok(output.stdout)
            } else {
                Err(FrameworkError::ServiceError("Command failed".to_string()))
            }
        }
    }

    struct CliAction {
        name: String,
        action_type: String,
    }

    impl CliAction {
        fn new(name: String, action_type: String) -> Self {
            Self { name, action_type }
        }
    }

    #[async_trait]
    impl Ros2Action for CliAction {
        fn name(&self) -> &str {
            &self.name
        }

        fn action_type(&self) -> &str {
            &self.action_type
        }

        async fn send_goal(&self, goal: &[u8]) -> Ros2Result<String> {
            let goal_str = String::from_utf8_lossy(goal).to_string();
            let output = Command::new("ros2")
                .args(&["action", "send_goal", &self.name, &self.action_type])
                .arg(&goal_str)
                .output()
                .map_err(|e| FrameworkError::ServiceError(e.to_string()))?;

            if output.status.success() {
                Ok("goal_id".to_string())
            } else {
                Err(FrameworkError::ServiceError("Command failed".to_string()))
            }
        }

        async fn get_result(&self, _goal_id: &str) -> Ros2Result<Vec<u8>> {
            Ok(vec![])
        }

        async fn cancel_goal(&self, goal_id: &str) -> Ros2Result<()> {
            let _ = goal_id;
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore = "requires ROS2 installation"]
        fn test_cli_ros2_node_creation() {
            let node = CliRos2Node::new("/test_node", "/ns", None);
            assert_eq!(node.node_name(), "/test_node");
            assert_eq!(node.namespace(), "/ns");
        }
    }
}
