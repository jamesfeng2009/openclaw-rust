//! 模型故障转移系统
//!
//! 提供：
//! - 多模型账户自动切换
//! - 失败重试机制
//! - 负载均衡
//! - 健康检查

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::providers::AIProvider;
use crate::types::{
    ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, Provider, StreamChunk,
};

/// 故障转移错误
#[derive(Debug, Error)]
pub enum FailoverError {
    #[error("没有可用的提供商")]
    NoProviderAvailable,

    #[error("所有提供商都失败")]
    AllProvidersFailed,

    #[error("提供商被熔断: {0}")]
    CircuitOpen(String),

    #[error("超时")]
    Timeout,

    #[error("内部错误: {0}")]
    Internal(String),
}

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverProviderConfig {
    /// 提供商名称
    pub name: String,
    /// 提供商类型
    pub provider_type: Provider,
    /// 优先级 (越小越高)
    pub priority: u8,
    /// 权重 (用于负载均衡)
    pub weight: u8,
    /// 是否启用
    pub enabled: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试延迟 (毫秒)
    pub retry_delay_ms: u64,
    /// 超时时间 (秒)
    pub timeout_seconds: u64,
    /// 每分钟最大请求数
    pub rate_limit: Option<u32>,
    /// 模型映射 (通用模型名 -> 具体模型名)
    pub model_mapping: HashMap<String, String>,
}

impl Default for FailoverProviderConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            provider_type: Provider::OpenAI,
            priority: 100,
            weight: 1,
            enabled: true,
            max_retries: 3,
            retry_delay_ms: 1000,
            timeout_seconds: 60,
            rate_limit: None,
            model_mapping: HashMap::new(),
        }
    }
}

/// 提供商状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    /// 提供商名称
    pub name: String,
    /// 是否健康
    pub healthy: bool,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 最后成功时间
    pub last_success: Option<DateTime<Utc>>,
    /// 最后失败时间
    pub last_failure: Option<DateTime<Utc>>,
    /// 最后错误消息
    pub last_error: Option<String>,
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 平均延迟 (毫秒)
    pub avg_latency_ms: f64,
    /// 当前是否熔断
    pub circuit_open: bool,
    /// 熔断重试时间
    pub circuit_retry_at: Option<DateTime<Utc>>,
}

impl ProviderStatus {
    pub fn new(name: String) -> Self {
        Self {
            name,
            healthy: true,
            consecutive_failures: 0,
            last_success: None,
            last_failure: None,
            last_error: None,
            total_requests: 0,
            successful_requests: 0,
            avg_latency_ms: 0.0,
            circuit_open: false,
            circuit_retry_at: None,
        }
    }

    pub fn record_success(&mut self, latency_ms: u64) {
        self.healthy = true;
        self.consecutive_failures = 0;
        self.last_success = Some(Utc::now());
        self.total_requests += 1;
        self.successful_requests += 1;
        self.circuit_open = false;
        self.circuit_retry_at = None;

        // 更新平均延迟
        if self.total_requests == 1 {
            self.avg_latency_ms = latency_ms as f64;
        } else {
            self.avg_latency_ms = (self.avg_latency_ms * (self.total_requests - 1) as f64
                + latency_ms as f64)
                / self.total_requests as f64;
        }
    }

    pub fn record_failure(&mut self, error: String, threshold: u32) {
        self.last_failure = Some(Utc::now());
        self.last_error = Some(error);
        self.total_requests += 1;
        self.consecutive_failures += 1;

        if self.consecutive_failures >= threshold {
            self.healthy = false;
            self.circuit_open = true;
            self.circuit_retry_at = Some(Utc::now() + chrono::Duration::seconds(30));
            warn!("提供商 {} 熔断，将在 30 秒后重试", self.name);
        }
    }

    pub fn should_try(&self) -> bool {
        if !self.circuit_open {
            return true;
        }

        // 检查是否过了熔断期
        if let Some(retry_at) = self.circuit_retry_at {
            Utc::now() >= retry_at
        } else {
            false
        }
    }
}

/// 故障转移策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FailoverStrategy {
    /// 按优先级顺序
    #[default]
    Priority,
    /// 加权随机
    WeightedRandom,
    /// 轮询
    RoundRobin,
    /// 最少连接
    LeastConnections,
    /// 最低延迟
    LowestLatency,
}


/// 故障转移配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// 故障转移策略
    pub strategy: FailoverStrategy,
    /// 熔断阈值 (连续失败次数)
    pub circuit_breaker_threshold: u32,
    /// 熔断恢复时间 (秒)
    pub circuit_breaker_recovery_seconds: u64,
    /// 全局超时 (秒)
    pub global_timeout_seconds: u64,
    /// 健康检查间隔 (秒)
    pub health_check_interval: u64,
    /// 是否记录详细日志
    pub verbose_logging: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            strategy: FailoverStrategy::Priority,
            circuit_breaker_threshold: 3,
            circuit_breaker_recovery_seconds: 30,
            global_timeout_seconds: 120,
            health_check_interval: 60,
            verbose_logging: false,
        }
    }
}

/// 模型故障转移管理器
pub struct FailoverManager {
    config: FailoverConfig,
    providers: Arc<RwLock<HashMap<String, (FailoverProviderConfig, Arc<dyn AIProvider>)>>>,
    statuses: Arc<RwLock<HashMap<String, ProviderStatus>>>,
    round_robin_counter: AtomicUsize,
    request_counts: Arc<RwLock<HashMap<String, u64>>>,
}

impl FailoverManager {
    pub fn new(config: FailoverConfig) -> Self {
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            round_robin_counter: AtomicUsize::new(0),
            request_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册提供商
    pub async fn register_provider(
        &self,
        config: FailoverProviderConfig,
        provider: Arc<dyn AIProvider>,
    ) {
        let name = config.name.clone();
        let status_name = config.name.clone();

        {
            let mut providers = self.providers.write().await;
            providers.insert(name.clone(), (config, provider));
        }

        {
            let mut statuses = self.statuses.write().await;
            statuses.insert(name.clone(), ProviderStatus::new(status_name));
        }

        info!("注册提供商: {}", name);
    }

    /// 移除提供商
    pub async fn remove_provider(&self, name: &str) {
        {
            let mut providers = self.providers.write().await;
            providers.remove(name);
        }

        {
            let mut statuses = self.statuses.write().await;
            statuses.remove(name);
        }

        info!("移除提供商: {}", name);
    }

    /// 发送聊天请求 (自动故障转移)
    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, FailoverError> {
        let providers = self.providers.read().await;
        let available: Vec<_> = providers
            .iter()
            .filter(|(_, (config, _))| config.enabled)
            .collect();

        if available.is_empty() {
            return Err(FailoverError::NoProviderAvailable);
        }

        // 选择提供商
        let selected = self.select_provider(&available).await?;

        // 执行请求
        self.execute_with_retry(selected, request).await
    }

    /// 选择提供商
    async fn select_provider<'a>(
        &self,
        available: &'a [(
            &'a String,
            &'a (FailoverProviderConfig, Arc<dyn AIProvider>),
        )],
    ) -> Result<
        (
            &'a String,
            &'a (FailoverProviderConfig, Arc<dyn AIProvider>),
        ),
        FailoverError,
    > {
        let statuses = self.statuses.read().await;

        // 过滤可用的提供商
        let healthy: Vec<_> = available
            .iter()
            .filter(|(name, _)| statuses.get(*name).map(|s| s.should_try()).unwrap_or(true))
            .collect();

        if healthy.is_empty() {
            return Err(FailoverError::AllProvidersFailed);
        }

        match self.config.strategy {
            FailoverStrategy::Priority => {
                // 按优先级排序，选择最高的
                healthy
                    .into_iter()
                    .min_by_key(|(_name, (config, _))| config.priority)
                    .map(|(name, provider)| (*name, *provider))
                    .ok_or(FailoverError::NoProviderAvailable)
            }
            FailoverStrategy::RoundRobin => {
                let count = self.round_robin_counter.fetch_add(1, Ordering::SeqCst);
                let idx = count % healthy.len();
                let (name, provider) = healthy[idx];
                Ok((name, *provider))
            }
            FailoverStrategy::LowestLatency => {
                // 选择平均延迟最低的
                healthy
                    .into_iter()
                    .min_by(|a, b| {
                        let a_latency = statuses
                            .get(a.0)
                            .map(|s| s.avg_latency_ms)
                            .unwrap_or(f64::MAX);
                        let b_latency = statuses
                            .get(b.0)
                            .map(|s| s.avg_latency_ms)
                            .unwrap_or(f64::MAX);
                        a_latency.partial_cmp(&b_latency).unwrap()
                    })
                    .map(|(name, provider)| (*name, *provider))
                    .ok_or(FailoverError::NoProviderAvailable)
            }
            _ => {
                // 默认返回第一个
                let (name, provider) = healthy[0];
                Ok((name, *provider))
            }
        }
    }

    /// 带重试的执行
    async fn execute_with_retry(
        &self,
        selected: (&String, &(FailoverProviderConfig, Arc<dyn AIProvider>)),
        mut request: ChatRequest,
    ) -> Result<ChatResponse, FailoverError> {
        let (name, (config, provider)) = selected;
        let name = name.clone();
        let max_retries = config.max_retries;

        // 应用模型映射
        if let Some(mapped) = config.model_mapping.get(&request.model) {
            request.model = mapped.clone();
        }

        for attempt in 0..=max_retries {
            let start = std::time::Instant::now();

            match provider.chat(request.clone()).await {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;

                    // 记录成功
                    {
                        let mut statuses = self.statuses.write().await;
                        if let Some(status) = statuses.get_mut(&name) {
                            status.record_success(latency);
                        }
                    }

                    if self.config.verbose_logging {
                        debug!(
                            "提供商 {} 请求成功 (延迟: {}ms, 尝试: {})",
                            name,
                            latency,
                            attempt + 1
                        );
                    }

                    return Ok(response);
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    warn!(
                        "提供商 {} 请求失败: {} (尝试 {}/{})",
                        name,
                        error_msg,
                        attempt + 1,
                        max_retries + 1
                    );

                    // 记录失败
                    {
                        let mut statuses = self.statuses.write().await;
                        if let Some(status) = statuses.get_mut(&name) {
                            status.record_failure(
                                error_msg.clone(),
                                self.config.circuit_breaker_threshold,
                            );
                        }
                    }

                    // 如果还有重试次数，等待后重试
                    if attempt < max_retries {
                        tokio::time::sleep(Duration::from_millis(config.retry_delay_ms)).await;
                    } else {
                        // 尝试故障转移到其他提供商
                        return self.failover_to_next(request).await;
                    }
                }
            }
        }

        Err(FailoverError::AllProvidersFailed)
    }

    /// 故障转移到下一个提供商
    async fn failover_to_next(&self, request: ChatRequest) -> Result<ChatResponse, FailoverError> {
        info!("尝试故障转移到其他提供商");

        let providers = self.providers.read().await;
        let statuses = self.statuses.read().await;

        // 获取所有可用提供商（排除已熔断的）
        let available: Vec<_> = providers
            .iter()
            .filter(|(name, (config, _))| {
                config.enabled && statuses.get(*name).map(|s| s.should_try()).unwrap_or(true)
            })
            .collect();

        drop(statuses);

        // 尝试每个提供商
        for (name, (config, provider)) in available {
            let mut mapped_request = request.clone();
            if let Some(mapped) = config.model_mapping.get(&request.model) {
                mapped_request.model = mapped.clone();
            }

            match provider.chat(mapped_request).await {
                Ok(response) => {
                    // 记录成功
                    let mut statuses = self.statuses.write().await;
                    if let Some(status) = statuses.get_mut(name) {
                        status.record_success(0);
                    }

                    info!("故障转移到 {} 成功", name);
                    return Ok(response);
                }
                Err(e) => {
                    warn!("提供商 {} 也失败: {}", name, e);

                    let mut statuses = self.statuses.write().await;
                    if let Some(status) = statuses.get_mut(name) {
                        status.record_failure(e.to_string(), self.config.circuit_breaker_threshold);
                    }
                }
            }
        }

        Err(FailoverError::AllProvidersFailed)
    }

    /// 获取提供商状态
    pub async fn get_status(&self, name: &str) -> Option<ProviderStatus> {
        let statuses = self.statuses.read().await;
        statuses.get(name).cloned()
    }

    /// 获取所有状态
    pub async fn get_all_statuses(&self) -> HashMap<String, ProviderStatus> {
        let statuses = self.statuses.read().await;
        statuses.clone()
    }

    /// 重置提供商状态
    pub async fn reset_status(&self, name: &str) {
        let mut statuses = self.statuses.write().await;
        if let Some(status) = statuses.get_mut(name) {
            status.healthy = true;
            status.consecutive_failures = 0;
            status.circuit_open = false;
            status.circuit_retry_at = None;
        }
        info!("重置提供商 {} 状态", name);
    }

    /// 健康检查
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let providers = self.providers.read().await;
        let mut results = HashMap::new();

        for (name, (_, provider)) in providers.iter() {
            // 简单的健康检查：尝试获取模型列表
            let healthy = provider.models().await.is_ok();
            results.insert(name.clone(), healthy);

            if !healthy {
                let mut statuses = self.statuses.write().await;
                if let Some(status) = statuses.get_mut(name) {
                    status.healthy = false;
                }
            }
        }

        results
    }
}

#[async_trait]
impl AIProvider for FailoverManager {
    fn name(&self) -> &str {
        "failover"
    }

    async fn chat(&self, request: ChatRequest) -> openclaw_core::Result<ChatResponse> {
        self.chat(request)
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> openclaw_core::Result<Pin<Box<dyn Stream<Item = openclaw_core::Result<StreamChunk>> + Send>>>
    {
        // 尝试使用第一个可用提供商进行流式聊天
        let providers = self.providers.read().await;
        for (_, (config, provider)) in providers.iter() {
            if config.enabled {
                return provider.chat_stream(request).await;
            }
        }
        Err(openclaw_core::OpenClawError::AIProvider(
            "没有可用的提供商".to_string(),
        ))
    }

    async fn embed(&self, request: EmbeddingRequest) -> openclaw_core::Result<EmbeddingResponse> {
        // 使用第一个可用提供商进行嵌入
        let providers = self.providers.read().await;
        for (_, (config, provider)) in providers.iter() {
            if config.enabled {
                return provider.embed(request).await;
            }
        }
        Err(openclaw_core::OpenClawError::AIProvider(
            "没有可用的提供商".to_string(),
        ))
    }

    async fn models(&self) -> openclaw_core::Result<Vec<String>> {
        let providers = self.providers.read().await;
        let mut models = Vec::new();

        for (_, (config, provider)) in providers.iter() {
            if config.enabled
                && let Ok(provider_models) = provider.models().await {
                    models.extend(provider_models);
                }
        }

        Ok(models)
    }

    async fn health_check(&self) -> openclaw_core::Result<bool> {
        let results = self.health_check().await;
        Ok(results.values().any(|&v| v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_status() {
        let mut status = ProviderStatus::new("test".to_string());
        assert!(status.healthy);

        status.record_success(100);
        assert_eq!(status.total_requests, 1);
        assert_eq!(status.successful_requests, 1);

        status.record_failure("error".to_string(), 3);
        assert_eq!(status.consecutive_failures, 1);
    }

    #[test]
    fn test_circuit_breaker() {
        let mut status = ProviderStatus::new("test".to_string());

        // 连续失败达到阈值
        for _ in 0..3 {
            status.record_failure("error".to_string(), 3);
        }

        assert!(!status.should_try());
        assert!(status.circuit_open);
    }
}
