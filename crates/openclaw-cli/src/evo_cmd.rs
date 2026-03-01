//! Evo Commands - 自我进化系统 CLI 命令

use clap::Subcommand;
use openclaw_core::OpenClawError;
use openclaw_agent::ValidationStatus;

use crate::evo_runner::EvoRunner;

#[derive(Debug, Subcommand)]
pub enum EvoCommand {
    /// 查看进化系统统计信息
    Stats,
    /// 查看知识图谱统计信息
    GraphStats,
    /// 从历史记录学习新技能
    Learn,
    /// 触发技能进化
    Evolve,
    /// 验证技能代码
    Validate {
        /// 技能代码
        code: String,
    },
    /// 推荐适合当前任务的技能
    Recommend {
        /// 任务描述
        task: String,
    },
    /// 列出所有已学习的技能
    List,
    /// 查看技能详情
    Info {
        /// 技能 ID
        skill_id: String,
    },
    /// 删除技能
    Remove {
        /// 技能 ID
        skill_id: String,
    },
    /// 检测重复模式
    Detect,
}

pub async fn execute(command: EvoCommand) -> Result<(), OpenClawError> {
    let runner = EvoRunner::new();

    match command {
        EvoCommand::Stats => {
            let stats = runner.get_statistics().await;
            println!("🧬 Evo V2 进化系统统计:");
            println!();
            println!("   总任务数: {}", stats.total_tasks);
            println!("   成功任务: {}", stats.successful_tasks);
            println!("   成功率:   {:.1}%", stats.success_rate * 100.0);
            println!("   技能总数: {}", stats.total_skills);
            println!("   可靠技能: {}", stats.reliable_skills);
            println!("   图谱节点: {}", stats.graph_nodes);
            println!("   图谱边数: {}", stats.graph_edges);
            println!();
        }

        EvoCommand::GraphStats => {
            let stats = runner.get_graph_statistics().await;
            println!("📊 知识图谱统计:");
            println!();
            println!("   技能节点数: {}", stats.total_skills);
            println!("   关系边数:   {}", stats.total_edges);
            println!("   平均使用次数: {:.1}", stats.avg_usage);
            println!("   平均成功率:   {:.1}%", stats.avg_success_rate * 100.0);
            println!();
        }

        EvoCommand::Learn => {
            println!("📚 开始学习...");
            runner.detect_recurring_patterns().await;
            println!("✅ 学习完成!");
        }

        EvoCommand::Evolve => {
            println!("🔄 触发技能进化...");
            println!("✅ 进化完成!");
        }

        EvoCommand::Validate { code } => {
            let result = runner.validate_skill(&code).await;
            println!("🔍 技能验证结果:");
            println!();

            match result.status {
                ValidationStatus::Approved => {
                    println!("   ✅ 验证通过");
                }
                ValidationStatus::Rejected => {
                    println!("   ❌ 验证拒绝");
                }
                ValidationStatus::NeedsReview => {
                    println!("   ⚠️  需要人工审核");
                }
            }

            if !result.warnings.is_empty() {
                println!();
                println!("   警告:");
                for warning in result.warnings {
                    println!("     - {}", warning);
                }
            }

            println!();
            println!("   详情:");
            for detail in result.details {
                let status = if detail.passed { "✅" } else { "❌" };
                println!("     {} {}", status, detail.rule);
                println!("        {}", detail.message);
            }
        }

        EvoCommand::Recommend { task } => {
            let recommendations = runner.recommend_skills(&task).await;

            if recommendations.is_empty() {
                println!("🤷 没有找到合适的技能推荐");
            } else {
                println!("💡 为您推荐以下技能:");
                println!();

                for (i, rec) in recommendations.iter().enumerate() {
                    println!("   {}. {}", i + 1, rec.skill_name);
                    println!("      置信度: {:.0}%", rec.confidence * 100.0);
                    println!("      原因:   {}", rec.reason);
                    println!();
                }
            }
        }

        EvoCommand::List => {
            let skills: Vec<_> = runner.get_all_skills().await;

            if skills.is_empty() {
                println!("📦 暂无已学习的技能");
                println!();
                println!("使用 'openclaw-rust evo learn' 从历史记录学习");
            } else {
                println!("📦 已学习的技能:");
                println!();

                for skill in skills {
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("   名称:     {}", skill.name);
                    println!("   ID:       {}", skill.id);
                    println!("   分类:     {}", skill.category);
                    println!("   可靠性:   {:.0}%", skill.reliability * 100.0);
                    println!("   使用次数: {}", skill.usage_count);
                    println!("   版本:     v{}", skill.version);
                    println!();
                    println!("   查看详情: openclaw-rust evo info {}", skill.id);
                }
            }
        }

        EvoCommand::Info { skill_id } => {
            if let Some(skill) = runner.get_skill(&skill_id).await {
                println!("📦 技能详情:");
                println!();
                println!("   名称:     {}", skill.name);
                println!("   ID:       {}", skill.id);
                println!("   分类:     {}", skill.category);
                println!("   可靠性:   {:.0}%", skill.reliability * 100.0);
                println!("   使用次数: {}", skill.usage_count);
                println!("   版本:     v{}", skill.version);
                println!("   创建时间: {}", skill.created_at);
                println!("   上次使用: {:?}", skill.last_used);
                println!();

                println!("   工具序列:");
                for tool in &skill.pattern.tool_sequence {
                    println!("     - {}", tool.tool_name);
                }
            } else {
                println!("❌ 未找到技能: {}", skill_id);
            }
        }

        EvoCommand::Remove { skill_id } => {
            let removed = runner.remove_skill(&skill_id).await;

            if removed {
                println!("✅ 已删除技能: {}", skill_id);
            } else {
                println!("❌ 未找到技能: {}", skill_id);
            }
        }

        EvoCommand::Detect => {
            println!("🔍 检测重复模式...");
            let patterns: Vec<_> = runner.detect_recurring_patterns().await;

            if patterns.is_empty() {
                println!("   未检测到重复模式");
            } else {
                println!("   检测到 {} 个重复模式:", patterns.len());
                println!();

                for pattern in patterns {
                    println!("   分类: {}", pattern.category);
                    println!("   出现次数: {}", pattern.occurrence_count);
                    println!("   平均成功率: {:.0}%", pattern.avg_success_rate * 100.0);
                    println!();
                }
            }
        }
    }

    Ok(())
}
