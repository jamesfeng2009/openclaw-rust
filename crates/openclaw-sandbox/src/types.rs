//! 沙箱类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 沙箱 ID
pub type SandboxId = String;

/// 容器 ID
pub type ContainerId = String;

/// 沙箱配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 镜像名称
    pub image: String,
    /// 容器名称
    pub name: Option<String>,
    /// 环境变量
    pub env: HashMap<String, String>,
    /// 挂载点
    pub mounts: Vec<Mount>,
    /// 资源限制
    pub resources: ResourceLimits,
    /// 网络模式
    pub network: NetworkMode,
    /// 超时时间（秒）
    pub timeout_secs: u64,
    /// 是否自动删除
    pub auto_remove: bool,
    /// 工作目录
    pub work_dir: Option<String>,
    /// 命令
    pub cmd: Vec<String>,
    /// 安全配置
    pub security: SecurityConfig,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            image: "alpine:latest".to_string(),
            name: None,
            env: HashMap::new(),
            mounts: vec![],
            resources: ResourceLimits::default(),
            network: NetworkMode::None,
            timeout_secs: 300,
            auto_remove: true,
            work_dir: None,
            cmd: vec![],
            security: SecurityConfig::default(),
        }
    }
}

/// 挂载点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    /// 主机路径
    pub host_path: String,
    /// 容器路径
    pub container_path: String,
    /// 是否只读
    pub read_only: bool,
}

/// 资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU 限制（核心数）
    pub cpu_cores: Option<f64>,
    /// 内存限制（字节）
    pub memory_bytes: Option<u64>,
    /// 内存交换限制（字节）
    pub memory_swap_bytes: Option<u64>,
    /// 磁盘限制（字节）
    pub disk_bytes: Option<u64>,
    /// PIDs 限制
    pub pids_limit: Option<i64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_cores: Some(1.0),
            memory_bytes: Some(256 * 1024 * 1024), // 256MB
            memory_swap_bytes: None,
            disk_bytes: None,
            pids_limit: Some(100),
        }
    }
}

/// 网络模式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkMode {
    None,
    Bridge,
    Host,
    Custom(String),
}

/// 安全配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// 是否禁用网络
    pub no_new_privileges: bool,
    /// 只读根文件系统
    pub read_only_root_fs: bool,
    /// 用户命名空间
    pub user_namespace: Option<UserNamespace>,
    /// 能力列表
    pub capabilities: Capabilities,
    /// Seccomp 配置文件
    pub seccomp_profile: Option<String>,
    /// AppArmor 配置文件
    pub apparmor_profile: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            no_new_privileges: true,
            read_only_root_fs: true,
            user_namespace: None,
            capabilities: Capabilities::minimal(),
            seccomp_profile: None,
            apparmor_profile: None,
        }
    }
}

/// 用户命名空间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserNamespace {
    pub uid_map: Vec<IdMap>,
    pub gid_map: Vec<IdMap>,
}

/// ID 映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdMap {
    pub container_id: u32,
    pub host_id: u32,
    pub size: u32,
}

/// 能力配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// 添加的能力
    pub add: Vec<String>,
    /// 移除的能力
    pub drop: Vec<String>,
}

impl Capabilities {
    /// 最小能力集
    pub fn minimal() -> Self {
        Self {
            add: vec![],
            drop: vec!["ALL".to_string()],
        }
    }

    /// 添加能力
    pub fn with_capability(mut self, cap: &str) -> Self {
        self.add.push(cap.to_string());
        self
    }
}

/// 沙箱状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxStatus {
    pub id: SandboxId,
    pub container_id: Option<ContainerId>,
    pub state: SandboxState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub resource_usage: Option<ResourceUsage>,
}

/// 沙箱状态枚举
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SandboxState {
    Created,
    Running,
    Paused,
    Stopped,
    Error,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_time_nanos: u64,
    pub memory_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_secs: f64,
    pub resource_usage: Option<ResourceUsage>,
}

/// 沙箱事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SandboxEvent {
    Created { id: SandboxId },
    Started { id: SandboxId },
    Stopped { id: SandboxId, exit_code: i32 },
    Error { id: SandboxId, message: String },
    ResourceLimitExceeded { id: SandboxId, resource: String },
}
