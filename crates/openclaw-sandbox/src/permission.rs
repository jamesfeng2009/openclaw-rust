//! 权限管理系统

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// 权限错误
#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("权限不足: {0}")]
    PermissionDenied(String),

    #[error("角色不存在: {0}")]
    RoleNotFound(String),

    #[error("用户不存在: {0}")]
    UserNotFound(String),

    #[error("资源不存在: {0}")]
    ResourceNotFound(String),

    #[error("无效操作: {0}")]
    InvalidOperation(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 用户 ID
pub type UserId = String;

/// 角色 ID
pub type RoleId = String;

/// 资源 ID
pub type ResourceId = String;

/// 权限类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    /// 创建沙箱
    SandboxCreate,
    /// 执行沙箱
    SandboxExecute,
    /// 删除沙箱
    SandboxDelete,
    /// 查看沙箱
    SandboxView,
    /// 管理沙箱
    SandboxManage,

    /// 创建画布
    CanvasCreate,
    /// 编辑画布
    CanvasEdit,
    /// 删除画布
    CanvasDelete,
    /// 查看画布
    CanvasView,

    /// 浏览器控制
    BrowserControl,
    /// 浏览器截图
    BrowserScreenshot,
    /// 浏览器导航
    BrowserNavigate,

    /// 工具调用
    ToolCall,
    /// 工具管理
    ToolManage,

    /// 定时任务管理
    ScheduleManage,

    /// Webhook 管理
    WebhookManage,

    /// 系统管理
    SystemAdmin,
    /// 用户管理
    UserAdmin,
    /// 角色管理
    RoleAdmin,

    /// 自定义权限
    Custom(String),
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Permission::Custom(s) => write!(f, "custom:{}", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// 角色
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Role {
    pub fn new(name: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            permissions: HashSet::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<Permission>) -> Self {
        for p in permissions {
            self.permissions.insert(p);
        }
        self.updated_at = Utc::now();
        self
    }

    /// 创建管理员角色
    pub fn admin() -> Self {
        Self::new("admin".to_string(), "系统管理员".to_string()).with_permissions(vec![
            Permission::SandboxCreate,
            Permission::SandboxExecute,
            Permission::SandboxDelete,
            Permission::SandboxView,
            Permission::SandboxManage,
            Permission::CanvasCreate,
            Permission::CanvasEdit,
            Permission::CanvasDelete,
            Permission::CanvasView,
            Permission::BrowserControl,
            Permission::BrowserScreenshot,
            Permission::BrowserNavigate,
            Permission::ToolCall,
            Permission::ToolManage,
            Permission::ScheduleManage,
            Permission::WebhookManage,
            Permission::SystemAdmin,
            Permission::UserAdmin,
            Permission::RoleAdmin,
        ])
    }

    /// 创建普通用户角色
    pub fn user() -> Self {
        Self::new("user".to_string(), "普通用户".to_string()).with_permissions(vec![
            Permission::SandboxView,
            Permission::CanvasCreate,
            Permission::CanvasEdit,
            Permission::CanvasView,
            Permission::BrowserControl,
            Permission::BrowserScreenshot,
            Permission::ToolCall,
        ])
    }

    /// 创建访客角色
    pub fn guest() -> Self {
        Self::new("guest".to_string(), "访客".to_string())
            .with_permissions(vec![Permission::CanvasView, Permission::SandboxView])
    }
}

/// 用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub name: String,
    pub email: Option<String>,
    pub roles: HashSet<RoleId>,
    pub permissions: HashSet<Permission>,
    pub denied_permissions: HashSet<Permission>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

impl User {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            email: None,
            roles: HashSet::new(),
            permissions: HashSet::new(),
            denied_permissions: HashSet::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
        }
    }

    pub fn with_role(mut self, role_id: RoleId) -> Self {
        self.roles.insert(role_id);
        self.updated_at = Utc::now();
        self
    }

    pub fn with_permission(mut self, permission: Permission) -> Self {
        self.permissions.insert(permission);
        self.updated_at = Utc::now();
        self
    }

    pub fn deny_permission(mut self, permission: Permission) -> Self {
        self.denied_permissions.insert(permission);
        self.updated_at = Utc::now();
        self
    }
}

/// 资源 ACL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAcl {
    pub resource_id: ResourceId,
    pub resource_type: ResourceType,
    pub owner_id: UserId,
    pub public: bool,
    pub user_permissions: HashMap<UserId, HashSet<Permission>>,
    pub role_permissions: HashMap<RoleId, HashSet<Permission>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 资源类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    Sandbox,
    Canvas,
    Tool,
    Schedule,
    Webhook,
}

impl ResourceAcl {
    pub fn new(resource_id: ResourceId, resource_type: ResourceType, owner_id: UserId) -> Self {
        Self {
            resource_id,
            resource_type,
            owner_id,
            public: false,
            user_permissions: HashMap::new(),
            role_permissions: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn grant_user(&mut self, user_id: UserId, permissions: Vec<Permission>) {
        let entry = self.user_permissions.entry(user_id).or_default();
        for p in permissions {
            entry.insert(p);
        }
        self.updated_at = Utc::now();
    }

    pub fn grant_role(&mut self, role_id: RoleId, permissions: Vec<Permission>) {
        let entry = self.role_permissions.entry(role_id).or_default();
        for p in permissions {
            entry.insert(p);
        }
        self.updated_at = Utc::now();
    }

    pub fn revoke_user(&mut self, user_id: &UserId, permission: &Permission) {
        if let Some(perms) = self.user_permissions.get_mut(user_id) {
            perms.remove(permission);
        }
        self.updated_at = Utc::now();
    }
}

/// 权限管理器
pub struct PermissionManager {
    roles: Arc<RwLock<HashMap<RoleId, Role>>>,
    users: Arc<RwLock<HashMap<UserId, User>>>,
    acls: Arc<RwLock<HashMap<ResourceId, ResourceAcl>>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            acls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 初始化默认角色
    pub async fn init_default_roles(&self) {
        let mut roles = self.roles.write().await;

        let admin = Role::admin();
        roles.insert(admin.id.clone(), admin);

        let user = Role::user();
        roles.insert(user.id.clone(), user);

        let guest = Role::guest();
        roles.insert(guest.id.clone(), guest);

        info!("初始化默认角色完成");
    }

    /// 创建角色
    pub async fn create_role(&self, name: String, description: String) -> RoleId {
        let role = Role::new(name, description);
        let id = role.id.clone();

        let mut roles = self.roles.write().await;
        roles.insert(id.clone(), role);

        info!("创建角色: {}", id);
        id
    }

    /// 获取角色
    pub async fn get_role(&self, role_id: &RoleId) -> Option<Role> {
        let roles = self.roles.read().await;
        roles.get(role_id).cloned()
    }

    /// 创建用户
    pub async fn create_user(&self, name: String) -> UserId {
        let user = User::new(name);
        let id = user.id.clone();

        let mut users = self.users.write().await;
        users.insert(id.clone(), user);

        info!("创建用户: {}", id);
        id
    }

    /// 获取用户
    pub async fn get_user(&self, user_id: &UserId) -> Option<User> {
        let users = self.users.read().await;
        users.get(user_id).cloned()
    }

    /// 为用户分配角色
    pub async fn assign_role(
        &self,
        user_id: &UserId,
        role_id: &RoleId,
    ) -> Result<(), PermissionError> {
        let mut users = self.users.write().await;
        let user = users
            .get_mut(user_id)
            .ok_or_else(|| PermissionError::UserNotFound(user_id.clone()))?;

        user.roles.insert(role_id.clone());
        user.updated_at = Utc::now();

        info!("为用户 {} 分配角色 {}", user_id, role_id);
        Ok(())
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        user_id: &UserId,
        permission: &Permission,
        resource_id: Option<&ResourceId>,
    ) -> Result<bool, PermissionError> {
        let users = self.users.read().await;
        let user = users
            .get(user_id)
            .ok_or_else(|| PermissionError::UserNotFound(user_id.clone()))?;

        // 检查是否被拒绝
        if user.denied_permissions.contains(permission) {
            return Ok(false);
        }

        // 检查用户直接权限
        if user.permissions.contains(permission) {
            return Ok(true);
        }

        // 检查角色权限
        let roles = self.roles.read().await;
        for role_id in &user.roles {
            if let Some(role) = roles.get(role_id) {
                if role.permissions.contains(permission) {
                    return Ok(true);
                }
            }
        }

        // 检查资源 ACL
        if let Some(resource_id) = resource_id {
            let acls = self.acls.read().await;
            if let Some(acl) = acls.get(resource_id) {
                // 检查是否是所有者
                if &acl.owner_id == user_id {
                    return Ok(true);
                }

                // 检查用户权限
                if let Some(perms) = acl.user_permissions.get(user_id) {
                    if perms.contains(permission) {
                        return Ok(true);
                    }
                }

                // 检查角色权限
                for role_id in &user.roles {
                    if let Some(perms) = acl.role_permissions.get(role_id) {
                        if perms.contains(permission) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// 创建资源 ACL
    pub async fn create_resource_acl(
        &self,
        resource_id: ResourceId,
        resource_type: ResourceType,
        owner_id: UserId,
    ) -> Result<(), PermissionError> {
        let acl = ResourceAcl::new(resource_id.clone(), resource_type, owner_id);

        let mut acls = self.acls.write().await;
        acls.insert(resource_id, acl);

        Ok(())
    }

    /// 授予资源权限
    pub async fn grant_resource_permission(
        &self,
        resource_id: &ResourceId,
        user_id: &UserId,
        permissions: Vec<Permission>,
    ) -> Result<(), PermissionError> {
        let mut acls = self.acls.write().await;
        let acl = acls
            .get_mut(resource_id)
            .ok_or_else(|| PermissionError::ResourceNotFound(resource_id.clone()))?;

        acl.grant_user(user_id.clone(), permissions);

        Ok(())
    }

    /// 生成 API Token
    pub fn generate_token(&self, user_id: &UserId, secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(user_id.as_bytes());
        hasher.update(secret.as_bytes());
        hasher.update(Utc::now().timestamp().to_string().as_bytes());
        format!(
            "tk_{}",
            base64::Engine::encode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                &hasher.finalize()
            )
        )
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}
