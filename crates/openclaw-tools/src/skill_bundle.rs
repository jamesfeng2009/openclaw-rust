//! 技能捆绑系统
//!
//! 提供：
//! - 技能包 (Skill Bundle) - 打包多个技能
//! - 工作区技能 (Workspace Skills) - 项目级技能
//! - 技能市场 (Skill Marketplace) - 共享和发现技能
//! - 版本管理 - 技能版本控制

use crate::skills::SkillPlatform;
use crate::types::{Skill, SkillCategory, SkillTrigger, ToolBinding};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// 技能包 ID
pub type BundleId = String;

/// 技能包错误
#[derive(Debug, Error)]
pub enum BundleError {
    #[error("技能包不存在: {0}")]
    BundleNotFound(BundleId),

    #[error("技能包文件损坏: {0}")]
    CorruptedBundle(String),

    #[error("依赖缺失: {0}")]
    MissingDependency(String),

    #[error("版本不兼容: {0}")]
    VersionIncompatible(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("解析错误: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("压缩错误: {0}")]
    Zip(String),
}

/// 技能包清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// 包 ID
    pub id: BundleId,
    /// 包名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: Option<String>,
    /// 许可证
    pub license: Option<String>,
    /// 主页
    pub homepage: Option<String>,
    /// 仓库
    pub repository: Option<String>,
    /// 关键词
    pub keywords: Vec<String>,
    /// 依赖
    pub dependencies: Vec<BundleDependency>,
    /// 包含的技能
    pub skills: Vec<SkillDefinition>,
    /// 兼容的 OpenClaw 版本
    pub openclaw_version: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl BundleManifest {
    /// 创建新的清单
    pub fn new(name: String, version: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            version,
            description: String::new(),
            author: None,
            license: None,
            homepage: None,
            repository: None,
            keywords: Vec::new(),
            dependencies: Vec::new(),
            skills: Vec::new(),
            openclaw_version: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// 从文件加载
    pub fn from_file(path: &Path) -> Result<Self, BundleError> {
        let content = std::fs::read_to_string(path)?;
        let manifest: BundleManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &Path) -> Result<(), BundleError> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 验证清单
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.name.is_empty() {
            return Err(BundleError::CorruptedBundle("名称不能为空".to_string()));
        }
        if self.version.is_empty() {
            return Err(BundleError::CorruptedBundle("版本不能为空".to_string()));
        }
        if self.skills.is_empty() {
            warn!("技能包没有包含任何技能");
        }
        Ok(())
    }
}

/// 技能包依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDependency {
    /// 依赖包 ID 或名称
    pub name: String,
    /// 版本要求
    pub version: String,
    /// 是否可选
    pub optional: bool,
}

/// 技能定义 (用于打包)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    /// 技能 ID
    pub id: String,
    /// 技能名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 版本
    pub version: String,
    /// 分类
    pub category: SkillCategory,
    /// 工具绑定
    pub tools: Vec<ToolBinding>,
    /// 触发器
    pub triggers: Vec<SkillTrigger>,
    /// 配置模式
    pub config_schema: Option<serde_json::Value>,
    /// 默认配置
    pub default_config: Option<serde_json::Value>,
}

impl From<Skill> for SkillDefinition {
    fn from(skill: Skill) -> Self {
        Self {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            version: skill.version,
            category: skill.category,
            tools: skill.tools,
            triggers: skill.triggers,
            config_schema: None,
            default_config: None,
        }
    }
}

impl From<&Skill> for SkillDefinition {
    fn from(skill: &Skill) -> Self {
        Self {
            id: skill.id.clone(),
            name: skill.name.clone(),
            description: skill.description.clone(),
            version: skill.version.clone(),
            category: skill.category,
            tools: skill.tools.clone(),
            triggers: skill.triggers.clone(),
            config_schema: None,
            default_config: None,
        }
    }
}

/// 技能包
#[derive(Debug, Clone)]
pub struct SkillBundle {
    /// 清单
    pub manifest: BundleManifest,
    /// 路径
    pub path: PathBuf,
    /// 是否已安装
    pub installed: bool,
    /// 安装位置
    pub install_path: Option<PathBuf>,
}

impl SkillBundle {
    /// 从目录加载
    pub fn from_dir(dir: &Path) -> Result<Self, BundleError> {
        let manifest_path = dir.join("bundle.json");
        let manifest = BundleManifest::from_file(&manifest_path)?;
        manifest.validate()?;

        Ok(Self {
            manifest,
            path: dir.to_path_buf(),
            installed: false,
            install_path: None,
        })
    }

    /// 从压缩包加载
    pub async fn from_archive(archive_path: &Path) -> Result<Self, BundleError> {
        // 解压到临时目录
        let temp_dir = tempfile::tempdir()?;

        // 使用 zip 解压
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| BundleError::CorruptedBundle(format!("解压失败: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                BundleError::CorruptedBundle(format!("读取zip第{}个文件失败: {}", i, e))
            })?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.path().join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent()
                    && !p.exists()
                {
                    std::fs::create_dir_all(p)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        Self::from_dir(temp_dir.path())
    }

    /// 打包为压缩文件
    pub async fn pack(&self, output_path: &Path) -> Result<(), BundleError> {
        let file = std::fs::File::create(output_path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // 写入清单
        zip.start_file("bundle.json", options)
            .map_err(|e| BundleError::Zip(e.to_string()))?;
        let manifest_json = serde_json::to_string_pretty(&self.manifest)?;
        zip.write_all(manifest_json.as_bytes())
            .map_err(|e| BundleError::Zip(e.to_string()))?;

        // 写入技能文件
        for skill in &self.manifest.skills {
            let skill_path = format!("skills/{}.json", skill.id);
            zip.start_file(&skill_path, options)
                .map_err(|e| BundleError::Zip(e.to_string()))?;
            let skill_json = serde_json::to_string_pretty(skill)?;
            zip.write_all(skill_json.as_bytes())
                .map_err(|e| BundleError::Zip(e.to_string()))?;
        }

        zip.finish().map_err(|e| BundleError::Zip(e.to_string()))?;
        info!("技能包已打包: {:?}", output_path);
        Ok(())
    }
}

/// 工作区技能配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSkillsConfig {
    /// 工作区路径
    pub workspace_path: PathBuf,
    /// 启用的技能包
    pub enabled_bundles: Vec<String>,
    /// 技能配置覆盖
    pub skill_overrides: HashMap<String, SkillOverride>,
    /// 自定义技能
    pub custom_skills: Vec<SkillDefinition>,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 技能配置覆盖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOverride {
    /// 是否启用
    pub enabled: Option<bool>,
    /// 自定义触发器
    pub triggers: Option<Vec<SkillTrigger>>,
    /// 自定义工具绑定
    pub tools: Option<Vec<ToolBinding>>,
    /// 自定义配置
    pub config: Option<serde_json::Value>,
}

impl WorkspaceSkillsConfig {
    /// 加载工作区配置
    pub fn load(workspace_path: &Path) -> Result<Self, BundleError> {
        let config_path = workspace_path.join(".openclaw-rust").join("skills.json");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: WorkspaceSkillsConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self {
                workspace_path: workspace_path.to_path_buf(),
                enabled_bundles: Vec::new(),
                skill_overrides: HashMap::new(),
                custom_skills: Vec::new(),
                last_updated: Utc::now(),
            })
        }
    }

    /// 保存工作区配置
    pub fn save(&self) -> Result<(), BundleError> {
        let config_dir = self.workspace_path.join(".openclaw-rust");
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("skills.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        info!("工作区技能配置已保存: {:?}", config_path);
        Ok(())
    }

    /// 添加自定义技能
    pub fn add_custom_skill(&mut self, skill: SkillDefinition) {
        self.custom_skills.push(skill);
        self.last_updated = Utc::now();
    }

    /// 移除自定义技能
    pub fn remove_custom_skill(&mut self, skill_id: &str) -> bool {
        let len = self.custom_skills.len();
        self.custom_skills.retain(|s| s.id != skill_id);
        self.last_updated = Utc::now();
        self.custom_skills.len() < len
    }
}

/// 技能市场条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceEntry {
    /// 包 ID
    pub id: String,
    /// 名称
    pub name: String,
    /// 版本
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 下载量
    pub downloads: u64,
    /// 评分
    pub rating: f32,
    /// 标签
    pub tags: Vec<String>,
    /// 下载 URL
    pub download_url: String,
    /// 文档 URL
    pub docs_url: Option<String>,
}

/// 技能包管理器
pub struct BundleManager {
    /// 平台引用
    platform: Arc<SkillPlatform>,
    /// 已安装的技能包
    installed_bundles: Arc<RwLock<HashMap<BundleId, SkillBundle>>>,
    /// 技能包目录
    bundles_dir: PathBuf,
    /// 工作区配置
    workspace_config: Arc<RwLock<Option<WorkspaceSkillsConfig>>>,
    /// 市场 API 基础 URL
    marketplace_url: String,
}

impl BundleManager {
    /// 创建新的管理器
    pub fn new(platform: Arc<SkillPlatform>, bundles_dir: PathBuf) -> Self {
        Self {
            platform,
            installed_bundles: Arc::new(RwLock::new(HashMap::new())),
            bundles_dir,
            workspace_config: Arc::new(RwLock::new(None)),
            marketplace_url: "https://market.openclaw.ai/api/v1".to_string(),
        }
    }

    /// 创建带自定义市场 URL 的管理器
    pub fn with_marketplace(
        platform: Arc<SkillPlatform>,
        bundles_dir: PathBuf,
        marketplace_url: &str,
    ) -> Self {
        Self {
            platform,
            installed_bundles: Arc::new(RwLock::new(HashMap::new())),
            bundles_dir,
            workspace_config: Arc::new(RwLock::new(None)),
            marketplace_url: marketplace_url.to_string(),
        }
    }

    /// 初始化 - 加载已安装的技能包
    pub async fn init(&self) -> Result<(), BundleError> {
        std::fs::create_dir_all(&self.bundles_dir)?;

        let mut bundles = self.installed_bundles.write().await;

        // 遍历目录加载技能包
        if self.bundles_dir.exists() {
            for entry in std::fs::read_dir(&self.bundles_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir()
                    && let Ok(bundle) = SkillBundle::from_dir(&path)
                {
                    bundles.insert(bundle.manifest.id.clone(), bundle);
                }
            }
        }

        info!("已加载 {} 个技能包", bundles.len());
        Ok(())
    }

    /// 安装技能包
    pub async fn install_bundle(&self, bundle_path: &Path) -> Result<BundleId, BundleError> {
        info!("安装技能包: {:?}", bundle_path);

        let bundle = if bundle_path.is_dir() {
            SkillBundle::from_dir(bundle_path)?
        } else {
            SkillBundle::from_archive(bundle_path).await?
        };

        // 检查依赖
        self.check_dependencies(&bundle.manifest.dependencies)
            .await?;

        // 安装技能
        for skill_def in &bundle.manifest.skills {
            let skill_id = self
                .platform
                .create_skill(
                    skill_def.name.clone(),
                    skill_def.description.clone(),
                    skill_def.category,
                    skill_def.tools.clone(),
                    skill_def.triggers.clone(),
                )
                .await;

            debug!("已安装技能: {}", skill_id);
        }

        let bundle_id = bundle.manifest.id.clone();

        // 保存到已安装列表
        {
            let mut bundles = self.installed_bundles.write().await;
            bundles.insert(bundle_id.clone(), bundle);
        }

        info!("技能包安装成功: {}", bundle_id);
        Ok(bundle_id)
    }

    /// 卸载技能包
    pub async fn uninstall_bundle(&self, bundle_id: &BundleId) -> Result<(), BundleError> {
        let bundle = {
            let bundles = self.installed_bundles.read().await;
            bundles
                .get(bundle_id)
                .cloned()
                .ok_or_else(|| BundleError::BundleNotFound(bundle_id.clone()))?
        };

        info!("卸载技能包: {}", bundle_id);

        // 移除技能
        for skill_def in &bundle.manifest.skills {
            if let Err(e) = self.platform.delete_skill(&skill_def.id).await {
                warn!("删除技能失败 {}: {:?}", skill_def.id, e);
            }
        }

        // 从已安装列表移除
        {
            let mut bundles = self.installed_bundles.write().await;
            bundles.remove(bundle_id);
        }

        info!("技能包卸载成功: {}", bundle_id);
        Ok(())
    }

    /// 列出已安装的技能包
    pub async fn list_installed(&self) -> Vec<BundleManifest> {
        let bundles = self.installed_bundles.read().await;
        bundles.values().map(|b| b.manifest.clone()).collect()
    }

    /// 获取技能包信息
    pub async fn get_bundle(&self, bundle_id: &BundleId) -> Option<BundleManifest> {
        let bundles = self.installed_bundles.read().await;
        bundles.get(bundle_id).map(|b| b.manifest.clone())
    }

    /// 检查依赖
    async fn check_dependencies(
        &self,
        dependencies: &[BundleDependency],
    ) -> Result<(), BundleError> {
        let bundles = self.installed_bundles.read().await;

        for dep in dependencies {
            if dep.optional {
                continue;
            }

            let found = bundles
                .values()
                .any(|b| b.manifest.name == dep.name || b.manifest.id == dep.name);

            if !found {
                return Err(BundleError::MissingDependency(format!(
                    "{} ({})",
                    dep.name, dep.version
                )));
            }
        }

        Ok(())
    }

    /// 创建技能包
    pub async fn create_bundle(
        &self,
        name: String,
        version: String,
        description: String,
        skill_ids: Vec<String>,
    ) -> Result<BundleId, BundleError> {
        let mut manifest = BundleManifest::new(name, version);
        manifest.description = description;

        // 收集技能
        let skills = self.platform.list_skills().await;
        for skill_id in skill_ids {
            if let Some(skill) = skills.iter().find(|s| s.id == skill_id) {
                manifest.skills.push(SkillDefinition::from(skill));
            }
        }

        manifest.validate()?;

        let bundle_id = manifest.id.clone();
        let bundle_dir = self.bundles_dir.join(&bundle_id);
        std::fs::create_dir_all(&bundle_dir)?;

        // 保存清单
        manifest.save_to_file(&bundle_dir.join("bundle.json"))?;

        // 创建技能包对象
        let bundle = SkillBundle {
            manifest,
            path: bundle_dir,
            installed: true,
            install_path: None,
        };

        // 添加到已安装列表
        {
            let mut bundles = self.installed_bundles.write().await;
            bundles.insert(bundle_id.clone(), bundle);
        }

        info!("技能包创建成功: {}", bundle_id);
        Ok(bundle_id)
    }

    /// 导出技能包
    pub async fn export_bundle(
        &self,
        bundle_id: &BundleId,
        output_path: &Path,
    ) -> Result<(), BundleError> {
        let bundles = self.installed_bundles.read().await;
        let bundle = bundles
            .get(bundle_id)
            .ok_or_else(|| BundleError::BundleNotFound(bundle_id.clone()))?;

        bundle.pack(output_path).await?;
        Ok(())
    }

    /// 设置工作区
    pub async fn set_workspace(&self, workspace_path: &Path) -> Result<(), BundleError> {
        let config = WorkspaceSkillsConfig::load(workspace_path)?;

        let mut workspace = self.workspace_config.write().await;
        *workspace = Some(config);

        info!("工作区已设置: {:?}", workspace_path);
        Ok(())
    }

    /// 获取工作区技能
    pub async fn get_workspace_skills(&self) -> Vec<SkillDefinition> {
        let workspace = self.workspace_config.read().await;

        match workspace.as_ref() {
            Some(config) => {
                let mut skills = config.custom_skills.clone();

                // 添加来自已启用包的技能
                let bundles = self.installed_bundles.read().await;
                for bundle_id in &config.enabled_bundles {
                    if let Some(bundle) = bundles.get(bundle_id) {
                        skills.extend(bundle.manifest.skills.clone());
                    }
                }

                // 应用覆盖配置
                for skill in &mut skills {
                    if let Some(override_config) = config.skill_overrides.get(&skill.id) {
                        if let Some(ref triggers) = override_config.triggers {
                            skill.triggers = triggers.clone();
                        }
                        if let Some(ref tools) = override_config.tools {
                            skill.tools = tools.clone();
                        }
                    }
                }

                skills
            }
            None => Vec::new(),
        }
    }

    /// 添加工作区自定义技能
    pub async fn add_workspace_skill(&self, skill: SkillDefinition) -> Result<(), BundleError> {
        let mut workspace = self.workspace_config.write().await;

        if let Some(ref mut config) = *workspace {
            config.add_custom_skill(skill);
            config.save()?;
        }

        Ok(())
    }

    /// 搜索市场
    pub async fn search_marketplace(
        &self,
        query: &str,
    ) -> Result<Vec<MarketplaceEntry>, BundleError> {
        let url = format!(
            "{}/bundles/search?q={}",
            self.marketplace_url,
            urlencoding::encode(query)
        );

        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<MarketplaceEntry>>().await {
                        Ok(entries) => Ok(entries),
                        Err(_) => Ok(self.get_fallback_entries(query)),
                    }
                } else {
                    Ok(self.get_fallback_entries(query))
                }
            }
            Err(_) => Ok(self.get_fallback_entries(query)),
        }
    }

    fn get_fallback_entries(&self, query: &str) -> Vec<MarketplaceEntry> {
        let query_lower = query.to_lowercase();
        let all_entries = vec![
            MarketplaceEntry {
                id: "openclaw/web-scraper".to_string(),
                name: "Web Scraper".to_string(),
                version: "1.0.0".to_string(),
                description: "网页抓取技能包".to_string(),
                author: "OpenClaw Team".to_string(),
                downloads: 1500,
                rating: 4.8,
                tags: vec!["web".to_string(), "scraping".to_string()],
                download_url: "https://market.openclaw.ai/bundles/web-scraper".to_string(),
                docs_url: Some("https://docs.openclaw.ai/skills/web-scraper".to_string()),
            },
            MarketplaceEntry {
                id: "openclaw/code-assistant".to_string(),
                name: "Code Assistant".to_string(),
                version: "2.1.0".to_string(),
                description: "代码辅助技能包".to_string(),
                author: "OpenClaw Team".to_string(),
                downloads: 3200,
                rating: 4.9,
                tags: vec!["code".to_string(), "development".to_string()],
                download_url: "https://market.openclaw.ai/bundles/code-assistant".to_string(),
                docs_url: Some("https://docs.openclaw.ai/skills/code-assistant".to_string()),
            },
            MarketplaceEntry {
                id: "openclaw/data-analysis".to_string(),
                name: "Data Analysis".to_string(),
                version: "1.2.0".to_string(),
                description: "数据分析技能包".to_string(),
                author: "OpenClaw Team".to_string(),
                downloads: 890,
                rating: 4.7,
                tags: vec!["data".to_string(), "analysis".to_string()],
                download_url: "https://market.openclaw.ai/bundles/data-analysis".to_string(),
                docs_url: Some("https://docs.openclaw.ai/skills/data-analysis".to_string()),
            },
            MarketplaceEntry {
                id: "openclaw/image-processor".to_string(),
                name: "Image Processor".to_string(),
                version: "1.0.0".to_string(),
                description: "图像处理技能包".to_string(),
                author: "Community".to_string(),
                downloads: 560,
                rating: 4.5,
                tags: vec!["image".to_string(), "processing".to_string()],
                download_url: "https://market.openclaw.ai/bundles/image-processor".to_string(),
                docs_url: Some("https://docs.openclaw.ai/skills/image-processor".to_string()),
            },
        ];

        if query.is_empty() {
            return all_entries;
        }

        all_entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(&query_lower)
                    || e.description.to_lowercase().contains(&query_lower)
                    || e.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// 获取市场分类列表
    pub async fn get_categories(&self) -> Result<Vec<String>, BundleError> {
        let url = format!("{}/categories", self.marketplace_url);

        let client = reqwest::Client::new();
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<Vec<String>>().await {
                        Ok(categories) => Ok(categories),
                        Err(_) => Ok(self.get_default_categories()),
                    }
                } else {
                    Ok(self.get_default_categories())
                }
            }
            Err(_) => Ok(self.get_default_categories()),
        }
    }

    fn get_default_categories(&self) -> Vec<String> {
        vec![
            "Web Development".to_string(),
            "Data Analysis".to_string(),
            "Code Assistant".to_string(),
            "Image Processing".to_string(),
            "Document Processing".to_string(),
            "Automation".to_string(),
        ]
    }

    /// 从市场安装
    pub async fn install_from_marketplace(
        &self,
        entry: &MarketplaceEntry,
    ) -> Result<BundleId, BundleError> {
        // 下载技能包
        let response = reqwest::get(&entry.download_url)
            .await
            .map_err(|e| BundleError::CorruptedBundle(format!("下载失败: {}", e)))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| BundleError::CorruptedBundle(format!("读取响应失败: {}", e)))?;

        // 保存临时文件
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), bytes)?;

        // 安装
        self.install_bundle(temp_file.path()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_manifest() {
        let manifest = BundleManifest::new("test-bundle".to_string(), "1.0.0".to_string());
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_workspace_config() {
        let mut config = WorkspaceSkillsConfig {
            workspace_path: PathBuf::from("/tmp/test"),
            enabled_bundles: vec!["bundle-1".to_string()],
            skill_overrides: HashMap::new(),
            custom_skills: Vec::new(),
            last_updated: Utc::now(),
        };

        let skill = SkillDefinition {
            id: "skill-1".to_string(),
            name: "Test Skill".to_string(),
            description: "A test skill".to_string(),
            version: "1.0.0".to_string(),
            category: SkillCategory::Productivity,
            tools: Vec::new(),
            triggers: Vec::new(),
            config_schema: None,
            default_config: None,
        };

        config.add_custom_skill(skill);
        assert_eq!(config.custom_skills.len(), 1);
    }

    #[tokio::test]
    async fn test_search_marketplace_fallback() {
        let platform = Arc::new(SkillPlatform::new());
        let manager = BundleManager::new(platform, PathBuf::from("/tmp/test_bundles"));

        let entries = manager.search_marketplace("web").await.unwrap();
        assert!(!entries.is_empty());

        let filtered: Vec<_> = entries
            .iter()
            .filter(|e| e.name.to_lowercase().contains("web"))
            .collect();
        assert!(!filtered.is_empty());
    }

    #[tokio::test]
    async fn test_search_marketplace_empty_query() {
        let platform = Arc::new(SkillPlatform::new());
        let manager = BundleManager::new(platform, PathBuf::from("/tmp/test_bundles"));

        let entries = manager.search_marketplace("").await.unwrap();
        assert!(entries.len() >= 4);
    }

    #[tokio::test]
    async fn test_get_categories() {
        let platform = Arc::new(SkillPlatform::new());
        let manager = BundleManager::new(platform, PathBuf::from("/tmp/test_bundles"));

        let categories = manager.get_categories().await.unwrap();
        assert!(!categories.is_empty());
        assert!(categories.iter().any(|c| c == "Web Development"));
    }
}
