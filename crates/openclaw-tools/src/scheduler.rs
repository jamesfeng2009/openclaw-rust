//! 定时任务调度器

use crate::types::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// 调度器错误
#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("任务不存在: {0}")]
    TaskNotFound(TaskId),

    #[error("无效的 cron 表达式: {0}")]
    InvalidCron(String),

    #[error("任务执行失败: {0}")]
    ExecutionFailed(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 任务执行器类型
pub type TaskExecutor = Box<
    dyn Fn(
            TaskAction,
            HashMap<String, serde_json::Value>,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send>,
        > + Send
        + Sync,
>;

/// 定时任务调度器
pub struct TaskScheduler {
    tasks: Arc<RwLock<HashMap<TaskId, ScheduleTask>>>,
    executors: Arc<RwLock<HashMap<String, TaskExecutor>>>,
    running: Arc<RwLock<bool>>,
}

impl TaskScheduler {
    /// 创建新的调度器
    pub async fn new() -> Result<Self, SchedulerError> {
        Ok(Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            executors: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// 注册执行器
    pub async fn register_executor(&self, name: String, executor: TaskExecutor) {
        let mut executors = self.executors.write().await;
        executors.insert(name, executor);
    }

    /// 添加定时任务
    pub async fn add_task(&self, mut task: ScheduleTask) -> Result<TaskId, SchedulerError> {
        let task_id = task.id.clone();

        task.enabled = true;
        task.updated_at = Utc::now();

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(task_id.clone(), task);
        }

        info!("添加定时任务: {}", task_id);
        Ok(task_id)
    }

    /// 移除任务
    pub async fn remove_task(&self, task_id: &TaskId) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;

        if tasks.remove(task_id).is_some() {
            info!("移除定时任务: {}", task_id);
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id.clone()))
        }
    }

    /// 启用任务
    pub async fn enable_task(&self, task_id: &TaskId) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.enabled = true;
            task.updated_at = Utc::now();
            info!("启用定时任务: {}", task_id);
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id.clone()))
        }
    }

    /// 禁用任务
    pub async fn disable_task(&self, task_id: &TaskId) -> Result<(), SchedulerError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.enabled = false;
            task.updated_at = Utc::now();
            info!("禁用定时任务: {}", task_id);
            Ok(())
        } else {
            Err(SchedulerError::TaskNotFound(task_id.clone()))
        }
    }

    /// 手动执行任务
    pub async fn execute_task(
        &self,
        task_id: &TaskId,
    ) -> Result<serde_json::Value, SchedulerError> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks
                .get(task_id)
                .cloned()
                .ok_or_else(|| SchedulerError::TaskNotFound(task_id.clone()))?
        };

        if !task.enabled {
            return Err(SchedulerError::ExecutionFailed("任务已禁用".to_string()));
        }

        info!("执行任务: {} ({})", task.name, task_id);

        // 更新运行次数
        {
            let mut tasks = self.tasks.write().await;
            if let Some(t) = tasks.get_mut(task_id) {
                t.run_count += 1;
                t.last_run = Some(Utc::now());
            }
        }

        // 执行任务
        let executors = self.executors.read().await;
        if let Some(executor) = executors.get("default") {
            let params = match &task.action {
                TaskAction::ExecuteTool { parameters, .. } => parameters.clone(),
                _ => HashMap::new(),
            };
            executor(task.action.clone(), params)
                .await
                .map_err(|e| SchedulerError::ExecutionFailed(e))
        } else {
            Err(SchedulerError::ExecutionFailed(
                "未找到任务执行器".to_string(),
            ))
        }
    }

    /// 获取任务
    pub async fn get_task(&self, task_id: &TaskId) -> Option<ScheduleTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// 列出所有任务
    pub async fn list_tasks(&self) -> Vec<ScheduleTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// 启动调度器
    pub async fn start(&self) -> Result<(), SchedulerError> {
        let mut running = self.running.write().await;
        *running = true;
        info!("定时任务调度器已启动");
        Ok(())
    }

    /// 停止调度器
    pub async fn shutdown(&self) -> Result<(), SchedulerError> {
        let mut running = self.running.write().await;
        *running = false;
        info!("定时任务调度器已停止");
        Ok(())
    }
}

/// 创建定时任务
pub fn create_task(
    name: String,
    description: String,
    schedule: Schedule,
    action: TaskAction,
) -> ScheduleTask {
    ScheduleTask {
        id: Uuid::new_v4().to_string(),
        name,
        description,
        schedule,
        action,
        enabled: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_run: None,
        next_run: None,
        run_count: 0,
    }
}
