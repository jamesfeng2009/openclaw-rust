//! Agent 工作区记忆管理
//! 
//! 实现 OpenClaw 风格的 Markdown 记忆系统：
//! - AGENTS.md: 智能体的操作说明和记忆使用策略
//! - SOUL.md: 个性设定
//! - USER.md: 用户信息
//! - memory/YYYY-MM-DD.md: 每日记忆
//! - MEMORY.md: 长期记忆汇总

use chrono::{Datelike, Local, NaiveDate};
use openclaw_core::Result;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

const AGENTS_FILENAME: &str = "AGENTS.md";
const SOUL_FILENAME: &str = "SOUL.md";
const USER_FILENAME: &str = "USER.md";
const MEMORY_FILENAME: &str = "MEMORY.md";
const MEMORY_DIR: &str = "memory";

#[derive(Debug, Clone)]
pub struct AgentWorkspace {
    pub agent_id: String,
    pub workspace_path: PathBuf,
}

impl AgentWorkspace {
    pub fn new(agent_id: String, workspace_path: PathBuf) -> Self {
        Self {
            agent_id,
            workspace_path,
        }
    }

    pub fn from_config(agent_id: String, workspace: PathBuf) -> Self {
        Self::new(agent_id, workspace)
    }

    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    pub fn memory_dir(&self) -> PathBuf {
        self.workspace_path.join(MEMORY_DIR)
    }

    pub fn agents_path(&self) -> PathBuf {
        self.workspace_path.join(AGENTS_FILENAME)
    }

    pub fn soul_path(&self) -> PathBuf {
        self.workspace_path.join(SOUL_FILENAME)
    }

    pub fn user_path(&self) -> PathBuf {
        self.workspace_path.join(USER_FILENAME)
    }

    pub fn memory_path(&self) -> PathBuf {
        self.workspace_path.join(MEMORY_FILENAME)
    }

    pub fn daily_memory_path(&self, date: NaiveDate) -> PathBuf {
        self.memory_dir().join(format!("{}.md", date.format("%Y-%m-%d")))
    }

    pub fn today_memory_path(&self) -> PathBuf {
        self.daily_memory_path(Local::now().date_naive())
    }

    pub fn initialize(&self) -> Result<()> {
        fs::create_dir_all(self.workspace_path())?;
        fs::create_dir_all(self.memory_dir())?;
        
        if !self.agents_path().exists() {
            self.create_default_agents()?;
        }
        if !self.soul_path().exists() {
            self.create_default_soul()?;
        }
        if !self.user_path().exists() {
            self.create_default_user()?;
        }
        if !self.memory_path().exists() {
            self.create_empty_memory()?;
        }
        
        Ok(())
    }

    fn create_default_agents(&self) -> Result<()> {
        let content = r#"# AGENTS.md - 智能体操作指南

## 记忆策略
- 每次启动先读取 SOUL.md、USER.md、MEMORY.md
- 每天结束前将重要信息记录到当日记忆文件
- 定期将重要记忆汇总到 MEMORY.md

## 行为规范
- 涉及删除、修改数据的操作必须先确认
- 遇到不确定的问题时先询问用户
- 重要信息要写入长期记忆 MEMORY.md

## 工具使用
- 使用工具时要记录结果
- 遇到错误要分析原因并记录
"#;
        fs::write(self.agents_path(), content)?;
        Ok(())
    }

    fn create_default_soul(&self) -> Result<()> {
        let content = format!(r#"# SOUL.md - 智能体个性设定

## 角色
我是你的 AI 助手，致力于帮助你解决问题。

## 语气
- 友好、专业、乐于助人
- 保持简洁明了

## 边界
- 不执行有害操作
- 不泄露敏感信息
- 遇到不确定情况会主动询问

## 名称
{}
"#, &self.agent_id);
        fs::write(self.soul_path(), content)?;
        Ok(())
    }

    fn create_default_user(&self) -> Result<()> {
        let content = r#"# USER.md - 用户信息

## 用户说明
（请在此处填写用户的相关信息）

## 偏好设置
- 沟通方式: 
- 响应详细程度: 
- 其他偏好: 

## 重要日期
- 

## 注意事项
（任何需要特别关注的用户习惯或要求）
"#;
        fs::write(self.user_path(), content)?;
        Ok(())
    }

    fn create_empty_memory(&self) -> Result<()> {
        let content = format!(r#"# MEMORY.md - 长期记忆

## 关于
这是 {} 的长期记忆文件，记录重要的学习经验和解决方案。

## 问题与解决方案
（记录遇到的问题和对应的解决方案）

## 学习笔记
（记录重要的学习内容）

## 用户偏好
（记录用户的偏好和习惯）

"#, self.agent_id);
        fs::write(self.memory_path(), content)?;
        Ok(())
    }

    pub fn read_agents(&self) -> Result<String> {
        Ok(fs::read_to_string(self.agents_path())?)
    }

    pub fn read_soul(&self) -> Result<String> {
        Ok(fs::read_to_string(self.soul_path())?)
    }

    pub fn read_user(&self) -> Result<String> {
        Ok(fs::read_to_string(self.user_path())?)
    }

    pub fn read_memory(&self) -> Result<String> {
        Ok(fs::read_to_string(self.memory_path())?)
    }

    pub fn read_today_memory(&self) -> Result<String> {
        let path = self.today_memory_path();
        if path.exists() {
            Ok(fs::read_to_string(path)?)
        } else {
            Ok(String::new())
        }
    }

    pub fn write_to_today(&self, content: &str) -> Result<()> {
        let path = self.today_memory_path();
        if path.exists() {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&path)?;
            writeln!(file, "\n{}", content)?;
        } else {
            let header = format!(
                "# {} - 记忆\n\n",
                Local::now().date_naive().format("%Y-%m-%d")
            );
            fs::write(&path, format!("{}{}", header, content))?;
        }
        Ok(())
    }

    pub fn append_problem_solution(
        &self,
        problem: &str,
        solution: &str,
        context: Option<&str>,
        outcome: &str,
    ) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!(
            r#"## {} - 问题与解决

**问题**: {}

**解决方案**: {}

**结果**: {}

{}
---
"#,
            timestamp,
            problem,
            solution,
            outcome,
            context.map(|c| format!("**上下文**: {}", c)).unwrap_or_default()
        );
        
        self.write_to_today(&entry)
    }

    pub fn append_learning(&self, content: &str) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!(
            r#"## {} - 学习笔记

{}

---
"#,
            timestamp,
            content
        );
        
        self.write_to_today(&entry)
    }

    pub fn append_improvement(&self, what: &str, how: &str) -> Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let entry = format!(
            r#"## {} - 改进

**改进内容**: {}

**改进方法**: {}

---
"#,
            timestamp,
            what,
            how
        );
        
        self.write_to_today(&entry)
    }

    pub fn consolidate_to_memory(&self, days: Option<u32>) -> Result<String> {
        let days = days.unwrap_or(7);
        let cutoff = Local::now().date_naive() - chrono::Duration::days(days as i64);
        
        let memory_dir = self.memory_dir();
        let mut consolidated = String::new();
        
        if let Ok(entries) = fs::read_dir(memory_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
                            if date >= cutoff {
                                let content = fs::read_to_string(&path)?;
                                consolidated.push_str(&content);
                                consolidated.push_str("\n\n");
                            }
                        }
                    }
                }
            }
        }
        
        Ok(consolidated)
    }

    pub fn merge_to_memory(&self, content: &str) -> Result<()> {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(self.memory_path())?;
        
        writeln!(file, "\n{}", content)?;
        
        Ok(())
    }

    pub fn get_context_for_prompt(&self) -> Result<String> {
        let mut context = String::new();
        
        if self.soul_path().exists() {
            context.push_str(&format!("\n## SOUL (个性设定)\n{}\n", self.read_soul()?));
        }
        
        if self.user_path().exists() {
            context.push_str(&format!("\n## USER (用户信息)\n{}\n", self.read_user()?));
        }
        
        if self.memory_path().exists() {
            context.push_str(&format!("\n## MEMORY (长期记忆)\n{}\n", self.read_memory()?));
        }
        
        if self.today_memory_path().exists() {
            let today = self.read_today_memory()?;
            if !today.is_empty() {
                context.push_str(&format!("\n## 今日记忆\n{}\n", today));
            }
        }
        
        if self.agents_path().exists() {
            context.push_str(&format!("\n## AGENTS (操作指南)\n{}\n", self.read_agents()?));
        }
        
        Ok(context)
    }

    pub fn list_daily_memories(&self, limit: Option<usize>) -> Result<Vec<DailyMemory>> {
        let mut memories = Vec::new();
        let memory_dir = self.memory_dir();
        
        if let Ok(entries) = fs::read_dir(memory_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") {
                            let preview = fs::read_to_string(&path)
                                .map(|c| c.chars().take(200).collect())
                                .unwrap_or_default();
                            
                            memories.push(DailyMemory {
                                date,
                                path,
                                preview,
                            });
                        }
                    }
                }
            }
        }
        
        memories.sort_by(|a, b| b.date.cmp(&a.date));
        
        if let Some(n) = limit {
            memories.truncate(n);
        }
        
        Ok(memories)
    }
}

#[derive(Debug, Clone)]
pub struct DailyMemory {
    pub date: NaiveDate,
    pub path: PathBuf,
    pub preview: String,
}

pub fn create_workspace(agent_id: &str, base_path: &Path) -> Result<AgentWorkspace> {
    let workspace_path = base_path.join(agent_id);
    let workspace = AgentWorkspace::new(agent_id.to_string(), workspace_path);
    workspace.initialize()?;
    Ok(workspace)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LearningTrigger {
    OnError,
    OnUserFeedback,
    OnUncertainty,
    OnSuccess,
    All,
}

impl Default for LearningTrigger {
    fn default() -> Self {
        Self::OnError
    }
}

#[derive(Debug, Clone)]
pub struct LearningRecord {
    pub trigger: LearningTrigger,
    pub problem: String,
    pub solution: String,
    pub context: String,
    pub outcome: String,
    pub tags: Vec<String>,
}

pub struct AutoLearner {
    workspace: AgentWorkspace,
    enabled_triggers: Vec<LearningTrigger>,
    auto_consolidate: bool,
    consolidate_interval_days: u32,
}

impl AutoLearner {
    pub fn new(workspace: AgentWorkspace) -> Self {
        Self {
            workspace,
            enabled_triggers: vec![LearningTrigger::OnError],
            auto_consolidate: true,
            consolidate_interval_days: 7,
        }
    }

    pub fn with_triggers(mut self, triggers: Vec<LearningTrigger>) -> Self {
        self.enabled_triggers = triggers;
        self
    }

    pub fn with_auto_consolidate(mut self, enabled: bool, days: u32) -> Self {
        self.auto_consolidate = enabled;
        self.consolidate_interval_days = days;
        self
    }

    pub fn is_trigger_enabled(&self, trigger: LearningTrigger) -> bool {
        self.enabled_triggers.contains(&trigger)
    }

    pub fn record_error(&self, error: &str, solution: &str, context: Option<&str>) -> Result<()> {
        if !self.is_trigger_enabled(LearningTrigger::OnError) {
            return Ok(());
        }

        self.workspace.append_problem_solution(
            error,
            solution,
            context,
            "已解决",
        )
    }

    pub fn record_feedback(&self, feedback: &str, adjustment: &str) -> Result<()> {
        if !self.is_trigger_enabled(LearningTrigger::OnUserFeedback) {
            return Ok(());
        }

        self.workspace.append_improvement(feedback, adjustment)
    }

    pub fn record_uncertainty(&self, question: &str, answer: &str) -> Result<()> {
        if !self.is_trigger_enabled(LearningTrigger::OnUncertainty) {
            return Ok(());
        }

        self.workspace.append_learning(&format!(
            "**不确定问题**: {}\n**答案**: {}",
            question, answer
        ))
    }

    pub fn record_success(&self, what: &str, how: &str) -> Result<()> {
        if !self.is_trigger_enabled(LearningTrigger::OnSuccess) {
            return Ok(());
        }

        self.workspace.append_learning(&format!(
            "**成功经验**: {}\n**方法**: {}",
            what, how
        ))
    }

    pub fn consolidate(&self) -> Result<String> {
        let content = self.workspace.consolidate_to_memory(Some(self.consolidate_interval_days))?;
        
        if !content.is_empty() {
            self.workspace.merge_to_memory(&content)?;
        }
        
        Ok(content)
    }

    pub fn get_context(&self) -> Result<String> {
        self.workspace.get_context_for_prompt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_workspace_creation() {
        let temp_dir = env::temp_dir().join("openclaw_test_workspace");
        let workspace = AgentWorkspace::new("test-agent".to_string(), temp_dir.clone());
        
        let result = workspace.initialize();
        assert!(result.is_ok() || result.is_err());
        
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_daily_memory_path() {
        let workspace = AgentWorkspace::new("test".to_string(), PathBuf::from("/tmp/test"));
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let path = workspace.daily_memory_path(date);
        
        assert!(path.to_string_lossy().contains("2024-01-15.md"));
    }

    #[test]
    fn test_auto_learner_default_triggers() {
        let workspace = AgentWorkspace::new("test".to_string(), PathBuf::from("/tmp/test"));
        let learner = AutoLearner::new(workspace);
        
        assert!(learner.is_trigger_enabled(LearningTrigger::OnError));
        assert!(!learner.is_trigger_enabled(LearningTrigger::OnUserFeedback));
    }

    #[test]
    fn test_auto_learner_custom_triggers() {
        let workspace = AgentWorkspace::new("test".to_string(), PathBuf::from("/tmp/test"));
        let learner = AutoLearner::new(workspace)
            .with_triggers(vec![LearningTrigger::OnError, LearningTrigger::OnSuccess]);
        
        assert!(learner.is_trigger_enabled(LearningTrigger::OnError));
        assert!(learner.is_trigger_enabled(LearningTrigger::OnSuccess));
        assert!(!learner.is_trigger_enabled(LearningTrigger::OnUserFeedback));
    }
}
