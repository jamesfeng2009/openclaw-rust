# OpenClaw 各模块架构 / 解耦 / 开闭原则 / 主流程打通 — 问题列表

仅列出问题，不包含修复方案。

---

## 一、主流程未打通（跨模块集成缺失）

### 1. Server 与 Orchestrator 脱节

- **现象**：`Gateway::start()` 里创建并启动了 `ServiceOrchestrator`（含 Agent、Channel、Canvas），但 `api::create_router()` 使用的是**独立的** `ApiState`（内部自带 `AgentService`、`channels`/`sessions` 等 `Vec`）。
- **结果**：所有 HTTP 路由（`/chat`、`/api/agent/message`、`/api/agents`、`/api/sessions` 等）只看到 `ApiState`，**完全看不到** Gateway 持有的 Orchestrator；Orchestrator 的 agents/channels 与 API 使用的不是同一套状态，相当于两套主流程并存且未打通。

### 2. Agent 未注入 AI / Memory / Security

- **现象**：`BaseAgent` 设计上支持 `set_ai_provider`、`set_memory`、`set_security_pipeline`，但 **Server 侧没有任何地方调用这些方法**。Orchestrator 的 `init_agents_from_config` 和 `init_default_agents` 只做 `BaseAgent::new(openclaw_cfg)` 或 `BaseAgent::orchestrator()` 等，然后 `register_agent`，从未从 Config 或其它地方创建 AIProvider/MemoryManager/SecurityPipeline 并注入 Agent。
- **结果**：通过 Server 注册的 Agent 在 `process()` 时一律走 “No AI provider configured” 失败路径，**主流程上的对话/任务实际上不可用**。openclaw-ai、openclaw-memory、openclaw-security 在 Server 主流程中**未被使用**。

### 3. Voice 与主流程未接通

- **现象**：Server 的 `voice_service` 仅是 `enabled: bool` 的开关，**没有依赖 openclaw-voice**，也没有 STT/TTS/唤醒等能力接入。
- **结果**：配置里 `enable_voice` 只影响一个布尔状态，**语音能力未与 Gateway/API 打通**。

### 4. CLI Agent 命令未走真实 AI

- **现象**：`agent_cmd` 的 `connect_and_send` 只打印 “AI response simulation” 和提示启动 Gateway，**没有真正调用 Gateway 的 agent 接口或本地 AI**。
- **结果**：`openclaw-rust agent --message "..."` 无法形成一条“用户消息 → Agent（含 AI）→ 回复”的完整主流程。

### 5. openclaw-memory / openclaw-vector 未接入主流程

- **现象**：Server 的 Cargo.toml 依赖了 openclaw-memory、openclaw-vector，但 **gateway / api / orchestrator / agent_service 中没有任何一处** 使用 MemoryManager、VectorStore 或相关类型。
- **结果**：记忆与向量能力只在 openclaw-agent 内部被类型引用（如 `set_memory`），**主流程（Server/CLI）从未创建或注入**，相当于未打通。

### 6. openclaw-security 未接入主流程

- **现象**：Server 未依赖 openclaw-security；Agent 的 `SecurityPipeline` 仅在 `BaseAgent::process` 内按可选使用，**没有任何上层在启动时构造并注入**。
- **结果**：安全管线在整条主流程中**未被使用**。

---

## 二、按模块列出的架构 / 解耦 / 开闭原则问题

### openclaw-core

- **开闭**：配置与类型集中在一个 crate，新增配置段或新通道类型时需要改 core 的 `Config` 等类型，**对扩展不封闭**。
- **与主流程**：被广泛依赖，主流程打通依赖 core 的 Config/Message 等，当前问题主要在“谁用 core、怎么用”，不在 core 自身。

### openclaw-server

- **解耦**：`api::create_router()` 与 `Gateway` 无参数/状态共享，路由层与“服务编排层”强割裂，**高耦合到两套独立状态**（ApiState vs Orchestrator）。
- **开闭**：新增一种服务（如新 API 域）需要改 `create_router` 和可能的 `ApiState`，**扩展需修改现有模块**。
- **架构**：Agent、Channel、Session 在 API 侧用 `Vec<*Info>` 做内存态，与 Orchestrator 的 AgentService/ChannelManager 等**重复且不一致**，缺少单一事实来源。

### openclaw-agent

- **解耦**：Agent trait 依赖 `openclaw_ai::AIProvider`、`openclaw_memory::MemoryManager`、`openclaw_security::SecurityPipeline` 等具体 crate，**对实现类有直接依赖**；若希望“仅 core + 抽象”则未完全做到“依赖抽象”。
- **开闭**：新增 Agent 类型通过实现 `Agent` trait 即可，**对扩展开放**；但依赖的 AI/Memory/Security 由外部注入，若上层不注入则能力缺失，**行为上未闭环**。
- **架构**：`BaseAgent` 同时承担“对话、安全、记忆”的编排，职责较多；记忆在 `process` 中的使用（如 build_messages 是否从 memory 取上下文）需要对照设计文档确认是否完整。

### openclaw-ai

- **解耦**：通过 `AIProvider` trait 和工厂/按名创建，**对扩展新提供商较友好**；但部分策略（如 failover 的 WeightedRandom/LeastConnections）未实现，**行为与抽象不一致**。
- **开闭**：新增 Provider 需在 factory/mod 里加分支，**未完全“对扩展开放、对修改封闭”**（如用注册表或配置驱动可更好）。
- **与主流程**：类型被 agent 使用，但**主流程没有任何地方实例化并注入**，见上文。

### openclaw-memory

- **解耦**：MemoryManager 依赖 `openclaw_vector::VectorStore`、自研 compressor/embedding 等，**与 vector 和 embedding 实现耦合**；长期记忆的 payload 字段（如 text_preview vs content）与 vector/hybrid_search 的约定需统一，否则是隐式耦合。
- **开闭**：新记忆策略/新存储需改 Manager 或相关模块，**扩展点不够清晰**（如策略模式/插件式）。
- **与主流程**：仅被 agent 类型引用，**主流程未创建、未注入**。

### openclaw-vector

- **解耦**：`VectorStore` trait 定义清晰，后端实现（Memory/SQLite/LanceDB/Qdrant/PgVector）可替换，**解耦良好**。
- **开闭**：`create_store` / `create_store_async` 用枚举分支选择后端，**新增后端需改此枚举和函数**，未用“注册 + 配置”式扩展。
- **与主流程**：仅被 memory（及可能的其他 crate）使用，**主流程未直接使用**，打通依赖 memory 先接入主流程。

### openclaw-channels

- **解耦**：`Channel`、`ChannelHandler` trait 清晰；ChannelManager 依赖 trait，**与具体通道实现解耦**。
- **开闭**：新增通道类型需实现 `Channel` 并在某处注册，若注册方式集中在少数几处则**扩展需修改调用方**；事件与 handler 的扩展相对开放。
- **与主流程**：Orchestrator 使用 ChannelManager，但 **HTTP API 的 channels 是 ApiState 里的 Vec<ChannelInfo>**，与 ChannelManager 不同源，**未打通**。

### openclaw-canvas

- **解耦**：Server 的 canvas_api 直接依赖 `openclaw_canvas` 的具体类型（如 `CanvasManager`、`CollabSession`、`Color`），**若未来多实现（如不同协作后端）需引入抽象**。
- **与主流程**：已通过 `create_canvas_router` 并入路由，**与主流程打通**；Orchestrator 侧也有 canvas 服务状态，需确认与 API 是否同一套画布实例。

### openclaw-browser

- **解耦**：Server 的 browser_api 直接使用 `openclaw_browser` 的类型（如 ScrollOptions、ScreenshotFormat、PaperFormat），**API 层与 browser 实现耦合**。
- **与主流程**：已通过 `create_browser_router` 并入路由，**与主流程打通**。

### openclaw-voice

- **与主流程**：仅在 CLI 的 voice 子命令中使用；Server 的 VoiceService 不依赖 openclaw-voice，**与 Gateway/API 主流程未打通**。

### openclaw-tools

- **解耦**：技能、MCP、调度器等子模块较多，相互之间通过类型/接口使用，**内部耦合度需按子模块单独评估**；技能包与平台等与 CLI 的 skill 命令打通。
- **与主流程**：主要被 CLI 使用；**Server 未依赖 openclaw-tools**，若希望“Agent 可调工具/技能”，需在主流程中接入。

### openclaw-device

- **解耦**：设备层有 registry、adapter、platform 等分层，**抽象清晰**；camera/screen 等与具体 OS/命令耦合，属合理。
- **与主流程**：Gateway 启动时初始化 DeviceManager，并从 Config 加载设备，**已与主流程打通**；API 是否暴露设备能力需看路由设计。

### openclaw-security

- **解耦**：Pipeline、InputFilter、Classifier 等可组合，**结构上解耦良好**。
- **与主流程**：Server 未依赖 openclaw-security，Agent 内 SecurityPipeline 为可选，**整条主流程未注入**，见上文。

### openclaw-sandbox

- **与主流程**：仅 CLI 依赖；Server 未使用，**若需在服务端跑不可信代码则未打通**。

### openclaw-testing

- **用途**：仅 `#[cfg(test)]` 的 mock（AI、device、config、channel、agent），**不参与生产主流程**；架构上无问题，主流程打通不涉及。

---

## 三、总结表（是否与主流程打通）

| 模块              | 与主流程打通情况 |
|-------------------|------------------|
| openclaw-core     | 已用（Config/Message 等） |
| openclaw-server   | 自身为主流程入口，但内部 API 与 Orchestrator 双轨、未统一 |
| openclaw-agent    | 被 Server 用，但未注入 AI/Memory/Security，能力未打通 |
| openclaw-ai       | 未在主流程中实例化或注入 |
| openclaw-memory   | 未在主流程中实例化或注入 |
| openclaw-vector   | 未在主流程中直接使用，仅被 memory 等引用 |
| openclaw-channels | Orchestrator 用 ChannelManager，API 用 Vec，未统一打通 |
| openclaw-canvas   | 已通过 canvas_api 打通 |
| openclaw-browser  | 已通过 browser_api 打通 |
| openclaw-voice    | 仅 CLI voice 命令；Server 未用 |
| openclaw-tools    | 仅 CLI；Server 未用 |
| openclaw-device   | Gateway/DeviceManager 已打通 |
| openclaw-security | 未在主流程中注入 |
| openclaw-sandbox  | 仅 CLI；Server 未用 |
| openclaw-testing  | 仅测试用 |

---

以上为各 openclaw-* 模块在**架构设计、解耦、开闭原则**以及**与主流程是否打通**方面的**问题列表**，不包含修改建议或具体修复方案。
