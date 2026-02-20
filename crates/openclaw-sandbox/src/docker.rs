//! Docker 容器管理

use crate::types::*;
use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Docker 错误
#[derive(Debug, Error)]
pub enum DockerError {
    #[error("Docker 连接失败: {0}")]
    ConnectionFailed(String),

    #[error("容器不存在: {0}")]
    ContainerNotFound(ContainerId),

    #[error("镜像不存在: {0}")]
    ImageNotFound(String),

    #[error("容器启动失败: {0}")]
    StartFailed(String),

    #[error("容器执行失败: {0}")]
    ExecutionFailed(String),

    #[error("超时")]
    Timeout,

    #[error("资源限制超出: {0}")]
    ResourceLimitExceeded(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// Docker 客户端包装
pub struct DockerClient {
    docker: Docker,
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

impl DockerClient {
    /// 创建新的 Docker 客户端
    pub async fn new() -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            docker,
            containers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 使用自定义 URL 连接
    pub async fn connect(uri: &str) -> Result<Self, DockerError> {
        let docker = Docker::connect_with_socket(uri, 120, bollard::API_DEFAULT_VERSION)
            .map_err(|e| DockerError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            docker,
            containers: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 拉取镜像
    pub async fn pull_image(&self, image: &str) -> Result<(), DockerError> {
        info!("拉取镜像: {}", image);

        let mut stream = self.docker.create_image(
            Some(CreateImageOptions {
                from_image: image,
                ..Default::default()
            }),
            None,
            None,
        );

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!("拉取状态: {}", status);
                    }
                }
                Err(e) => {
                    warn!("拉取镜像警告: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 创建沙箱
    pub async fn create_sandbox(&self, config: SandboxConfig) -> Result<SandboxId, DockerError> {
        let sandbox_id = Uuid::new_v4().to_string();

        // 确保镜像存在
        self.pull_image(&config.image).await?;

        // 构建容器配置
        let container_config = self.build_container_config(&config)?;

        // 创建容器
        let container_name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("openclaw-sandbox-{}", &sandbox_id[..8]));

        let container = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name: &container_name,
                    platform: None,
                }),
                container_config,
            )
            .await
            .map_err(|e| DockerError::StartFailed(e.to_string()))?;

        let container_id = container.id;

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
                        created_at: chrono::Utc::now(),
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
    pub async fn start_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), DockerError> {
        let container_id = {
            let containers = self.containers.read().await;
            containers
                .get(sandbox_id)
                .map(|c| c.container_id.clone())
                .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))?
        };

        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DockerError::StartFailed(e.to_string()))?;

        // 更新状态
        {
            let mut containers = self.containers.write().await;
            if let Some(info) = containers.get_mut(sandbox_id) {
                info.status.state = SandboxState::Running;
                info.status.started_at = Some(chrono::Utc::now());
            }
        }

        info!("启动沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 停止沙箱
    pub async fn stop_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), DockerError> {
        let container_id = {
            let containers = self.containers.read().await;
            containers
                .get(sandbox_id)
                .map(|c| c.container_id.clone())
                .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))?
        };

        self.docker
            .stop_container(&container_id, Some(StopContainerOptions { t: 10 }))
            .await
            .map_err(|e| DockerError::ExecutionFailed(e.to_string()))?;

        // 更新状态
        {
            let mut containers = self.containers.write().await;
            if let Some(info) = containers.get_mut(sandbox_id) {
                info.status.state = SandboxState::Stopped;
                info.status.finished_at = Some(chrono::Utc::now());
            }
        }

        info!("停止沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 等待沙箱完成
    pub async fn wait_sandbox(&self, sandbox_id: &SandboxId) -> Result<i32, DockerError> {
        let container_id = {
            let containers = self.containers.read().await;
            containers
                .get(sandbox_id)
                .map(|c| c.container_id.clone())
                .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))?
        };

        let result = self
            .docker
            .wait_container(&container_id, None::<WaitContainerOptions<String>>)
            .next()
            .await
            .ok_or_else(|| DockerError::ExecutionFailed("等待结果失败".to_string()))?
            .map_err(|e| DockerError::ExecutionFailed(e.to_string()))?;

        let exit_code = result.status_code as i32;

        // 更新状态
        {
            let mut containers = self.containers.write().await;
            if let Some(info) = containers.get_mut(sandbox_id) {
                info.status.state = SandboxState::Stopped;
                info.status.exit_code = Some(exit_code);
                info.status.finished_at = Some(chrono::Utc::now());
            }
        }

        Ok(exit_code)
    }

    /// 获取容器日志
    pub async fn get_logs(&self, sandbox_id: &SandboxId) -> Result<(String, String), DockerError> {
        let container_id = {
            let containers = self.containers.read().await;
            containers
                .get(sandbox_id)
                .map(|c| c.container_id.clone())
                .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))?
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut stream = self.docker.logs(
            &container_id,
            Some(LogsOptions::<String> {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => match output {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                },
                Err(e) => {
                    warn!("获取日志错误: {}", e);
                }
            }
        }

        Ok((stdout, stderr))
    }

    /// 删除沙箱
    pub async fn remove_sandbox(&self, sandbox_id: &SandboxId) -> Result<(), DockerError> {
        let container_id = {
            let containers = self.containers.read().await;
            containers
                .get(sandbox_id)
                .map(|c| c.container_id.clone())
                .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))?
        };

        self.docker
            .remove_container(
                &container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| DockerError::ExecutionFailed(e.to_string()))?;

        // 移除记录
        {
            let mut containers = self.containers.write().await;
            containers.remove(sandbox_id);
        }

        info!("删除沙箱: {}", sandbox_id);
        Ok(())
    }

    /// 获取沙箱状态
    pub async fn get_status(&self, sandbox_id: &SandboxId) -> Result<SandboxStatus, DockerError> {
        let containers = self.containers.read().await;
        containers
            .get(sandbox_id)
            .map(|c| c.status.clone())
            .ok_or_else(|| DockerError::ContainerNotFound(sandbox_id.clone()))
    }

    /// 列出所有沙箱
    pub async fn list_sandboxes(&self) -> Vec<SandboxStatus> {
        let containers = self.containers.read().await;
        containers.values().map(|c| c.status.clone()).collect()
    }

    /// 构建容器配置
    fn build_container_config(
        &self,
        config: &SandboxConfig,
    ) -> Result<Config<String>, DockerError> {
        // 环境变量
        let env: Vec<String> = config
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // 挂载点
        let mounts: Vec<bollard::models::Mount> = config
            .mounts
            .iter()
            .map(|m| bollard::models::Mount {
                target: Some(m.container_path.clone()),
                source: Some(m.host_path.clone()),
                read_only: Some(m.read_only),
                typ: Some(bollard::models::MountTypeEnum::BIND),
                ..Default::default()
            })
            .collect();

        // 资源限制
        let host_config = bollard::models::HostConfig {
            memory: config.resources.memory_bytes.map(|m| m as i64),
            cpu_period: Some(100000),
            cpu_quota: config.resources.cpu_cores.map(|c| (c * 100000.0) as i64),
            pids_limit: config.resources.pids_limit,
            network_mode: Some(match &config.network {
                NetworkMode::None => "none".to_string(),
                NetworkMode::Bridge => "bridge".to_string(),
                NetworkMode::Host => "host".to_string(),
                NetworkMode::Custom(n) => n.clone(),
            }),
            mounts: if mounts.is_empty() {
                None
            } else {
                Some(mounts)
            },
            auto_remove: Some(config.auto_remove),
            readonly_rootfs: Some(config.security.read_only_root_fs),
            cap_drop: Some(config.security.capabilities.drop.clone()),
            cap_add: if config.security.capabilities.add.is_empty() {
                None
            } else {
                Some(config.security.capabilities.add.clone())
            },
            security_opt: {
                let mut opts = vec!["no-new-privileges:true".to_string()];
                if let Some(ref profile) = config.security.seccomp_profile {
                    opts.push(format!("seccomp={}", profile));
                }
                if let Some(ref profile) = config.security.apparmor_profile {
                    opts.push(format!("apparmor={}", profile));
                }
                Some(opts)
            },
            ..Default::default()
        };

        Ok(Config {
            image: Some(config.image.clone()),
            env: if env.is_empty() { None } else { Some(env) },
            cmd: if config.cmd.is_empty() {
                None
            } else {
                Some(config.cmd.clone())
            },
            working_dir: config.work_dir.clone(),
            host_config: Some(host_config),
            ..Default::default()
        })
    }
}

impl Default for DockerClient {
    fn default() -> Self {
        // 同步创建默认客户端
        futures::executor::block_on(Self::new()).expect("无法连接 Docker")
    }
}
