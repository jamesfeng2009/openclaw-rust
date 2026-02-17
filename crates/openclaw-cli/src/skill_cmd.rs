use std::path::PathBuf;

use clap::Subcommand;
use openclaw_core::OpenClawError;

#[derive(Debug, Subcommand)]
pub enum SkillCommand {
    /// 列出已安装的技能包
    List,
    /// 从市场搜索技能
    Search {
        /// 搜索关键词
        query: String,
    },
    /// 从市场安装技能包
    Install {
        /// 技能包 ID
        bundle_id: String,
    },
    /// 卸载技能包
    Uninstall {
        /// 技能包 ID
        bundle_id: String,
    },
    /// 查看市场分类
    Categories,
    /// 查看技能包详情
    Info {
        /// 技能包 ID
        bundle_id: String,
    },
}

pub async fn execute(command: SkillCommand) -> Result<(), OpenClawError> {
    match command {
        SkillCommand::List => {
            println!("📦 已安装的技能包:");
            println!();
            println!("   (暂无已安装的技能包)");
            println!();
            println!("使用 'openclaw-rust skill search <关键词>' 搜索市场技能");
        }
        
        SkillCommand::Search { query } => {
            println!("🔍 搜索技能市场: {}", query);
            println!();
            
            let platform = openclaw_tools::SkillPlatform::new();
            let bundles_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("openclaw")
                .join("skills");
            
            let manager = openclaw_tools::BundleManager::new(
                Arc::new(platform),
                bundles_dir,
            );
            
            match manager.search_marketplace(&query).await {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("   未找到匹配的技能包");
                    } else {
                        for entry in entries {
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            println!("📦 {}", entry.name);
                            println!("   ID: {}", entry.id);
                            println!("   版本: {}", entry.version);
                            println!("   作者: {}", entry.author);
                            println!("   描述: {}", entry.description);
                            println!("   标签: {:?}", entry.tags);
                            println!("   下载: {} | 评分: ⭐ {:.1}", entry.downloads, entry.rating);
                            println!();
                            println!("   安装: openclaw-rust skill install {}", entry.id);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ 搜索失败: {}", e);
                }
            }
        }
        
        SkillCommand::Install { bundle_id } => {
            println!("📥 安装技能包: {}", bundle_id);
            println!();
            println!("⚠️  安装功能需要市场 API 支持");
            println!("   当前使用离线模式，请先使用 'openclaw-rust skill search' 查看可用技能");
        }
        
        SkillCommand::Uninstall { bundle_id } => {
            println!("🗑️  卸载技能包: {}", bundle_id);
            println!();
            println!("⚠️  卸载功能开发中");
        }
        
        SkillCommand::Categories => {
            println!("📂 技能市场分类:");
            println!();
            
            let platform = openclaw_tools::SkillPlatform::new();
            let bundles_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("openclaw")
                .join("skills");
            
            let manager = openclaw_tools::BundleManager::new(
                Arc::new(platform),
                bundles_dir,
            );
            
            match manager.get_categories().await {
                Ok(categories) => {
                    for (i, cat) in categories.iter().enumerate() {
                        println!("   {}. {}", i + 1, cat);
                    }
                }
                Err(e) => {
                    println!("❌ 获取分类失败: {}", e);
                }
            }
        }
        
        SkillCommand::Info { bundle_id } => {
            println!("ℹ️  技能包详情: {}", bundle_id);
            println!();
            println!("⚠️  详情功能需要市场 API 支持");
        }
    }
    
    Ok(())
}

use std::sync::Arc;
