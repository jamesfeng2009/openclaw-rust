//! Cron 定时任务调度器
//!
//! 支持 cron 表达式的定时任务调度：
//! - 标准 cron 表达式解析
//! - 时区支持
//! - 任务执行和错误处理
//! - 下次执行时间计算

use crate::types::{TaskAction, TaskId};
use chrono::{DateTime, Utc};
use cron::Schedule as CronSchedule;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Cron 调度器错误
#[derive(Debug, Error)]
pub enum CronError {
    #[error("无效的 cron 表达式: {0}")]
    InvalidExpression(String),

    #[error("任务不存在: {0}")]
    TaskNotFound(TaskId),

    #[error("任务执行失败: {0}")]
    ExecutionFailed(String),

    #[error("时区错误: {0}")]
    Timezone(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// Cron 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTask {
    /// 任务 ID
    pub id: TaskId,
    /// 任务名称
    pub name: String,
    /// 描述
    pub description: String,
    /// Cron 表达式 (如 "0 0 * * * *" 每小时执行)
    pub cron_expression: String,
    /// 时区 (默认 UTC)
    pub timezone: Option<String>,
    /// 任务动作
    pub action: TaskAction,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 上次执行时间
    pub last_run: Option<DateTime<Utc>>,
    /// 下次执行时间
    pub next_run: Option<DateTime<Utc>>,
    /// 执行次数
    pub run_count: usize,
    /// 最大执行次数 (0 为无限制)
    pub max_runs: usize,
    /// 错误计数
    pub error_count: usize,
    /// 上次错误
    pub last_error: Option<String>,
}

impl CronTask {
    /// 创建新的 cron 任务
    pub fn new(name: String, cron_expr: &str, action: TaskAction) -> Result<Self, CronError> {
        // 验证 cron 表达式
        let schedule = parse_cron_expression(cron_expr)?;

        // 计算下次执行时间
        let next_run = schedule
            .after(&Utc::now())
            .next()
            .map(|dt| DateTime::from_naive_utc_and_offset(dt.naive_utc(), Utc));

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            description: String::new(),
            cron_expression: cron_expr.to_string(),
            timezone: None,
            action,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_run: None,
            next_run,
            run_count: 0,
            max_runs: 0,
            error_count: 0,
            last_error: None,
        })
    }

    /// 解析并更新执行时间
    pub fn update_next_run(&mut self) -> Result<(), CronError> {
        let schedule = parse_cron_expression(&self.cron_expression)?;
        let now = Utc::now();

        self.next_run = schedule
            .after(&now)
            .next()
            .map(|dt| DateTime::from_naive_utc_and_offset(dt.naive_utc(), Utc));

        Ok(())
    }

    /// 检查是否应该执行
    pub fn should_run(&self) -> bool {
        if !self.enabled {
            return false;
        }

        if self.max_runs > 0 && self.run_count >= self.max_runs {
            return false;
        }

        match self.next_run {
            Some(next) => next <= Utc::now(),
            None => false,
        }
    }

    /// 记录执行
    pub fn record_execution(&mut self, success: bool, error: Option<String>) {
        self.last_run = Some(Utc::now());
        self.run_count += 1;

        if !success {
            self.error_count += 1;
            self.last_error = error;
        } else {
            self.last_error = None;
        }

        // 更新下次执行时间
        let _ = self.update_next_run();
    }
}

/// 解析 cron 表达式
fn parse_cron_expression(expr: &str) -> Result<CronSchedule, CronError> {
    // 支持 6 字段格式: 秒 分 时 日 月 周
    let expr = expr.trim();

    CronSchedule::try_from(expr)
        .map_err(|e| CronError::InvalidExpression(format!("{}: {}", expr, e)))
}

/// Cron 调度器
pub struct CronScheduler {
    tasks: Arc<RwLock<HashMap<TaskId, CronTask>>>,
    running: Arc<RwLock<bool>>,
}

impl CronScheduler {
    /// 创建新的调度器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 添加任务
    pub async fn add_task(&self, task: CronTask) -> Result<TaskId, CronError> {
        let task_id = task.id.clone();

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        info!("添加 cron 任务: {}", task_id);
        Ok(task_id)
    }

    /// 从 cron 表达式创建任务
    pub async fn schedule(
        &self,
        name: String,
        cron_expr: &str,
        action: TaskAction,
    ) -> Result<TaskId, CronError> {
        let task = CronTask::new(name, cron_expr, action)?;
        self.add_task(task).await
    }

    /// 移除任务
    pub async fn remove_task(&self, task_id: &TaskId) -> Result<(), CronError> {
        let mut tasks = self.tasks.write().await;

        if tasks.remove(task_id).is_some() {
            info!("移除 cron 任务: {}", task_id);
            Ok(())
        } else {
            Err(CronError::TaskNotFound(task_id.clone()))
        }
    }

    /// 启用任务
    pub async fn enable_task(&self, task_id: &TaskId) -> Result<(), CronError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.enabled = true;
            task.updated_at = Utc::now();
            info!("启用 cron 任务: {}", task_id);
            Ok(())
        } else {
            Err(CronError::TaskNotFound(task_id.clone()))
        }
    }

    /// 禁用任务
    pub async fn disable_task(&self, task_id: &TaskId) -> Result<(), CronError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.enabled = false;
            task.updated_at = Utc::now();
            info!("禁用 cron 任务: {}", task_id);
            Ok(())
        } else {
            Err(CronError::TaskNotFound(task_id.clone()))
        }
    }

    /// 获取任务
    pub async fn get_task(&self, task_id: &TaskId) -> Option<CronTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// 列出所有任务
    pub async fn list_tasks(&self) -> Vec<CronTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// 获取待执行任务
    pub async fn get_pending_tasks(&self) -> Vec<CronTask> {
        let tasks = self.tasks.read().await;
        tasks.values().filter(|t| t.should_run()).cloned().collect()
    }

    /// 启动调度器
    pub async fn start(&self) -> Result<(), CronError> {
        let mut running = self.running.write().await;
        *running = true;
        info!("Cron 调度器已启动");
        Ok(())
    }

    /// 停止调度器
    pub async fn shutdown(&self) -> Result<(), CronError> {
        let mut running = self.running.write().await;
        *running = false;
        info!("Cron 调度器已停止");
        Ok(())
    }

    /// 运行一次调度循环
    pub async fn tick(&self) -> Vec<(TaskId, Result<(), CronError>)> {
        let pending = self.get_pending_tasks().await;
        let mut results = Vec::new();

        for task in pending {
            let task_id = task.id.clone();
            let result = self.execute_task_internal(&task).await;
            results.push((task_id, result));
        }

        results
    }

    /// 执行任务
    async fn execute_task_internal(&self, task: &CronTask) -> Result<(), CronError> {
        info!("执行 cron 任务: {} ({})", task.name, task.id);

        let success = true;
        let error_msg = None;

        // 执行任务动作
        match &task.action {
            TaskAction::SendMessage { channel, message } => {
                debug!("发送消息到通道: {} 内容: {}", channel, message);
            }
            TaskAction::ExecuteTool {
                tool_id,
                parameters,
            } => {
                debug!("执行工具: {} 参数: {:?}", tool_id, parameters);
                // 实际执行由外部执行器完成
            }
            TaskAction::HttpCall {
                url,
                method,
                headers,
                body,
            } => {
                debug!(
                    "HTTP 调用: {} {} headers: {:?} body: {:?}",
                    method, url, headers, body
                );
                // 实际调用由外部处理器完成
            }
            TaskAction::TriggerWebhook {
                webhook_id,
                payload,
            } => {
                debug!("触发 Webhook: {} payload: {:?}", webhook_id, payload);
                // 实际调用由外部处理器完成
            }
            TaskAction::Script { language, code } => {
                debug!("执行脚本: {} code: {:?}", language, code);
                // 实际执行由外部处理器完成
            }
        }

        // 更新任务状态
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(&task.id) {
                t.record_execution(success, error_msg);
            }
        }

        Ok(())
    }

    /// 获取调度统计
    pub async fn stats(&self) -> SchedulerStats {
        let tasks = self.tasks.read().await;
        let enabled = tasks.values().filter(|t| t.enabled).count();
        let total_runs: usize = tasks.values().map(|t| t.run_count).sum();
        let total_errors: usize = tasks.values().map(|t| t.error_count).sum();

        SchedulerStats {
            total_tasks: tasks.len(),
            enabled_tasks: enabled,
            total_runs,
            total_errors,
        }
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// 调度器统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub total_tasks: usize,
    pub enabled_tasks: usize,
    pub total_runs: usize,
    pub total_errors: usize,
}

/// Cron 表达式工具函数
pub mod cron_utils {
    use super::*;

    /// 常用 cron 表达式
    pub const EVERY_MINUTE: &str = "0 * * * * *";
    pub const EVERY_HOUR: &str = "0 0 * * * *";
    pub const EVERY_DAY: &str = "0 0 0 * * *";
    pub const EVERY_WEEK: &str = "0 0 0 * * 0";
    pub const EVERY_MONTH: &str = "0 0 0 1 * *";

    /// 解析 cron 表达式并获取下次执行时间
    pub fn get_next_run(cron_expr: &str) -> Result<DateTime<Utc>, CronError> {
        let schedule = parse_cron_expression(cron_expr)?;

        schedule
            .after(&Utc::now())
            .next()
            .map(|dt| DateTime::from_naive_utc_and_offset(dt.naive_utc(), Utc))
            .ok_or_else(|| CronError::Internal("无法计算下次执行时间".to_string()))
    }

    /// 获取接下来 N 次执行时间
    pub fn get_next_runs(cron_expr: &str, count: usize) -> Result<Vec<DateTime<Utc>>, CronError> {
        let schedule = parse_cron_expression(cron_expr)?;

        Ok(schedule
            .after(&Utc::now())
            .take(count)
            .map(|dt| DateTime::from_naive_utc_and_offset(dt.naive_utc(), Utc))
            .collect())
    }

    /// 验证 cron 表达式
    pub fn validate_expression(expr: &str) -> Result<(), CronError> {
        parse_cron_expression(expr)?;
        Ok(())
    }

    /// 生成人类可读的描述
    pub fn describe(expr: &str) -> Result<String, CronError> {
        let _schedule = parse_cron_expression(expr)?;
        let next = get_next_run(expr)?;

        Ok(format!(
            "下次执行: {}",
            next.format("%Y-%m-%d %H:%M:%S UTC")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_task_creation() {
        let task = CronTask::new(
            "测试任务".to_string(),
            "0 0 * * * *",
            TaskAction::SendMessage {
                channel: "test".to_string(),
                message: "测试消息".to_string(),
            },
        );

        assert!(task.is_ok());
        let task = task.unwrap();
        assert_eq!(task.name, "测试任务");
        assert!(task.enabled);
    }

    #[test]
    fn test_invalid_cron_expression() {
        let result = CronTask::new(
            "测试".to_string(),
            "invalid",
            TaskAction::SendMessage {
                channel: "test".to_string(),
                message: "test".to_string(),
            },
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_next_run_calculation() {
        let next = cron_utils::get_next_run("0 0 * * * *");
        assert!(next.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler() {
        let scheduler = CronScheduler::new();

        let task_id = scheduler
            .schedule(
                "每小时任务".to_string(),
                "0 0 * * * *",
                TaskAction::SendMessage {
                    channel: "test".to_string(),
                    message: "定时消息".to_string(),
                },
            )
            .await
            .unwrap();

        let task = scheduler.get_task(&task_id).await;
        assert!(task.is_some());
    }
}
