use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DeviceNode {
    Camera(CameraNode),
    Screen(ScreenNode),
    Location(LocationNode),
    Notification(NotificationNode),
    System(SystemNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool,
    pub resolution: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: DeviceCategory,
    pub enabled: bool,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceCategory {
    Camera,
    Screen,
    Location,
    Notification,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatus {
    pub node_id: String,
    pub online: bool,
    pub last_update: i64,
    pub capabilities: Vec<DeviceCapability>,
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("设备未找到: {0}")]
    DeviceNotFound(String),

    #[error("设备不可用: {0}")]
    DeviceUnavailable(String),

    #[error("权限被拒绝: {0}")]
    PermissionDenied(String),

    #[error("操作失败: {0}")]
    OperationFailed(String),

    #[error("不支持的平台: {0}")]
    UnsupportedPlatform(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub success: bool,
    pub data: Option<String>,
    pub mime_type: String,
    pub timestamp: i64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResult {
    pub success: bool,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub accuracy: Option<f64>,
    pub timestamp: i64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationResult {
    pub success: bool,
    pub notification_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCommandResult {
    pub success: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

pub struct NodeManager {
    nodes: HashMap<String, DeviceNode>,
}

impl NodeManager {
    pub fn new() -> Self {
        let mut nodes = HashMap::new();
        
        nodes.insert(
            "camera".to_string(),
            DeviceNode::Camera(CameraNode {
                id: "camera".to_string(),
                name: "相机".to_string(),
                enabled: true,
                available: true,
            }),
        );
        
        nodes.insert(
            "screen".to_string(),
            DeviceNode::Screen(ScreenNode {
                id: "screen".to_string(),
                name: "屏幕录制".to_string(),
                enabled: true,
                available: true,
                resolution: None,
            }),
        );
        
        nodes.insert(
            "location".to_string(),
            DeviceNode::Location(LocationNode {
                id: "location".to_string(),
                name: "定位".to_string(),
                enabled: true,
                available: true,
                accuracy: None,
            }),
        );
        
        nodes.insert(
            "notification".to_string(),
            DeviceNode::Notification(NotificationNode {
                id: "notification".to_string(),
                name: "通知推送".to_string(),
                enabled: true,
                available: true,
            }),
        );
        
        nodes.insert(
            "system".to_string(),
            DeviceNode::System(SystemNode {
                id: "system".to_string(),
                name: "系统命令".to_string(),
                enabled: true,
                available: true,
            }),
        );

        Self { nodes }
    }

    pub fn get_nodes(&self) -> Vec<&DeviceNode> {
        self.nodes.values().collect()
    }

    pub fn get_node(&self, id: &str) -> Option<&DeviceNode> {
        self.nodes.get(id)
    }

    pub fn enable_node(&mut self, id: &str) -> Result<(), DeviceError> {
        match self.nodes.get_mut(id) {
            Some(DeviceNode::Camera(n)) => n.enabled = true,
            Some(DeviceNode::Screen(n)) => n.enabled = true,
            Some(DeviceNode::Location(n)) => n.enabled = true,
            Some(DeviceNode::Notification(n)) => n.enabled = true,
            Some(DeviceNode::System(n)) => n.enabled = true,
            None => return Err(DeviceError::DeviceNotFound(id.to_string())),
        }
        Ok(())
    }

    pub fn disable_node(&mut self, id: &str) -> Result<(), DeviceError> {
        match self.nodes.get_mut(id) {
            Some(DeviceNode::Camera(n)) => n.enabled = false,
            Some(DeviceNode::Screen(n)) => n.enabled = false,
            Some(DeviceNode::Location(n)) => n.enabled = false,
            Some(DeviceNode::Notification(n)) => n.enabled = false,
            Some(DeviceNode::System(n)) => n.enabled = false,
            None => return Err(DeviceError::DeviceNotFound(id.to_string())),
        }
        Ok(())
    }

    pub fn get_capabilities(&self) -> Vec<DeviceCapability> {
        let mut capabilities = Vec::new();
        
        for node in self.nodes.values() {
            match node {
                DeviceNode::Camera(n) => {
                    capabilities.push(DeviceCapability {
                        id: "camera.snap".to_string(),
                        name: "相机拍照".to_string(),
                        description: "使用设备相机拍摄照片".to_string(),
                        category: DeviceCategory::Camera,
                        enabled: n.enabled,
                        available: n.available,
                    });
                    capabilities.push(DeviceCapability {
                        id: "camera.clip".to_string(),
                        name: "相机录像".to_string(),
                        description: "使用设备相机录制视频".to_string(),
                        category: DeviceCategory::Camera,
                        enabled: n.enabled,
                        available: n.available,
                    });
                }
                DeviceNode::Screen(n) => {
                    capabilities.push(DeviceCapability {
                        id: "screen.record".to_string(),
                        name: "屏幕录制".to_string(),
                        description: "录制屏幕内容".to_string(),
                        category: DeviceCategory::Screen,
                        enabled: n.enabled,
                        available: n.available,
                    });
                    capabilities.push(DeviceCapability {
                        id: "screen.screenshot".to_string(),
                        name: "屏幕截图".to_string(),
                        description: "截取屏幕内容".to_string(),
                        category: DeviceCategory::Screen,
                        enabled: n.enabled,
                        available: n.available,
                    });
                }
                DeviceNode::Location(n) => {
                    capabilities.push(DeviceCapability {
                        id: "location.get".to_string(),
                        name: "获取定位".to_string(),
                        description: "获取设备当前地理位置".to_string(),
                        category: DeviceCategory::Location,
                        enabled: n.enabled,
                        available: n.available,
                    });
                }
                DeviceNode::Notification(n) => {
                    capabilities.push(DeviceCapability {
                        id: "notification.send".to_string(),
                        name: "发送通知".to_string(),
                        description: "向设备发送通知".to_string(),
                        category: DeviceCategory::Notification,
                        enabled: n.enabled,
                        available: n.available,
                    });
                }
                DeviceNode::System(n) => {
                    capabilities.push(DeviceCapability {
                        id: "system.run".to_string(),
                        name: "执行命令".to_string(),
                        description: "在设备上执行系统命令".to_string(),
                        category: DeviceCategory::System,
                        enabled: n.enabled,
                        available: n.available,
                    });
                    capabilities.push(DeviceCapability {
                        id: "system.notify".to_string(),
                        name: "系统通知".to_string(),
                        description: "发送系统级通知".to_string(),
                        category: DeviceCategory::System,
                        enabled: n.enabled,
                        available: n.available,
                    });
                }
            }
        }
        
        capabilities
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}
