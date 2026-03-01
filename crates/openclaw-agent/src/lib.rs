//! OpenClaw Agent - 多智能体系统
//!
//! 实现 Agent Teams 架构：
//! - 不同类型的 Agent 处理不同任务
//! - Agent Orchestrator 协调多个 Agent
//! - 任务路由和分配机制

// === Public API ===

pub mod agent;
pub mod aieos;
pub mod config;
pub mod control;
pub mod control_flow;
pub mod orchestrator;
pub mod ports;
pub mod provider;
pub mod safety;
pub mod session_tree;
pub mod sessions;
pub mod memory;
pub mod extension;
pub mod evo;
pub mod sub_agent;
pub mod task;
pub mod team;
pub mod types;

pub use agent::*;
pub use aieos::AIEOS;
pub use config::{AgentDefaults, AgentInstanceConfig, AgentsConfig};
pub use control::MessageQueue;
pub use control_flow::{AgentControlFlow, AgentRunResult, ControlFlowStats, RunReason, RunStatus};
pub use orchestrator::*;
pub use ports::{
    AIPort, MemoryEntry, MemoryPort, RecallItem, SecurityCheckResult, SecurityPort,
    ToolInfo, ToolPort, DevicePort,
};
pub use provider::*;
pub use safety::{TurnLimiter, TurnLimitConfig, LimitReason, TimeoutController, TimeoutConfig, TimeoutState, AbortableTask, AgentSafetyWrapper, AgentSafetyConfig, SafetyAction, SafetyStats};
pub use session_tree::*;
pub use sessions::*;
pub use memory::{CompressionConfig, ContextCompactor, ContextMessage, CleanupPolicy, MemoryCleanupPolicy};
pub use extension::{Extension, ExtensionError, ExtensionMeta, ExtensionRegistry, ExtensionResult, ExtensionSource, Priority};
pub use evo::{CompiledSkill, DynamicCompiler, EvolutionEngine, EvolutionResult, EvolutionStatus, ProgrammingLanguage, SkillGenerator, SkillSandbox, ToolNeed, Sandbox};
pub use evo::registry::{SharedSkillRegistry, DynamicSkill, SkillSource, SkillType, SkillGating, SkillFormat};
pub use evo::adapter::EvoToAcpAdapter;
pub use evo::unified_skill_service::UnifiedSkillService;
pub use evo::skill_loader::SkillLoader;
pub use evo::skill_tool_adapter::SkillToolAdapter;
pub use evo::propagation::SkillPropagationService;
pub use evo::skill_prompt_injector::SkillPromptInjector;
pub use evo::evo_v2_engine::{EvoV2Engine, EvoConfig, EvoContext, EvoSkill, Recommendation, EvoStatistics};
pub use evo::pattern_analyzer::{PatternAnalyzer, TaskPattern, ToolCall, ToolCallPattern};
pub use evo::learning_history::{LearningHistory, LearningRecord, RecurringPattern};
pub use evo::knowledge_graph::{KnowledgeGraph, SkillNode, GraphStatistics};
pub use evo::skill_validator::{SkillValidator, ValidationResult, ValidationStatus, ValidationDetail};
pub use evo::version_manager::{VersionManager, VersionRecord, VersionDiff};
pub use sub_agent::*;
pub use task::*;
pub use team::*;
pub use types::*;

pub use openclaw_core::{OpenClawError, Result};

// === Internal (crate-only) ===

pub mod channels;
pub mod decision;
pub mod dependencies;
pub mod device_tool_registry;
pub mod device_tools;
pub mod graph;
pub mod integration;
pub mod memory_pipeline;
pub mod presence;
pub mod real_device_tools;
pub mod router;
pub mod ui_tools;
pub mod voice;

#[cfg(feature = "testing")]
pub mod mock;
