use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRegistry {
    pub bundled_skills: Vec<SkillMetadata>,
    pub managed_skills: Vec<SkillMetadata>,
    pub workspace_skills: Vec<SkillMetadata>,
    pub clawhub_skills: HashMap<String, ClawHubSkill>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub category: SkillType,
    pub tags: Vec<String>,
    pub enabled: bool,
    pub source: SkillSource,
    pub installed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillSource {
    Bundled,
    Managed,
    Workspace,
    ClawHub,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillType {
    Productivity,
    Automation,
    Analysis,
    Communication,
    Development,
    Media,
    Security,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClawHubSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub downloads: u32,
    pub rating: f32,
    pub tags: Vec<String>,
    pub manifest_url: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub entry_point: String,
    pub dependencies: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedSkill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source_url: String,
    pub config: HashMap<String, serde_json::Value>,
    pub status: ManagedSkillStatus,
    pub last_sync: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ManagedSkillStatus {
    Active,
    Outdated,
    Error,
    Disabled,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            bundled_skills: Self::default_bundled_skills(),
            managed_skills: Vec::new(),
            workspace_skills: Vec::new(),
            clawhub_skills: HashMap::new(),
        }
    }

    fn default_bundled_skills() -> Vec<SkillMetadata> {
        vec![
            SkillMetadata {
                id: "builtin.file_ops".to_string(),
                name: "文件操作".to_string(),
                description: "读取、写入、复制、移动文件和目录".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Productivity,
                tags: vec!["文件".to_string(), "IO".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.web_search".to_string(),
                name: "网页搜索".to_string(),
                description: "使用搜索引擎查找信息".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Analysis,
                tags: vec!["搜索".to_string(), "网络".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.image_gen".to_string(),
                name: "图像生成".to_string(),
                description: "使用 AI 生成图像".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Media,
                tags: vec!["图像".to_string(), "AI".to_string(), "生成".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.code_analyze".to_string(),
                name: "代码分析".to_string(),
                description: "分析代码结构、检测问题、优化建议".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Development,
                tags: vec!["代码".to_string(), "分析".to_string(), "开发".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.data_process".to_string(),
                name: "数据处理".to_string(),
                description: "处理和分析结构化数据".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Analysis,
                tags: vec!["数据".to_string(), "处理".to_string(), "分析".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.automation".to_string(),
                name: "自动化任务".to_string(),
                description: "创建和执行自动化工作流".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Automation,
                tags: vec!["自动化".to_string(), "工作流".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
            SkillMetadata {
                id: "builtin.safe_execute".to_string(),
                name: "安全执行".to_string(),
                description: "在沙箱环境中安全执行代码".to_string(),
                version: "1.0.0".to_string(),
                author: Some("OpenClaw".to_string()),
                category: SkillType::Security,
                tags: vec!["安全".to_string(), "沙箱".to_string(), "执行".to_string()],
                enabled: true,
                source: SkillSource::Bundled,
                installed_at: Some(Utc::now()),
            },
        ]
    }

    pub fn get_all_skills(&self) -> Vec<&SkillMetadata> {
        let mut skills = Vec::new();
        skills.extend(&self.bundled_skills);
        skills.extend(&self.managed_skills);
        skills.extend(&self.workspace_skills);
        skills
    }

    pub fn get_enabled_skills(&self) -> Vec<&SkillMetadata> {
        self.get_all_skills()
            .into_iter()
            .filter(|s| s.enabled)
            .collect()
    }

    pub fn enable_skill(&mut self, id: &str) -> Result<(), String> {
        if let Some(skill) = self.bundled_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = true;
            return Ok(());
        }
        if let Some(skill) = self.managed_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = true;
            return Ok(());
        }
        if let Some(skill) = self.workspace_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = true;
            return Ok(());
        }
        Err(format!("技能 {} 不存在", id))
    }

    pub fn disable_skill(&mut self, id: &str) -> Result<(), String> {
        if let Some(skill) = self.bundled_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = false;
            return Ok(());
        }
        if let Some(skill) = self.managed_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = false;
            return Ok(());
        }
        if let Some(skill) = self.workspace_skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = false;
            return Ok(());
        }
        Err(format!("技能 {} 不存在", id))
    }

    pub fn add_workspace_skill(&mut self, skill: SkillMetadata) {
        self.workspace_skills.push(skill);
    }

    pub fn add_managed_skill(&mut self, skill: SkillMetadata) {
        self.managed_skills.push(skill);
    }

    pub fn remove_skill(&mut self, id: &str) -> Result<(), String> {
        if let Some(pos) = self.managed_skills.iter().position(|s| s.id == id) {
            self.managed_skills.remove(pos);
            return Ok(());
        }
        if let Some(pos) = self.workspace_skills.iter().position(|s| s.id == id) {
            self.workspace_skills.remove(pos);
            return Ok(());
        }
        Err(format!("技能 {} 不存在或无法删除", id))
    }

    pub fn fetch_clawhub_skills(&mut self) {
        self.clawhub_skills.insert(
            "clawhub.web_scraper".to_string(),
            ClawHubSkill {
                id: "clawhub.web_scraper".to_string(),
                name: "网页抓取".to_string(),
                description: "高效抓取网页内容".to_string(),
                version: "1.2.0".to_string(),
                author: "Community".to_string(),
                downloads: 1523,
                rating: 4.5,
                tags: vec!["爬虫".to_string(), "网页".to_string()],
                manifest_url: "https://clawhub.example.com/skills/web_scraper/manifest.json".to_string(),
                icon_url: None,
            },
        );
        self.clawhub_skills.insert(
            "clawhub.pdf_tool".to_string(),
            ClawHubSkill {
                id: "clawhub.pdf_tool".to_string(),
                name: "PDF 工具".to_string(),
                description: "PDF 创建、编辑和转换".to_string(),
                version: "2.0.1".to_string(),
                author: "Community".to_string(),
                downloads: 892,
                rating: 4.2,
                tags: vec!["PDF".to_string(), "文档".to_string()],
                manifest_url: "https://clawhub.example.com/skills/pdf_tool/manifest.json".to_string(),
                icon_url: None,
            },
        );
        self.clawhub_skills.insert(
            "clawhub.ocr".to_string(),
            ClawHubSkill {
                id: "clawhub.ocr".to_string(),
                name: "OCR 文字识别".to_string(),
                description: "从图像中提取文字".to_string(),
                version: "1.5.0".to_string(),
                author: "Community".to_string(),
                downloads: 2341,
                rating: 4.8,
                tags: vec!["OCR".to_string(), "文字识别".to_string(), "图像".to_string()],
                manifest_url: "https://clawhub.example.com/skills/ocr/manifest.json".to_string(),
                icon_url: None,
            },
        );
    }

    pub fn get_clawhub_skills(&self) -> Vec<&ClawHubSkill> {
        self.clawhub_skills.values().collect()
    }

    pub fn install_clawhub_skill(&mut self, skill_id: &str) -> Result<SkillMetadata, String> {
        let clawhub_skill = self.clawhub_skills.get(skill_id)
            .ok_or_else(|| format!("ClawHub 技能 {} 不存在", skill_id))?;

        let metadata = SkillMetadata {
            id: clawhub_skill.id.clone(),
            name: clawhub_skill.name.clone(),
            description: clawhub_skill.description.clone(),
            version: clawhub_skill.version.clone(),
            author: Some(clawhub_skill.author.clone()),
            category: SkillType::Custom,
            tags: clawhub_skill.tags.clone(),
            enabled: true,
            source: SkillSource::Managed,
            installed_at: Some(Utc::now()),
        };

        self.managed_skills.push(metadata.clone());
        Ok(metadata)
    }
}
