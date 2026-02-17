//! Agent Team 定义

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::agent::{Agent, BaseAgent};
use crate::types::{AgentConfig, AgentType, Capability};

/// Agent Team 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    /// Team ID
    pub id: String,
    /// Team 名称
    pub name: String,
    /// Team 描述
    pub description: Option<String>,
    /// Agent 配置列表
    pub agents: Vec<AgentConfig>,
    /// 默认路由策略
    pub routing_strategy: RoutingStrategy,
}

impl TeamConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            agents: Vec::new(),
            routing_strategy: RoutingStrategy::default(),
        }
    }

    pub fn with_agent(mut self, agent: AgentConfig) -> Self {
        self.agents.push(agent);
        self
    }

    pub fn with_routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.routing_strategy = strategy;
        self
    }

    /// 创建默认 Team 配置
    pub fn default_team() -> Self {
        Self::new("default", "Default Team")
            .with_agent(
                AgentConfig::new("orchestrator", "Orchestrator", AgentType::Orchestrator)
                    .with_priority(100),
            )
            .with_agent(AgentConfig::new(
                "chat",
                "Chat Agent",
                AgentType::Conversationalist,
            ))
            .with_agent(AgentConfig::new(
                "researcher",
                "Researcher",
                AgentType::Researcher,
            ))
            .with_agent(AgentConfig::new("coder", "Coder", AgentType::Coder))
    }

    /// 创建开发团队配置
    pub fn dev_team() -> Self {
        Self::new("dev", "Development Team")
            .with_agent(AgentConfig::new("orchestrator", "Orchestrator", AgentType::Orchestrator)
                .with_priority(100))
            .with_agent(AgentConfig::new("coder", "Coder", AgentType::Coder)
                .with_priority(80))
            .with_agent(AgentConfig::new("reviewer", "Code Reviewer", AgentType::Coder)
                .with_system_prompt("You are a code reviewer. Focus on code quality, security, and best practices.")
                .with_priority(70))
            .with_agent(AgentConfig::new("researcher", "Researcher", AgentType::Researcher))
    }

    /// 创建研究团队配置
    pub fn research_team() -> Self {
        Self::new("research", "Research Team")
            .with_agent(
                AgentConfig::new("orchestrator", "Orchestrator", AgentType::Orchestrator)
                    .with_priority(100),
            )
            .with_agent(
                AgentConfig::new("researcher", "Primary Researcher", AgentType::Researcher)
                    .with_priority(80),
            )
            .with_agent(AgentConfig::new(
                "analyst",
                "Data Analyst",
                AgentType::DataAnalyst,
            ))
            .with_agent(AgentConfig::new(
                "writer",
                "Report Writer",
                AgentType::Writer,
            ))
    }
}

/// 路由策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// 能力匹配 - 选择具备所需能力的 Agent
    CapabilityMatch,
    /// 负载均衡 - 选择负载最低的 Agent
    LoadBalance,
    /// 优先级 - 选择优先级最高的 Agent
    Priority,
    /// 轮询 - 依次分配
    RoundRobin,
    /// 智能路由 - 根据任务类型和 Agent 能力智能选择
    Smart,
    /// 手动指定 - 由用户指定 Agent
    Manual,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::Smart
    }
}

/// Agent Team
pub struct AgentTeam {
    config: TeamConfig,
    agents: RwLock<HashMap<String, Arc<BaseAgent>>>,
    round_robin_counter: RwLock<usize>,
}

impl AgentTeam {
    pub fn new(config: TeamConfig) -> Self {
        let agents = config
            .agents
            .iter()
            .map(|agent_config| {
                let agent = Arc::new(BaseAgent::new(agent_config.clone()));
                (agent_config.id.clone(), agent)
            })
            .collect();

        Self {
            config,
            agents: RwLock::new(agents),
            round_robin_counter: RwLock::new(0),
        }
    }

    /// 创建默认 Team
    pub fn default_team() -> Self {
        Self::new(TeamConfig::default_team())
    }

    /// 获取 Team 配置
    pub fn config(&self) -> &TeamConfig {
        &self.config
    }

    /// 获取所有 Agent ID
    pub fn agent_ids(&self) -> Vec<String> {
        let agents = self.agents.read().unwrap();
        agents.keys().cloned().collect()
    }

    /// 获取指定 Agent
    pub fn get_agent(&self, id: &str) -> Option<Arc<BaseAgent>> {
        let agents = self.agents.read().unwrap();
        agents.get(id).cloned()
    }

    /// 添加 Agent
    pub fn add_agent(&self, agent: BaseAgent) {
        let mut agents = self.agents.write().unwrap();
        agents.insert(agent.id().to_string(), Arc::new(agent));
    }

    /// 移除 Agent
    pub fn remove_agent(&self, id: &str) -> Option<Arc<BaseAgent>> {
        let mut agents = self.agents.write().unwrap();
        agents.remove(id)
    }

    /// 选择最佳 Agent 处理任务
    pub fn select_agent(
        &self,
        required_capabilities: &[Capability],
        preferred_agent: Option<&str>,
    ) -> Option<String> {
        // 如果指定了偏好 Agent，优先选择
        if let Some(preferred) = preferred_agent {
            if let Some(agent) = self.get_agent(preferred) {
                if agent.is_available()
                    && required_capabilities
                        .iter()
                        .all(|c| agent.has_capability(c))
                {
                    return Some(preferred.to_string());
                }
            }
        }

        let agents = self.agents.read().unwrap();

        match self.config.routing_strategy {
            RoutingStrategy::CapabilityMatch => {
                self.select_by_capability(&agents, required_capabilities)
            }
            RoutingStrategy::LoadBalance => self.select_by_load(&agents, required_capabilities),
            RoutingStrategy::Priority => self.select_by_priority(&agents, required_capabilities),
            RoutingStrategy::RoundRobin => {
                self.select_by_round_robin(&agents, required_capabilities)
            }
            RoutingStrategy::Smart | RoutingStrategy::Manual => {
                self.select_smart(&agents, required_capabilities)
            }
        }
    }

    fn select_by_capability(
        &self,
        agents: &HashMap<String, Arc<BaseAgent>>,
        required_capabilities: &[Capability],
    ) -> Option<String> {
        agents
            .values()
            .filter(|a| a.is_available())
            .filter(|a| required_capabilities.iter().all(|c| a.has_capability(c)))
            .max_by_key(|a| {
                a.capabilities()
                    .iter()
                    .filter(|c| required_capabilities.contains(c))
                    .count()
            })
            .map(|a| a.id().to_string())
    }

    fn select_by_load(
        &self,
        agents: &HashMap<String, Arc<BaseAgent>>,
        required_capabilities: &[Capability],
    ) -> Option<String> {
        agents
            .values()
            .filter(|a| a.is_available())
            .filter(|a| required_capabilities.iter().all(|c| a.has_capability(c)))
            .min_by(|a, b| {
                a.load()
                    .partial_cmp(&b.load())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.id().to_string())
    }

    fn select_by_priority(
        &self,
        agents: &HashMap<String, Arc<BaseAgent>>,
        required_capabilities: &[Capability],
    ) -> Option<String> {
        agents
            .values()
            .filter(|a| a.is_available())
            .filter(|a| required_capabilities.iter().all(|c| a.has_capability(c)))
            .max_by_key(|a| a.info().config.priority)
            .map(|a| a.id().to_string())
    }

    fn select_by_round_robin(
        &self,
        agents: &HashMap<String, Arc<BaseAgent>>,
        required_capabilities: &[Capability],
    ) -> Option<String> {
        let available: Vec<_> = agents
            .values()
            .filter(|a| a.is_available())
            .filter(|a| required_capabilities.iter().all(|c| a.has_capability(c)))
            .collect();

        if available.is_empty() {
            return None;
        }

        let mut counter = self.round_robin_counter.write().unwrap();
        *counter = (*counter + 1) % available.len();
        Some(available[*counter].id().to_string())
    }

    fn select_smart(
        &self,
        agents: &HashMap<String, Arc<BaseAgent>>,
        required_capabilities: &[Capability],
    ) -> Option<String> {
        // 智能选择：综合考虑能力匹配、负载和优先级
        agents
            .values()
            .filter(|a| a.is_available())
            .filter(|a| required_capabilities.iter().all(|c| a.has_capability(c)))
            .max_by(|a, b| {
                let score_a = self.calculate_agent_score(a, required_capabilities);
                let score_b = self.calculate_agent_score(b, required_capabilities);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|a| a.id().to_string())
    }

    fn calculate_agent_score(
        &self,
        agent: &BaseAgent,
        required_capabilities: &[Capability],
    ) -> f32 {
        // 能力匹配分数 (0-50)
        let capability_score = required_capabilities
            .iter()
            .filter(|c| agent.has_capability(c))
            .count() as f32
            / required_capabilities.len().max(1) as f32
            * 50.0;

        // 优先级分数 (0-30)
        let priority_score = agent.info().config.priority as f32 / 100.0 * 30.0;

        // 负载分数 (0-20, 负载越低分数越高)
        let load_score = (1.0 - agent.load()) * 20.0;

        capability_score + priority_score + load_score
    }
}
