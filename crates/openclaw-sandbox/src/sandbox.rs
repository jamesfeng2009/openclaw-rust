//! 沙箱管理器

use crate::docker::{DockerClient, DockerError};
use crate::permission::{Permission, PermissionError, PermissionManager, ResourceType};
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 沙箱管理器错误
#[derive(Debug, Error)]
pub enum SandboxManagerError {
    #[error("Docker 错误: {0}")]
    Docker(#[from] DockerError),

    #[error("权限错误: {0}")]
    Permission(#[from] PermissionError),

    #[error("沙箱不存在: {0}")]
    SandboxNotFound(SandboxId),

    #[error("沙箱正在运行")]
    SandboxRunning,

    #[error("超时")]
    Timeout,

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 沙箱管理器
pub struct SandboxManager {
    docker: Arc<DockerClient>,
    permissions: Arc<PermissionManager>,
    sandboxes: Arc<RwLock<HashMap<SandboxId, SandboxRecord>>>,
}

/// 沙箱记录
#[derive(Debug, Clone)]
struct SandboxRecord {
    config: SandboxConfig,
    owner_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl SandboxManager {
    /// 创建新的沙箱管理器
    pub async fn new() -> Result<Self, SandboxManagerError> {
        let docker = DockerClient::new().await?;
        let permissions = PermissionManager::new();

        // 初始化默认角色
        permissions.init_default_roles().await;

        Ok(Self {
            docker: Arc::new(docker),
            permissions: Arc::new(permissions),
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 使用现有组件创建
    pub fn with_components(docker: Arc<DockerClient>, permissions: Arc<PermissionManager>) -> Self {
        Self {
            docker,
            permissions,
            sandboxes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建沙箱
    pub async fn create_sandbox(
        &self,
        user_id: &str,
        config: SandboxConfig,
    ) -> Result<SandboxId, SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxCreate, None)
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("创建沙箱".to_string()),
            ));
        }

        // 创建沙箱
        let sandbox_id = self.docker.create_sandbox(config.clone()).await?;

        // 创建 ACL
        self.permissions
            .create_resource_acl(
                sandbox_id.clone(),
                ResourceType::Sandbox,
                user_id.to_string(),
            )
            .await?;

        // 记录沙箱
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.insert(
                sandbox_id.clone(),
                SandboxRecord {
                    config,
                    owner_id: user_id.to_string(),
                    created_at: chrono::Utc::now(),
                },
            );
        }

        info!("用户 {} 创建沙箱 {}", user_id, sandbox_id);
        Ok(sandbox_id)
    }

    /// 启动沙箱
    pub async fn start_sandbox(
        &self,
        user_id: &str,
        sandbox_id: &SandboxId,
    ) -> Result<(), SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxExecute, Some(sandbox_id))
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("启动沙箱".to_string()),
            ));
        }

        self.docker.start_sandbox(sandbox_id).await?;
        info!("用户 {} 启动沙箱 {}", user_id, sandbox_id);
        Ok(())
    }

    /// 执行命令并等待完成
    pub async fn execute(
        &self,
        user_id: &str,
        config: SandboxConfig,
    ) -> Result<ExecutionResult, SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxExecute, None)
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("执行沙箱".to_string()),
            ));
        }

        let start_time = std::time::Instant::now();

        // 创建并启动沙箱
        let sandbox_id = self.docker.create_sandbox(config.clone()).await?;

        // 设置超时
        let timeout_duration = Duration::from_secs(config.timeout_secs);

        // 启动
        self.docker.start_sandbox(&sandbox_id).await?;

        // 等待完成或超时
        let exit_code =
            tokio::time::timeout(timeout_duration, self.docker.wait_sandbox(&sandbox_id))
                .await
                .map_err(|_| SandboxManagerError::Timeout)?
                .map_err(|e| SandboxManagerError::Docker(e))?;

        // 获取日志
        let (stdout, stderr) = self.docker.get_logs(&sandbox_id).await?;

        // 清理
        if config.auto_remove {
            let _ = self.docker.remove_sandbox(&sandbox_id).await;
        }

        let duration = start_time.elapsed().as_secs_f64();

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            timed_out: false,
            duration_secs: duration,
            resource_usage: None,
        })
    }

    /// 停止沙箱
    pub async fn stop_sandbox(
        &self,
        user_id: &str,
        sandbox_id: &SandboxId,
    ) -> Result<(), SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxManage, Some(sandbox_id))
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("停止沙箱".to_string()),
            ));
        }

        self.docker.stop_sandbox(sandbox_id).await?;
        info!("用户 {} 停止沙箱 {}", user_id, sandbox_id);
        Ok(())
    }

    /// 删除沙箱
    pub async fn delete_sandbox(
        &self,
        user_id: &str,
        sandbox_id: &SandboxId,
    ) -> Result<(), SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxDelete, Some(sandbox_id))
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("删除沙箱".to_string()),
            ));
        }

        self.docker.remove_sandbox(sandbox_id).await?;

        // 移除记录
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.remove(sandbox_id);
        }

        info!("用户 {} 删除沙箱 {}", user_id, sandbox_id);
        Ok(())
    }

    /// 获取沙箱状态
    pub async fn get_status(
        &self,
        user_id: &str,
        sandbox_id: &SandboxId,
    ) -> Result<SandboxStatus, SandboxManagerError> {
        let user_id = user_id.to_string();

        // 检查权限
        let has_perm = self
            .permissions
            .check_permission(&user_id, &Permission::SandboxView, Some(sandbox_id))
            .await?;

        if !has_perm {
            return Err(SandboxManagerError::Permission(
                PermissionError::PermissionDenied("查看沙箱".to_string()),
            ));
        }

        let status = self.docker.get_status(sandbox_id).await?;
        Ok(status)
    }

    /// 列出用户的沙箱
    pub async fn list_user_sandboxes(&self, user_id: &str) -> Vec<SandboxStatus> {
        let sandboxes = self.sandboxes.read().await;
        let mut result = vec![];

        for (id, record) in sandboxes.iter() {
            if record.owner_id == user_id {
                if let Ok(status) = self.docker.get_status(id).await {
                    result.push(status);
                }
            }
        }

        result
    }

    /// 获取权限管理器
    pub fn permissions(&self) -> Arc<PermissionManager> {
        self.permissions.clone()
    }
}
