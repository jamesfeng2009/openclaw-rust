//! Podman 容器管理
//!
//! Podman 是 Docker 的无守护进程替代方案：
//! - 无需 root 权限 (rootless mode)
//! - 兼容 Docker CLI
//! - 更安全的容器隔离
//! - 支持 Kubernetes YAML

use crate::types::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Podman 错误
#[derive(Debug, Error)]
pub enum PodmanError {
    #[error("Podman 未安装或不可用")]
    NotInstalled,

    #[error("容器不存在: {0}")]
    ContainerNotFound(ContainerId),

    #[error("镜像不存在: {0}")]
    ImageNotFound(String),

    #[error("容器启动失败: {0}")]
    StartFailed(String),

    #[error("容器执行失败: {0}")]
    ExecutionFailed(String),

    #[error("Pod 操作失败: {0}")]
    PodFailed(String),

    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("超时")]
    Timeout,

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("解析错误: {0}")]
    Parse(String),
}

/// Podman 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodmanConfig {
    /// 是否使用 rootless 模式
    pub rootless: bool,
    /// 默认网络
    pub default_network: String,
    /// 默认命名空间
    pub namespace: Option<String>,
    /// Podman 命令路径
    pub podman_path: String,
    /// 是否启用 Pod 支持
    pub enable_pods: bool,
    /// 日志驱动
    pub log_driver: String,
    /// 存储驱动
    pub storage_driver: Option<String>,
}

impl Default for PodmanConfig {
    fn default() -> Self {
        Self {
            rootless: true,
            default_network: "bridge".to_string(),
            namespace: None,
            podman_path: "podman".to_string(),
            enable_pods: true,
            log_driver: "journald".to_string(),
            storage_driver: None,
        }
    }
}

/// Podman 容器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodmanContainer {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub status: String,
    pub state: String,
    pub created: i64,
    pub ports: Vec<String>,
    pub labels: HashMap<String, String>,
}

/// Podman 镜像信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodmanImage {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: u64,
    pub created: i64,
}

/// Podman Pod 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodmanPod {
    pub id: String,
    pub name: String,
    pub status: String,
    pub num_containers: usize,
    pub containers: Vec<String>,
}

/// Podman 客户端
pub struct PodmanClient {
    config: PodmanConfig,
    containers: Arc<RwLock<HashMap<SandboxId, ContainerInfo>>>,
}

/// 容器信息
#[derive(Debug, Clone)]
struct ContainerInfo {
    container_id: ContainerId,
    #[allow(dead_code)]
    config: SandboxConfig,
    status: SandboxStatus,
}

impl PodmanClient {
    /// 创建新的 Podman 客户端
    pub async fn new(config: PodmanConfig) -> Result<Self, PodmanError> {
        // 检查 Podman 是否可用
        let client = Self {
            config,
            containers: Arc::new(RwLock::new(HashMap::new())),
        };

        client.check_available().await?;

        info!(
            "Podman 客户端已初始化 (rootless: {})",
            client.config.rootless
        );
        Ok(client)
    }

    /// 检查 Podman 是否可用
    async fn check_available(&self) -> Result<(), PodmanError> {
        let output = Command::new(&self.config.podman_path)
            .arg("--version")
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let version = String::from_utf8_lossy(&o.stdout);
                debug!("Podman 版本: {}", version.trim());
                Ok(())
            }
            Ok(_) => {
                // 尝试 rootless 模式
                if self.config.rootless {
                    warn!("Podman 可能需要 rootless 模式配置");
                }
                Err(PodmanError::NotInstalled)
            }
            Err(_) => Err(PodmanError::NotInstalled),
        }
    }

    /// 执行 podman 命令
    fn run_command(&self, args: &[&str]) -> Result<std::process::Output, PodmanError> {
        let mut cmd = Command::new(&self.config.podman_path);

        // 添加命名空间参数
        if let Some(ref ns) = self.config.namespace {
            cmd.args(["--namespace", ns]);
        }

        cmd.args(args);

        debug!("执行命令: podman {}", args.join(" "));

        cmd.output().map_err(PodmanError::Io)
    }

    /// 拉取镜像
    pub async fn pull_image(&self, image: &str) -> Result<(), PodmanError> {
        info!("拉取镜像: {}", image);

        let output = self.run_command(&["pull", image])?;

        if output.status.success() {
            info!("镜像拉取成功: {}", image);
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            error!("拉取镜像失败: {}", error);
            Err(PodmanError::ImageNotFound(image.to_string()))
        }
    }

    /// 创建沙箱
    pub async fn create_sandbox(&self, config: SandboxConfig) -> Result<SandboxId, PodmanError> {
        let sandbox_id = uuid::Uuid::new_v4().to_string();

        // 确保镜像存在
        self.pull_image(&config.image).await.ok();

        // 构建创建参数 - 使用 String 避免生命周期问题
        let mut args: Vec<String> = vec!["run".to_string(), "--detach".to_string()];

        // 添加容器名称
        let container_name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("openclaw-sandbox-{}", &sandbox_id[..8]));
        args.push("--name".to_string());
        args.push(container_name);

        // 资源限制
        if let Some(memory) = config.resources.memory_bytes {
            args.push("--memory".to_string());
            args.push(memory.to_string());
        }

        if let Some(cpus) = config.resources.cpu_cores {
            args.push("--cpus".to_string());
            args.push(cpus.to_string());
        }

        if let Some(pids) = config.resources.pids_limit {
            args.push("--pids-limit".to_string());
            args.push(pids.to_string());
        }

        // 网络配置
        match &config.network {
            NetworkMode::None => args.push("--network=none".to_string()),
            NetworkMode::Host => args.push("--network=host".to_string()),
            NetworkMode::Bridge => args.push("--network=bridge".to_string()),
            NetworkMode::Custom(n) => {
                args.push("--network".to_string());
                args.push(n.clone());
            }
        }

        // 挂载
        for mount in &config.mounts {
            let mount_str = format!(
                "{}:{}:{}",
                mount.host_path,
                mount.container_path,
                if mount.read_only { "ro" } else { "rw" }
            );
            args.push("--volume".to_string());
            args.push(mount_str);
        }

        // 安全配置
        if config.security.read_only_root_fs {
            args.push("--read-only".to_string());
        }

        for cap in &config.security.capabilities.drop {
            args.push("--cap-drop".to_string());
            args.push(cap.clone());
        }

        for cap in &config.security.capabilities.add {
            args.push("--cap-add".to_string());
            args.push(cap.clone());
        }

        if config.security.no_new_privileges {
            args.push("--security-opt=no-new-privileges".to_string());
        }

        // 环境变量
        for (key, value) in &config.env {
            args.push("--env".to_string());
            args.push(format!("{}={}", key, value));
        }

        // 工作目录
        if let Some(ref work_dir) = config.work_dir {
            args.push("--workdir".to_string());
            args.push(work_dir.clone());
        }

        // 镜像和命令
        args.push(config.image.clone());

        if !config.cmd.is_empty() {
            for cmd_arg in &config.cmd {
                args.push(cmd_arg.clone());
            }
        }

        let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let output = self.run_command(&args)?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::StartFailed(error.to_string()));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 记录容器信息
        {
            let mut containers = self.containers.write().await;
            containers.insert(
                sandbox_id.clone(),
                ContainerInfo {
                    container_id: container_id.clone(),
                    config: config.clone(),
                    status: SandboxStatus {
                        id: sandbox_id.clone(),
                        container_id: Some(container_id.clone()),
                        state: SandboxState::Created,
                        created_at: Utc::now(),
                        started_at: None,
                        finished_at: None,
                        exit_code: None,
                        resource_usage: None,
                    },
                },
            );
        }

        info!("创建沙箱: {} -> {}", sandbox_id, container_id);
        Ok(sandbox_id)
    }

    /// 启动沙箱
    pub async fn start_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), PodmanError> {
        let container_id = self.get_container_id(sandbox_id).await?;

        let output = self.run_command(&["start", &container_id])?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::StartFailed(error.to_string()));
        }

        // 更新状态
        {
            let mut containers = self.containers.write().await;
            if let Some(info) = containers.get_mut(sandbox_id) {
                info.status.state = SandboxState::Running;
                info.status.started_at = Some(Utc::now());
            }
        }

        info!("启动沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 停止沙箱
    pub async fn stop_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), PodmanError> {
        let container_id = self.get_container_id(sandbox_id).await?;

        let _output = self.run_command(&["stop", "-t", "10", &container_id])?;

        // 更新状态
        {
            let mut containers = self.containers.write().await;
            if let Some(info) = containers.get_mut(sandbox_id) {
                info.status.state = SandboxState::Stopped;
                info.status.finished_at = Some(Utc::now());
            }
        }

        info!("停止沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 删除沙箱
    pub async fn remove_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), PodmanError> {
        let container_id = self.get_container_id(sandbox_id).await?;

        let _output = self.run_command(&["rm", "--force", &container_id])?;

        // 移除记录
        {
            let mut containers = self.containers.write().await;
            containers.remove(sandbox_id);
        }

        info!("删除沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 获取日志
    pub async fn get_logs(&self, sandbox_id: &SandboxId) -> Result<(String, String), PodmanError> {
        let container_id = self.get_container_id(sandbox_id).await?;

        let output = self.run_command(&["logs", &container_id])?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr))
    }

    /// 获取状态
    pub async fn get_status(&self, sandbox_id: &SandboxId) -> Result<SandboxStatus, PodmanError> {
        let containers = self.containers.read().await;
        containers
            .get(sandbox_id)
            .map(|c| c.status.clone())
            .ok_or_else(|| PodmanError::ContainerNotFound(sandbox_id.clone()))
    }

    /// 列出所有沙箱
    pub async fn list_sandboxes(&self) -> Vec<SandboxStatus> {
        let containers = self.containers.read().await;
        containers.values().map(|c| c.status.clone()).collect()
    }

    /// 在容器中执行命令
    pub async fn exec(
        &self,
        sandbox_id: &SandboxId,
        cmd: &[String],
    ) -> Result<(String, String, i32), PodmanError> {
        let container_id = self.get_container_id(sandbox_id).await?;

        let mut args = vec!["exec", &container_id];
        for c in cmd {
            args.push(c);
        }

        let output = self.run_command(&args)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        Ok((stdout, stderr, exit_code))
    }

    /// 创建 Pod (容器组)
    pub async fn create_pod(
        &self,
        name: &str,
        port_mappings: Vec<(u16, u16)>,
    ) -> Result<String, PodmanError> {
        if !self.config.enable_pods {
            return Err(PodmanError::PodFailed("Pod 支持未启用".to_string()));
        }

        let mut args: Vec<String> = vec![
            "pod".to_string(),
            "create".to_string(),
            "--name".to_string(),
            name.to_string(),
        ];

        for (host_port, container_port) in port_mappings {
            args.push("-p".to_string());
            args.push(format!("{}:{}", host_port, container_port));
        }

        let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_command(&args)?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::PodFailed(error.to_string()));
        }

        let pod_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        info!("创建 Pod: {} -> {}", name, pod_id);
        Ok(pod_id)
    }

    /// 删除 Pod
    pub async fn remove_pod(&self, name: &str) -> Result<(), PodmanError> {
        let output = self.run_command(&["pod", "rm", "--force", name])?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(PodmanError::PodFailed(error.to_string()));
        }

        info!("删除 Pod: {}", name);
        Ok(())
    }

    /// 列出镜像
    pub async fn list_images(&self) -> Result<Vec<PodmanImage>, PodmanError> {
        let output = self.run_command(&["images", "--format", "json"])?;

        if !output.status.success() {
            return Err(PodmanError::Parse("获取镜像列表失败".to_string()));
        }

        let images: Vec<PodmanImage> = serde_json::from_slice(&output.stdout)
            .map_err(|e| PodmanError::Parse(e.to_string()))?;

        Ok(images)
    }

    /// 列出容器
    pub async fn list_containers(&self) -> Result<Vec<PodmanContainer>, PodmanError> {
        let output = self.run_command(&["ps", "-a", "--format", "json"])?;

        if !output.status.success() {
            return Err(PodmanError::Parse("获取容器列表失败".to_string()));
        }

        let containers: Vec<PodmanContainer> = serde_json::from_slice(&output.stdout)
            .map_err(|e| PodmanError::Parse(e.to_string()))?;

        Ok(containers)
    }

    /// 获取容器 ID
    async fn get_container_id(&self, sandbox_id: &SandboxId) -> Result<ContainerId, PodmanError> {
        let containers = self.containers.read().await;
        containers
            .get(sandbox_id)
            .map(|c| c.container_id.clone())
            .ok_or_else(|| PodmanError::ContainerNotFound(sandbox_id.clone()))
    }

    /// 检查 rootless 模式
    pub async fn is_rootless(&self) -> bool {
        let output = self.run_command(&["info", "--format", "json"]);

        match output {
            Ok(o) if o.status.success() => {
                if let Ok(info) = serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                    info["host"]["rootless"].as_bool().unwrap_or(false)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// 生成 Kubernetes YAML
    pub fn generate_kube_yaml(&self, sandbox_id: &SandboxId) -> Result<String, PodmanError> {
        // 简化实现，返回基本信息
        Ok(format!(
            r#"apiVersion: v1
kind: Pod
metadata:
  name: {}
spec:
  containers:
  - name: main
    image: placeholder
"#,
            sandbox_id
        ))
    }
}

/// 默认实现
impl Default for PodmanClient {
    fn default() -> Self {
        futures::executor::block_on(Self::new(PodmanConfig::default()))
            .expect("无法初始化 Podman 客户端")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_podman_config_default() {
        let config = PodmanConfig::default();
        assert!(config.rootless);
        assert!(config.enable_pods);
    }

    #[test]
    fn test_podman_image_deserialize() {
        let json =
            r#"{"id":"abc123","repository":"test","tag":"latest","size":100,"created":12345}"#;
        let image: PodmanImage = serde_json::from_str(json).unwrap();
        assert_eq!(image.id, "abc123");
    }
}
