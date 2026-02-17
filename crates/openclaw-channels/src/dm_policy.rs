//! 通道 DM (私聊) 策略管理
//!
//! 提供：
//! - 每账户 DM 策略覆盖
//! - 未知用户配对码机制
//! - 访问控制列表 (ACL)
//! - 消息过滤和限流

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// DM 策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmPolicyConfig {
    /// 默认策略
    pub default_policy: DmAccessPolicy,
    /// 是否需要配对码
    pub require_pairing_code: bool,
    /// 配对码有效期 (秒)
    pub pairing_code_ttl: u64,
    /// 消息限流: 每分钟最大消息数
    pub rate_limit_per_minute: u32,
    /// 是否允许群组消息
    pub allow_group_messages: bool,
    /// 禁止的用户列表
    pub blocked_users: HashSet<String>,
    /// 允许的用户列表 (白名单)
    pub allowed_users: HashSet<String>,
}

impl Default for DmPolicyConfig {
    fn default() -> Self {
        Self {
            default_policy: DmAccessPolicy::RequirePairing,
            require_pairing_code: true,
            pairing_code_ttl: 300, // 5 分钟
            rate_limit_per_minute: 10,
            allow_group_messages: true,
            blocked_users: HashSet::new(),
            allowed_users: HashSet::new(),
        }
    }
}

/// DM 访问策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmAccessPolicy {
    /// 允许所有
    AllowAll,
    /// 需要配对码
    RequirePairing,
    /// 仅白名单
    WhitelistOnly,
    /// 完全禁止
    Blocked,
}

/// 账户级 DM 策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDmPolicy {
    /// 账户 ID
    pub account_id: String,
    /// 通道类型
    pub channel: String,
    /// 此账户的策略
    pub policy: DmAccessPolicy,
    /// 是否覆盖默认策略
    pub override_default: bool,
    /// 此账户的黑名单
    pub blocked: HashSet<String>,
    /// 此账户的白名单
    pub allowed: HashSet<String>,
    /// 已配对用户
    pub paired_users: HashSet<String>,
    /// 配对码
    pub pairing_codes: HashMap<String, PairingCode>,
    /// 消息计数 (用于限流)
    pub message_counts: HashMap<String, u32>,
    /// 上次重置时间
    pub last_reset: DateTime<Utc>,
}

impl AccountDmPolicy {
    pub fn new(account_id: String, channel: String) -> Self {
        Self {
            account_id,
            channel,
            policy: DmAccessPolicy::RequirePairing,
            override_default: false,
            blocked: HashSet::new(),
            allowed: HashSet::new(),
            paired_users: HashSet::new(),
            pairing_codes: HashMap::new(),
            message_counts: HashMap::new(),
            last_reset: Utc::now(),
        }
    }

    /// 生成配对码
    pub fn generate_pairing_code(&mut self, ttl_seconds: u64) -> String {
        use rand::Rng;
        let code: String = (0..6)
            .map(|_| rand::thread_rng().gen_range(0..10).to_string())
            .collect();

        let pairing_code = PairingCode {
            code: code.clone(),
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::seconds(ttl_seconds as i64),
            used: false,
        };

        self.pairing_codes.insert(code.clone(), pairing_code);
        info!("为账户 {} 生成配对码: {}", self.account_id, code);
        code
    }

    /// 验证配对码
    pub fn verify_pairing_code(&mut self, code: &str, user_id: &str) -> bool {
        if let Some(pairing_code) = self.pairing_codes.get_mut(code) {
            if pairing_code.used {
                warn!("配对码已使用: {}", code);
                return false;
            }

            if pairing_code.expires_at < Utc::now() {
                warn!("配对码已过期: {}", code);
                return false;
            }

            // 标记为已使用
            pairing_code.used = true;

            // 添加到已配对用户
            self.paired_users.insert(user_id.to_string());
            info!("用户 {} 已配对到账户 {}", user_id, self.account_id);

            true
        } else {
            warn!("无效的配对码: {}", code);
            false
        }
    }

    /// 检查用户是否已配对
    pub fn is_paired(&self, user_id: &str) -> bool {
        self.paired_users.contains(user_id)
    }

    /// 移除配对
    pub fn unpair(&mut self, user_id: &str) {
        self.paired_users.remove(user_id);
        info!("移除用户 {} 的配对", user_id);
    }

    /// 检查消息限流
    pub fn check_rate_limit(&mut self, user_id: &str, limit: u32) -> bool {
        // 每分钟重置
        let now = Utc::now();
        if (now - self.last_reset).num_minutes() >= 1 {
            self.message_counts.clear();
            self.last_reset = now;
        }

        let count = self.message_counts.entry(user_id.to_string()).or_insert(0);
        if *count >= limit {
            warn!("用户 {} 触发限流", user_id);
            return false;
        }

        *count += 1;
        true
    }

    /// 清理过期配对码
    pub fn cleanup_expired_codes(&mut self) {
        let now = Utc::now();
        self.pairing_codes.retain(|_, code| code.expires_at > now);
    }
}

/// 配对码
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCode {
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// DM 策略管理器
pub struct DmPolicyManager {
    config: DmPolicyConfig,
    account_policies: Arc<RwLock<HashMap<String, AccountDmPolicy>>>,
}

impl DmPolicyManager {
    pub fn new(config: DmPolicyConfig) -> Self {
        Self {
            config,
            account_policies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取或创建账户策略
    pub async fn get_or_create_policy(&self, account_id: &str, channel: &str) -> AccountDmPolicy {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        policies
            .entry(key.clone())
            .or_insert_with(|| AccountDmPolicy::new(account_id.to_string(), channel.to_string()))
            .clone()
    }

    /// 设置账户策略
    pub async fn set_account_policy(
        &self,
        account_id: &str,
        channel: &str,
        policy: DmAccessPolicy,
    ) {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        if let Some(account_policy) = policies.get_mut(&key) {
            account_policy.policy = policy;
            account_policy.override_default = true;
        } else {
            let mut account_policy =
                AccountDmPolicy::new(account_id.to_string(), channel.to_string());
            account_policy.policy = policy;
            account_policy.override_default = true;
            policies.insert(key, account_policy);
        }

        info!(
            "设置账户 {} 在 {} 的 DM 策略: {:?}",
            account_id, channel, policy
        );
    }

    /// 检查用户是否可以发送消息
    pub async fn can_send_message(
        &self,
        account_id: &str,
        channel: &str,
        user_id: &str,
        is_group: bool,
    ) -> DmAccessResult {
        // 群组消息检查
        if is_group && !self.config.allow_group_messages {
            return DmAccessResult::Blocked("群组消息已禁用".to_string());
        }

        // 全局黑名单检查
        if self.config.blocked_users.contains(user_id) {
            return DmAccessResult::Blocked("用户在全局黑名单中".to_string());
        }

        // 全局白名单检查
        if self.config.allowed_users.contains(user_id) {
            return DmAccessResult::Allowed;
        }

        // 获取账户策略
        let account_policy = self.get_or_create_policy(account_id, channel).await;

        // 账户黑名单检查
        if account_policy.blocked.contains(user_id) {
            return DmAccessResult::Blocked("用户在账户黑名单中".to_string());
        }

        // 账户白名单检查
        if account_policy.allowed.contains(user_id) {
            return DmAccessResult::Allowed;
        }

        // 检查已配对
        if account_policy.is_paired(user_id) {
            return DmAccessResult::Allowed;
        }

        // 获取有效策略
        let effective_policy = if account_policy.override_default {
            account_policy.policy
        } else {
            self.config.default_policy
        };

        match effective_policy {
            DmAccessPolicy::AllowAll => DmAccessResult::Allowed,
            DmAccessPolicy::RequirePairing => {
                DmAccessResult::NeedsPairing("需要配对码才能发送消息".to_string())
            }
            DmAccessPolicy::WhitelistOnly => {
                DmAccessResult::Blocked("仅白名单用户可以发送消息".to_string())
            }
            DmAccessPolicy::Blocked => DmAccessResult::Blocked("DM 已禁用".to_string()),
        }
    }

    /// 生成配对码
    pub async fn generate_pairing_code(
        &self,
        account_id: &str,
        channel: &str,
    ) -> Result<String, String> {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        let account_policy = policies
            .get_mut(&key)
            .ok_or_else(|| "账户不存在".to_string())?;

        Ok(account_policy.generate_pairing_code(self.config.pairing_code_ttl))
    }

    /// 验证配对码
    pub async fn verify_pairing_code(
        &self,
        account_id: &str,
        channel: &str,
        code: &str,
        user_id: &str,
    ) -> bool {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        if let Some(account_policy) = policies.get_mut(&key) {
            account_policy.verify_pairing_code(code, user_id)
        } else {
            false
        }
    }

    /// 检查限流
    pub async fn check_rate_limit(&self, account_id: &str, channel: &str, user_id: &str) -> bool {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        if let Some(account_policy) = policies.get_mut(&key) {
            account_policy.check_rate_limit(user_id, self.config.rate_limit_per_minute)
        } else {
            true // 无策略时允许
        }
    }

    /// 添加到黑名单
    pub async fn add_to_blocked(&self, account_id: &str, channel: &str, user_id: &str) {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        if let Some(account_policy) = policies.get_mut(&key) {
            account_policy.blocked.insert(user_id.to_string());
            account_policy.allowed.remove(user_id);
        }
    }

    /// 添加到白名单
    pub async fn add_to_allowed(&self, account_id: &str, channel: &str, user_id: &str) {
        let key = format!("{}:{}", channel, account_id);

        let mut policies = self.account_policies.write().await;
        if let Some(account_policy) = policies.get_mut(&key) {
            account_policy.allowed.insert(user_id.to_string());
            account_policy.blocked.remove(user_id);
        }
    }

    /// 清理过期数据
    pub async fn cleanup(&self) {
        let mut policies = self.account_policies.write().await;
        for policy in policies.values_mut() {
            policy.cleanup_expired_codes();
        }
    }

    /// 获取统计信息
    pub async fn stats(&self) -> DmPolicyStats {
        let policies = self.account_policies.read().await;

        let mut total_accounts = 0;
        let mut total_paired_users = 0;
        let mut total_blocked = 0;

        for policy in policies.values() {
            total_accounts += 1;
            total_paired_users += policy.paired_users.len();
            total_blocked += policy.blocked.len();
        }

        DmPolicyStats {
            total_accounts,
            total_paired_users,
            total_blocked,
            global_blocked: self.config.blocked_users.len(),
            global_allowed: self.config.allowed_users.len(),
        }
    }
}

impl Default for DmPolicyManager {
    fn default() -> Self {
        Self::new(DmPolicyConfig::default())
    }
}

/// DM 访问检查结果
#[derive(Debug, Clone)]
pub enum DmAccessResult {
    /// 允许
    Allowed,
    /// 需要配对
    NeedsPairing(String),
    /// 被阻止
    Blocked(String),
}

impl DmAccessResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, DmAccessResult::Allowed)
    }

    pub fn needs_pairing(&self) -> bool {
        matches!(self, DmAccessResult::NeedsPairing(_))
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, DmAccessResult::Blocked(_))
    }
}

/// DM 策略统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmPolicyStats {
    pub total_accounts: usize,
    pub total_paired_users: usize,
    pub total_blocked: usize,
    pub global_blocked: usize,
    pub global_allowed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_code_generation() {
        let mut policy = AccountDmPolicy::new("test-account".to_string(), "telegram".to_string());
        let code = policy.generate_pairing_code(300);

        assert_eq!(code.len(), 6);
        assert!(policy.pairing_codes.contains_key(&code));
    }

    #[test]
    fn test_pairing_code_verification() {
        let mut policy = AccountDmPolicy::new("test".to_string(), "test".to_string());
        let code = policy.generate_pairing_code(300);

        let result = policy.verify_pairing_code(&code, "user-123");
        assert!(result);
        assert!(policy.is_paired("user-123"));

        // 再次使用应该失败
        let result = policy.verify_pairing_code(&code, "user-456");
        assert!(!result);
    }

    #[test]
    fn test_rate_limiting() {
        let mut policy = AccountDmPolicy::new("test".to_string(), "test".to_string());

        // 前 5 次应该允许
        for _ in 0..5 {
            assert!(policy.check_rate_limit("user-1", 10));
        }
    }

    #[tokio::test]
    async fn test_dm_policy_manager() {
        let manager = DmPolicyManager::default();

        let result = manager
            .can_send_message("account-1", "telegram", "user-1", false)
            .await;

        assert!(result.needs_pairing());
    }
}
