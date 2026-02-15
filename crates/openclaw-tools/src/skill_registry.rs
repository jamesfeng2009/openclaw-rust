//! 技能注册表 (ClawHub)
//!
//! 提供：
//! - 技能发现和搜索
//! - 自动安装技能
//! - 版本管理
//! - 依赖解析

use crate::skill_bundle::{BundleError, BundleId, BundleManifest, SkillBundle};
use crate::skills::SkillPlatform;
use crate::types::{Skill, SkillCategory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 注册表错误
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("网络错误: {0}")]
    Network(String),

    #[error("技能不存在: {0}")]
    SkillNotFound(String),

    #[error("版本不存在: {0}")]
    VersionNotFound(String),

    #[error("安装失败: {0}")]
    InstallFailed(String),

    #[error("依赖解析失败: {0}")]
    DependencyFailed(String),

    #[error("验证失败: {0}")]
    VerificationFailed(String),

    #[error("Bundle 错误: {0}")]
    Bundle(#[from] BundleError),
}

/// 注册表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// ClawHub API 地址
    pub registry_url: String,
    /// 本地缓存目录
    pub cache_dir: PathBuf,
    /// 技能安装目录
    pub install_dir: PathBuf,
    /// 是否自动更新
    pub auto_update: bool,
    /// 更新检查间隔 (小时)
    pub update_check_interval: u64,
    /// 是否允许预发布版本
    pub allow_prerelease: bool,
    /// 受信任的发布者
    pub trusted_publishers: Vec<String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            registry_url: "https://api.clawhub.io/v1".to_string(),
            cache_dir: PathBuf::from(format!("{}/.openclaw/cache/skills", home)),
            install_dir: PathBuf::from(format!("{}/.openclaw/skills", home)),
            auto_update: true,
            update_check_interval: 24,
            allow_prerelease: false,
            trusted_publishers: vec!["openclaw".to_string()],
        }
    }
}

/// 注册表技能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySkill {
    /// 技能 ID
    pub id: String,
    /// 名称
    pub name: String,
    /// 描述
    pub description: String,
    /// 发布者
    pub publisher: String,
    /// 最新版本
    pub latest_version: String,
    /// 可用版本列表
    pub versions: Vec<SkillVersion>,
    /// 标签
    pub tags: Vec<String>,
    /// 分类
    pub category: SkillCategory,
    /// 下载量
    pub downloads: u64,
    /// 评分
    pub rating: f32,
    /// 验证状态
    pub verified: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 技能版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub released_at: DateTime<Utc>,
    pub changelog: Option<String>,
    pub download_url: String,
    pub checksum: String,
    pub min_openclaw_version: Option<String>,
    pub deprecated: bool,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub skills: Vec<RegistrySkill>,
    pub total: usize,
    pub page: usize,
    pub per_page: usize,
}

/// 安装选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    /// 指定版本 (默认最新)
    pub version: Option<String>,
    /// 是否强制重新安装
    pub force: bool,
    /// 是否安装依赖
    pub install_deps: bool,
    /// 自定义安装路径
    pub install_path: Option<PathBuf>,
    /// 是否跳过验证
    pub skip_verification: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            version: None,
            force: false,
            install_deps: true,
            install_path: None,
            skip_verification: false,
        }
    }
}

/// 技能注册表客户端
pub struct SkillRegistry {
    config: RegistryConfig,
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, RegistrySkill>>>,
    installed: Arc<RwLock<HashMap<String, InstalledSkill>>>,
    platform: Arc<SkillPlatform>,
}

/// 已安装技能信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub id: String,
    pub version: String,
    pub installed_at: DateTime<Utc>,
    pub install_path: PathBuf,
    pub publisher: String,
    pub auto_update: bool,
}

impl SkillRegistry {
    pub fn new(config: RegistryConfig, platform: Arc<SkillPlatform>) -> Self {
        std::fs::create_dir_all(&config.cache_dir).ok();
        std::fs::create_dir_all(&config.install_dir).ok();

        Self {
            config,
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            installed: Arc::new(RwLock::new(HashMap::new())),
            platform,
        }
    }

    /// 搜索技能
    pub async fn search(&self, query: &str, page: usize, per_page: usize) -> Result<SearchResult, RegistryError> {
        let url = format!(
            "{}/skills/search?q={}&page={}&per_page={}",
            self.config.registry_url,
            urlencoding::encode(query),
            page,
            per_page
        );

        info!("搜索技能: {}", query);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(RegistryError::Network(format!(
                "搜索失败: {}",
                response.status()
            )));
        }

        let result: SearchResult = response
            .json()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        // 缓存结果
        {
            let mut cache = self.cache.write().await;
            for skill in &result.skills {
                cache.insert(skill.id.clone(), skill.clone());
            }
        }

        Ok(result)
    }

    /// 获取技能详情
    pub async fn get_skill(&self, skill_id: &str) -> Result<RegistrySkill, RegistryError> {
        // 先检查缓存
        {
            let cache = self.cache.read().await;
            if let Some(skill) = cache.get(skill_id) {
                return Ok(skill.clone());
            }
        }

        let url = format!("{}/skills/{}", self.config.registry_url, skill_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        if response.status() == 404 {
            return Err(RegistryError::SkillNotFound(skill_id.to_string()));
        }

        let skill: RegistrySkill = response
            .json()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        // 缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(skill_id.to_string(), skill.clone());
        }

        Ok(skill)
    }

    /// 安装技能
    pub async fn install(
        &self,
        skill_id: &str,
        options: InstallOptions,
    ) -> Result<InstalledSkill, RegistryError> {
        info!("安装技能: {} (版本: {:?})", skill_id, options.version);

        // 获取技能信息
        let skill = self.get_skill(skill_id).await?;

        // 检查是否已安装
        {
            let installed = self.installed.read().await;
            if let Some(existing) = installed.get(skill_id) {
                if !options.force {
                    info!("技能 {} 已安装 (版本 {})", skill_id, existing.version);
                    return Ok(existing.clone());
                }
            }
        }

        // 确定版本
        let version = options
            .version
            .clone()
            .unwrap_or_else(|| skill.latest_version.clone());

        let version_info = skill
            .versions
            .iter()
            .find(|v| v.version == version)
            .ok_or_else(|| RegistryError::VersionNotFound(version.clone()))?;

        // 验证发布者
        if !options.skip_verification && !self.config.trusted_publishers.contains(&skill.publisher) {
            warn!("技能发布者 {} 不在信任列表中", skill.publisher);
        }

        // 下载技能包
        let bundle_path = self.download_bundle(&version_info.download_url, skill_id, &version).await?;

        // 验证校验和
        if !options.skip_verification {
            self.verify_checksum(&bundle_path, &version_info.checksum)?;
        }

        // 安装路径
        let install_path = options.install_path.clone().unwrap_or_else(|| {
            self.config.install_dir.join(skill_id)
        });

        // 解压并安装
        let bundle = self.extract_bundle(&bundle_path, &install_path).await?;

        // 安装依赖
        if options.install_deps {
            self.install_dependencies(&bundle.manifest).await?;
        }

        // 注册到平台
        self.register_to_platform(&bundle).await?;

        // 记录安装
        let installed_skill = InstalledSkill {
            id: skill_id.to_string(),
            version: version.clone(),
            installed_at: Utc::now(),
            install_path: install_path.clone(),
            publisher: skill.publisher.clone(),
            auto_update: self.config.auto_update,
        };

        {
            let mut installed = self.installed.write().await;
            installed.insert(skill_id.to_string(), installed_skill.clone());
        }

        info!("技能 {} 安装成功", skill_id);
        Ok(installed_skill)
    }

    /// 卸载技能
    pub async fn uninstall(&self, skill_id: &str) -> Result<(), RegistryError> {
        info!("卸载技能: {}", skill_id);

        // 获取安装信息
        let installed_info = {
            let installed = self.installed.read().await;
            installed
                .get(skill_id)
                .cloned()
                .ok_or_else(|| RegistryError::SkillNotFound(skill_id.to_string()))?
        };

        // 删除安装目录
        if installed_info.install_path.exists() {
            std::fs::remove_dir_all(&installed_info.install_path)
                .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;
        }

        // 从平台移除
        // TODO: 调用 platform.delete_skill

        // 移除记录
        {
            let mut installed = self.installed.write().await;
            installed.remove(skill_id);
        }

        info!("技能 {} 已卸载", skill_id);
        Ok(())
    }

    /// 更新技能
    pub async fn update(&self, skill_id: &str) -> Result<InstalledSkill, RegistryError> {
        info!("更新技能: {}", skill_id);

        // 获取最新版本
        let skill = self.get_skill(skill_id).await?;

        // 获取当前安装版本
        let current = {
            let installed = self.installed.read().await;
            installed
                .get(skill_id)
                .cloned()
                .ok_or_else(|| RegistryError::SkillNotFound(skill_id.to_string()))?
        };

        // 检查是否需要更新
        if current.version == skill.latest_version {
            info!("技能 {} 已是最新版本", skill_id);
            return Ok(current);
        }

        // 安装新版本
        self.install(
            skill_id,
            InstallOptions {
                version: Some(skill.latest_version.clone()),
                force: true,
                ..Default::default()
            },
        ).await
    }

    /// 更新所有技能
    pub async fn update_all(&self) -> Vec<Result<String, RegistryError>> {
        let installed = self.installed.read().await;
        let skill_ids: Vec<String> = installed.keys().cloned().collect();
        drop(installed);

        let mut results = Vec::new();

        for skill_id in skill_ids {
            match self.update(&skill_id).await {
                Ok(_) => results.push(Ok(skill_id)),
                Err(e) => results.push(Err(e)),
            }
        }

        results
    }

    /// 列出已安装技能
    pub async fn list_installed(&self) -> Vec<InstalledSkill> {
        let installed = self.installed.read().await;
        installed.values().cloned().collect()
    }

    /// 检查更新
    pub async fn check_updates(&self) -> HashMap<String, String> {
        let installed = self.installed.read().await;
        let mut updates = HashMap::new();

        for (skill_id, info) in installed.iter() {
            if let Ok(skill) = self.get_skill(skill_id).await {
                if skill.latest_version != info.version {
                    updates.insert(skill_id.clone(), skill.latest_version.clone());
                }
            }
        }

        updates
    }

    /// 下载技能包
    async fn download_bundle(
        &self,
        url: &str,
        skill_id: &str,
        version: &str,
    ) -> Result<PathBuf, RegistryError> {
        let cache_file = self
            .config
            .cache_dir
            .join(format!("{}-{}.bundle", skill_id, version));

        // 检查缓存
        if cache_file.exists() {
            debug!("使用缓存的技能包: {:?}", cache_file);
            return Ok(cache_file);
        }

        info!("下载技能包: {}", url);

        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| RegistryError::Network(e.to_string()))?;

        std::fs::write(&cache_file, bytes)
            .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;

        Ok(cache_file)
    }

    /// 验证校验和
    fn verify_checksum(&self, path: &PathBuf, expected: &str) -> Result<(), RegistryError> {
        use sha2::{Digest, Sha256};

        let content = std::fs::read(path)
            .map_err(|e| RegistryError::VerificationFailed(e.to_string()))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = format!("{:x}", hasher.finalize());

        if hash != expected {
            return Err(RegistryError::VerificationFailed(format!(
                "校验和不匹配: {} != {}",
                hash, expected
            )));
        }

        Ok(())
    }

    /// 解压技能包
    async fn extract_bundle(
        &self,
        bundle_path: &PathBuf,
        install_path: &PathBuf,
    ) -> Result<SkillBundle, RegistryError> {
        // 创建安装目录
        std::fs::create_dir_all(install_path)
            .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;

        // 解压
        let file = std::fs::File::open(bundle_path)
            .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => install_path.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath).ok();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).ok();
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)
                    .map_err(|e| RegistryError::InstallFailed(e.to_string()))?;
                std::io::copy(&mut file, &mut outfile).ok();
            }
        }

        // 加载清单
        let bundle = SkillBundle::from_dir(install_path)?;
        Ok(bundle)
    }

    /// 安装依赖
    async fn install_dependencies(
        &self,
        manifest: &BundleManifest,
    ) -> Result<(), RegistryError> {
        for dep in &manifest.dependencies {
            if dep.optional {
                continue;
            }

            // 检查是否已安装
            let installed = self.installed.read().await;
            if installed.contains_key(&dep.name) {
                continue;
            }
            drop(installed);

            // 安装依赖 - 使用 Box::pin 避免递归问题
            let self_clone = Self {
                config: self.config.clone(),
                client: self.client.clone(),
                cache: self.cache.clone(),
                installed: self.installed.clone(),
                platform: self.platform.clone(),
            };
            
            Box::pin(self_clone.install(
                &dep.name,
                InstallOptions {
                    version: Some(dep.version.clone()),
                    ..Default::default()
                },
            ))
            .await?;
        }

        Ok(())
    }

    /// 注册到平台
    async fn register_to_platform(&self, bundle: &SkillBundle) -> Result<(), RegistryError> {
        for skill_def in &bundle.manifest.skills {
            self.platform
                .create_skill(
                    skill_def.name.clone(),
                    skill_def.description.clone(),
                    skill_def.category.clone(),
                    skill_def.tools.clone(),
                    skill_def.triggers.clone(),
                )
                .await;
        }

        Ok(())
    }
}

/// 创建默认注册表
pub fn create_default_registry(platform: Arc<SkillPlatform>) -> SkillRegistry {
    SkillRegistry::new(RegistryConfig::default(), platform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert!(!config.registry_url.is_empty());
        assert!(config.auto_update);
    }

    #[test]
    fn test_install_options_default() {
        let options = InstallOptions::default();
        assert!(options.install_deps);
        assert!(!options.force);
    }
}
