//! 技能平台模块

use crate::types::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

/// 技能错误
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("技能不存在: {0}")]
    SkillNotFound(SkillId),

    #[error("工具不存在: {0}")]
    ToolNotFound(ToolId),

    #[error("执行失败: {0}")]
    ExecutionFailed(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 工具执行器类型
pub type ToolExecutor = Box<
    dyn Fn(
            &str,
            HashMap<String, serde_json::Value>,
            ToolContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult> + Send>>
        + Send
        + Sync,
>;

/// 技能平台
pub struct SkillPlatform {
    skills: Arc<RwLock<HashMap<SkillId, Skill>>>,
    executions: Arc<RwLock<Vec<SkillExecution>>>,
    tool_executors: Arc<RwLock<HashMap<String, ToolExecutor>>>,
}

impl SkillPlatform {
    /// 创建新的技能平台
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(Vec::new())),
            tool_executors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册工具执行器
    pub async fn register_tool_executor(&self, tool_id: &str, executor: ToolExecutor) {
        let mut executors = self.tool_executors.write().await;
        executors.insert(tool_id.to_string(), executor);
        info!("注册工具执行器: {}", tool_id);
    }

    /// 创建技能
    pub async fn create_skill(
        &self,
        name: String,
        description: String,
        category: SkillCategory,
        tools: Vec<ToolBinding>,
        triggers: Vec<SkillTrigger>,
    ) -> SkillId {
        let skill = Skill {
            id: Uuid::new_v4().to_string(),
            name,
            description,
            version: "1.0.0".to_string(),
            author: None,
            category,
            tools,
            triggers,
            enabled: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let id = skill.id.clone();

        let mut skills = self.skills.write().await;
        skills.insert(id.clone(), skill);

        info!("创建技能: {}", id);
        id
    }

    /// 更新技能
    pub async fn update_skill(
        &self,
        skill_id: &SkillId,
        updates: SkillUpdates,
    ) -> Result<(), SkillError> {
        let mut skills = self.skills.write().await;

        let skill = skills
            .get_mut(skill_id)
            .ok_or_else(|| SkillError::SkillNotFound(skill_id.clone()))?;

        if let Some(name) = updates.name {
            skill.name = name;
        }
        if let Some(description) = updates.description {
            skill.description = description;
        }
        if let Some(enabled) = updates.enabled {
            skill.enabled = enabled;
        }
        if let Some(tools) = updates.tools {
            skill.tools = tools;
        }
        if let Some(triggers) = updates.triggers {
            skill.triggers = triggers;
        }
        skill.updated_at = Utc::now();

        info!("更新技能: {}", skill_id);
        Ok(())
    }

    /// 删除技能
    pub async fn delete_skill(&self, skill_id: &SkillId) -> Result<(), SkillError> {
        let mut skills = self.skills.write().await;

        if skills.remove(skill_id).is_some() {
            info!("删除技能: {}", skill_id);
            Ok(())
        } else {
            Err(SkillError::SkillNotFound(skill_id.clone()))
        }
    }

    /// 获取技能
    pub async fn get_skill(&self, skill_id: &SkillId) -> Option<Skill> {
        let skills = self.skills.read().await;
        skills.get(skill_id).cloned()
    }

    /// 列出所有技能
    pub async fn list_skills(&self) -> Vec<Skill> {
        let skills = self.skills.read().await;
        skills.values().cloned().collect()
    }

    /// 按分类列出技能
    pub async fn list_skills_by_category(&self, category: SkillCategory) -> Vec<Skill> {
        let skills = self.skills.read().await;
        skills
            .values()
            .filter(|s| s.category == category)
            .cloned()
            .collect()
    }

    /// 触发技能
    pub async fn trigger_skill(
        &self,
        skill_id: &SkillId,
        trigger: SkillTrigger,
        context: ToolContext,
    ) -> Result<SkillExecution, SkillError> {
        let skill = {
            let skills = self.skills.read().await;
            skills
                .get(skill_id)
                .cloned()
                .ok_or_else(|| SkillError::SkillNotFound(skill_id.clone()))?
        };

        if !skill.enabled {
            return Err(SkillError::ExecutionFailed("技能已禁用".to_string()));
        }

        let execution_id = Uuid::new_v4().to_string();
        let started_at = Utc::now();

        info!("执行技能: {} ({})", skill.name, execution_id);

        let mut results = vec![];

        // 执行所有绑定的工具
        for tool_binding in &skill.tools {
            let tool_id = &tool_binding.tool_id;
            let mut params = tool_binding.parameters.clone();

            // 从上下文中替换变量
            for key in context.variables.keys() {
                if let Some(serde_json::Value::String(template)) = params.get(key)
                    && template.starts_with("${")
                    && template.ends_with("}")
                {
                    let var_name = &template[2..template.len() - 1];
                    if let Some(var_value) = context.variables.get(var_name) {
                        params.insert(key.to_string(), var_value.clone());
                    }
                }
            }

            // 获取执行器并执行
            let executors = self.tool_executors.read().await;
            if let Some(executor) = executors.get(tool_id) {
                let result = executor(tool_id, params, context.clone()).await;
                results.push(result);
            } else {
                results.push(ToolResult::error(format!("工具执行器未注册: {}", tool_id)));
            }
        }

        let completed_at = Utc::now();
        let status = if results.iter().all(|r| r.success) {
            SkillExecutionStatus::Completed
        } else if results.iter().any(|r| r.success) {
            SkillExecutionStatus::Failed
        } else {
            SkillExecutionStatus::Failed
        };

        let execution = SkillExecution {
            id: execution_id,
            skill_id: skill_id.clone(),
            trigger,
            context,
            results,
            started_at,
            completed_at: Some(completed_at),
            status,
        };

        // 记录执行
        {
            let mut executions = self.executions.write().await;
            executions.push(execution.clone());
        }

        Ok(execution)
    }

    /// 匹配触发器
    pub async fn match_trigger(&self, input: &str) -> Option<(SkillId, SkillTrigger)> {
        let skills = self.skills.read().await;

        for skill in skills.values() {
            if !skill.enabled {
                continue;
            }

            for trigger in &skill.triggers {
                match trigger {
                    SkillTrigger::Command { pattern } => {
                        if input.starts_with(&format!("/{}", pattern)) {
                            return Some((skill.id.clone(), trigger.clone()));
                        }
                    }
                    SkillTrigger::Keyword { keywords } => {
                        for keyword in keywords {
                            if input.to_lowercase().contains(&keyword.to_lowercase()) {
                                return Some((skill.id.clone(), trigger.clone()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        None
    }

    /// 获取执行历史
    pub async fn get_execution_history(&self, skill_id: Option<&SkillId>) -> Vec<SkillExecution> {
        let executions = self.executions.read().await;

        match skill_id {
            Some(id) => executions
                .iter()
                .filter(|e| &e.skill_id == id)
                .cloned()
                .collect(),
            None => executions.clone(),
        }
    }
}

impl Default for SkillPlatform {
    fn default() -> Self {
        Self::new()
    }
}

/// 技能更新字段
pub struct SkillUpdates {
    pub name: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub tools: Option<Vec<ToolBinding>>,
    pub triggers: Option<Vec<SkillTrigger>>,
}
