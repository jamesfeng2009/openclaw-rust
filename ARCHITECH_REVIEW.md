总体评价（先说结论）
整体分层是清晰的：
core（基础类型） → ai / vector / memory → agent → server → cli，
横向有 device / channels / voice / canvas / browser / tools / sandbox 做能力扩展。
主流程是打通的：
CLI 的 Gateway 子命令会初始化 openclaw-device，然后跑 openclaw-server 的网关。
openclaw-server 的 service_factory / app_context 里组合了 ai + memory + vector + agent + channels + device + voice 等。
openclaw-agent 通过 ports/adapters 依赖 ai + memory + vector + device + channels + voice + browser + canvas + tools + security。
主要问题集中在：
一些 crate 顶层 re-export 太多东西，内聚边界模糊，容易“全局大依赖”。
某些“扩展点”形式上支持多实现（traits + 工厂），但仍靠中心枚举 / match 扩展，开闭原则不彻底。
少数模块（例如 device、browser）通过全局单例/静态工厂耦合到系统中，降低可测试性和替换性。
下面按模块细分。
1. openclaw-core（核心）
现状
提供 Config / UserConfig / Message / Session / OpenClawError 等基础类型，并在 lib.rs 统一 re-export。
大部分其他 crate 都直接 use openclaw_core::{Result, Message, Config, ...}，作为“领域基础层”。
问题
职责略多：配置加载、用户配置、消息模型、session、错误等都堆在一个 crate，核心层有轻微“上膨胀”的趋势。
开放扩展的空间有限：比如 OpenClawError 目前是枚举，新的大模块如果要加错误类型，需要改这里。
建议
内部分组更清晰：在 crate 内再划逻辑层（例如 core::config, core::messaging, core::session），对外不要一次性 glob re-export 全部，鼓励按子域依赖。
对错误类型，考虑：
核心层保留通用错误（IO、Serialize、Network），
业务子系统（memory/agent/server）用各自的错误类型，再通过 From 转成 OpenClawError，减少核心枚举的扩张。
2. openclaw-ai
现状
明确的“AI provider 抽象层”：AIProvider trait + ProviderConfig + ProviderType 枚举 + FailoverManager 等。
openclaw-agent / openclaw-memory / openclaw-server 都通过 AIProvider 接口使用，而不是直接依赖 OpenAI/DeepSeek 的 HTTP 实现。
问题
开闭原则不彻底：
新增一个 provider 需要：
扩展 ProviderType 枚举，
在工厂或匹配逻辑里增加分支，
也就是说“扩展”仍然需要改核心代码。
lib.rs 的 re-export 比较“大”，导致上层容易直接拿到所有底层类型，而不是只面向 AIProvider + ChatRequest/Response 这样的接口编程。
建议
更彻底的插件式设计：
将 ProviderType 只用于配置序列化，真正实例化 provider 时使用注册表/插件工厂（register_provider("openai", Box<dyn ProviderFactory>)），新增提供商只需新增 crate + 注册，不必改枚举和主工厂。
对上层 crate，推荐只依赖：
AIProvider trait
ChatRequest/Response / EmbeddingRequest/Response，
而非直接 import 整个 providers::*。
3. openclaw-vector
现状
提供统一的 VectorStore trait + 多后端实现（memory / sqlite / lancedb / qdrant / pgvector / milvus）。
有 VectorStoreFactory + init_all_factories()，并用 feature 控制不同后端。
问题
工厂注册逻辑目前在 init_all_factories() 里，仍然是“中心点 + feature 条件编译”。新增后端时，仍需改这一处代码。
store 和 types 全部 re-export，扩展点（factory）和具体类型（SqliteStore、MemoryStore）混在一起，对使用者来说不够“端口/适配器”分明。
建议
将工厂注册拆成：
核心 crate 只提供 VectorStore trait + Factory 接口 + 注册函数；
每个后端实现自己的 init()（在自己 crate 的 lib.rs 中调用注册），这样新后端只通过 feature 打开自己的 crate 即可自动注册。
在其他模块（memory/server）中，尽量只依赖 VectorStore / BackendConfig / 工厂接口，不直接引用具体 store 类型。
4. openclaw-memory
现状
架构比较完备：工作记忆、短期总结、长期向量存储 + Hybrid Search + BM25 + AgentWorkspace 等。
提供 MemoryManager / MemoryManagerFactory / AgentWorkspace，被 openclaw-agent 和 openclaw-server 的 app_context / agentic_rag 使用，和主流程是打通的。
问题
lib 里 re-export 很多内部细节（Bm25Index, ChunkManager, FileTracker, RecallStrategy 等），对上层暴露过宽，容易产生“从 server 直接用底层 BM25”的耦合。
领域职责有点多：记忆管理、文件追踪、知识图谱、工作区、schema 常量等都堆在一个 crate 里，未来可能变成“巨石记忆模块”。
建议
对上层模块，建议只暴露有限的“端口”：
服务级：MemoryManager / MemoryManagerFactory / AgentWorkspace / MemoryRecall。
其余底层索引、知识图谱等，通过组合在内部实现。
若项目继续扩展，考虑拆分：
openclaw-memory-core（MemoryItem / MemoryConfig / MemoryManager）
openclaw-memory-index（BM25/HybridSearch/向量集成）
openclaw-workspace（AgentWorkspace + 文件布局）
以便不同项目可以只依赖需要的部分。
5. openclaw-agent
现状
职责：Agent 类型、团队、orchestrator、decision、ports、memory pipeline、channels/device/voice 的适配等。
lib.rs 基本把所有模块 pub use 出来，openclaw-server 通过 AgentOrchestrator、ports 等接入，已经与主流程打通。
ports、provider、memory_pipeline 体现了典型的“端口/适配器”思想，整体架构思路是好的。
问题
依赖扇出非常大：
Cargo.toml 中 openclaw-agent 依赖 core/ai/memory/vector/device/channels/voice/tools/sandbox/canvas/browser/security 等，几乎“全家桶”。这使得 agent 层变成强耦合中心。
顶层 pub use 几乎导出了所有类型，边界不再清晰：上层代码可以直接访问内部实现（如 RealDeviceTools、ui_tools 等），绕过 orchestrator/ports。
建议
将 agent 分层：
public API 层（例如 AgentOrchestrator、AgentConfig、Task、port 接口）——在 lib.rs 暴露；
internal adapters 层（channels/device/voice/tools 的具体适配）——仅在 crate 内使用，不对外 pub use。
在 openclaw-server 中，只依赖 agent 提供的port 接口（MemoryPort、ToolPort、DevicePort 等）和 orchestrator，减少对具体 device/channel 实现的直接感知。
6. openclaw-server
现状
典型“Composition Root”：app_context + service_factory 负责把 core/ai/memory/vector/agent/device/channels/voice/canvas/browser/security 装配成完整 HTTP/WebSocket 服务（agent_service, channel_service, voice_service, canvas_api, browser_api 等）。
CLI 的 Gateway 命令通过 commands::gateway::run() 调用 Gateway（server），主流程是打通的。
问题
服务器层承担了大量组装逻辑，有变成“巨型上帝模块”的风险（但目前结构还算分模块）。
有些子模块更多偏 demo 性质（例如 agentic_rag_api、hardware_tools 等）——如果上层业务需要更强隔离，这些 demo 应与核心 HTTP API 分开。
建议
把 AppContext 明确当作依赖注入容器：
对下只依赖 trait/port（如 VectorStoreFactory、AIProvider、AgentOrchestrator），
不感知具体后端类型。
对“实验性功能”（agentic RAG、特殊 device 工具），可以放到 feature-gated 子模块或单独 crate，避免 server 核心随着实验功能变得臃肿。
7. openclaw-device
现状
设计为多层：Platform / Device / HAL / Framework / Modules + DeviceRegistry + UnifiedDeviceManager。
init_device() 使用 OnceLock 构建全局 DeviceRegistry，CLI 和 server 都通过它来初始化和查询设备能力，已经接入主流程。
问题
全局单例 DEVICE_REGISTRY + init_device() 这种硬单例对测试/替换不友好，也和依赖注入风格（server/app_context）不太一致。
init_device() 内直接 println! 多行 ASCII banner，I/O 副作用固定在库层，CLI / Server 无法自定义输出策略（例如在 TUI/GUI 中嵌入）。
建议
把 DeviceRegistry 的生命周期交给 openclaw-server 的 AppContext 管理：
init_device() 仅返回 Arc<DeviceRegistry>，
CLI/Server 再决定是否缓存进全局或挂入上下文。
将 banner 输出封装成一个可选的 helper（例如 print_device_info(&DeviceCapabilities)），由 CLI 选择调用，库只返回数据。
8. openclaw-voice
现状
模块划分清晰：stt / tts / talk_mode / wake / voice_agent，统一通过 VoiceAgent 和配置管理对外。
被 openclaw-server::voice_service、openclaw-cli::voice_cmd、openclaw-agent::voice 使用，与主流程打通。
问题
provider 切换逻辑和 AI 类似，扩展新 STT/TTS 时需要改内部枚举/工厂（开闭原则不完全）。
VoiceAgent 和对话模式与 openclaw-agent 的文本对话 orchestrator 是分离的；如果想要“语音驱动的多 agent 流程”，目前需要上层手动把两者接起来。
建议
对 STT/TTS 也采用更插件化的 provider 注册表（类似 AI）。
在 openclaw-agent 里提供一个标准的“语音入口端口”（例如 VoicePort），使 VoiceAgent 能直接作为 agent 的一个输入源，而不是平行于 agent 存在。
9. openclaw-channels
现状
做得比较“正统”的消息通道抽象：Channel trait + ChannelManager + ChannelFactoryRegistry，多个平台实现各自 Channel。
与 openclaw-server::channel_service 和 openclaw-agent::channels 对接，主流程已打通。
问题
新增 channel 仍需：
在 registry 里注册，
在 config/enum 中添加类型，
扩展时需要改中心代码。
某些 channel 实现聚合了很多平台特性，测试替换较难（例如 telegram/signal 的 HTTP/本地集成混合在一起）。
建议
将 ChannelFactoryRegistry 设计为真正的“插件总线”：
每个 channel 模块在自己的 lib.rs 或初始化函数中注册自己，核心只持有 dyn Channel。
把“认证 / 回调 URL / Webhook 验证”等通道特有结构尽量局部化在实现内部，对上仅以 ChannelMessage 与 SendMessage 交流。
10. openclaw-canvas
现状
提供 CanvasManager + CollabManager + WebSocket 协作事件等。
与 openclaw-server::canvas_api 对接，并在 agent 层 ui_tools 里有工具，已接入整体架构。
问题
目前看更像一个相对独立子系统，与 agent/workspace 的“知识流”结合不深（比如画布元素不直接进入 memory/knowledge graph）。
与 channels/voice/agent 的联动较弱（例如“在 chat 中创建/操作画布”的统一流程还不明显）。
建议
在 openclaw-agent 的工具层中，定义标准 Canvas Tool 接口（例如 “CreateCanvas”, “UpdateElement”），由 openclaw-canvas 实现，这样 agent 决策层看到的是统一工具，而不直接和具体 CanvasManager 对话。
有计划的话，可让 canvas 事件流（比如用户涂鸦/注释）进入 openclaw-memory 的 Working/ShortTerm 记忆中，形成统一上下文。
11. openclaw-browser
现状
清晰的 Browser / Page / ScreenshotUtils / BrowserClient trait；
提供 BrowserClient trait，Browser 自己实现了这个接口，方便 mock/替换。
被 openclaw-tools::browser_tools、openclaw-server::browser_api 使用，和主流程已打通。
问题
目前 Browser 类型本身既是“客户端实现”又是“资源拥有者”，对于大规模并发和池化场景而言可能过于单一。
浏览器配置、代理、认证等部分集中在 types 里，但上层如何切换具体浏览器后端还不够显性（例如 Chrome vs Playwright vs remote driver）。
建议
在 openclaw-tools 或 openclaw-server 层明确定义一个 BrowserPool 抽象端口，openclaw-browser 仅实现一个具体池。
若未来支持多种后端（Chromiumoxide、Playwright、remote driver），建议像 openclaw-vector 一样用 factory + registry 管理。
12. openclaw-cli
现状
作为真正的“最上层”：负责解析命令行，把参数传给 openclaw-server / openclaw-agent / openclaw-device / openclaw-voice / openclaw-tools 等。
Gateway 子命令明确调用了 openclaw_device::init_device() + commands::gateway::run(...)，主流程入口清晰。
问题
CLI 某些子命令承担了部分“业务逻辑”（如 skill marketplace 输出、onboarding 文案），这些逻辑将来可能在 GUI / API 中重用时，只能复制。
对配置文件路径、默认 gateway URL 等约束都写死在 CLI 层，和 server/config 有一定重复。
建议
将“系统能力说明、默认 config 路径、skill 市场交互”等抽成服务接口（例如 OnboardingService、SkillMarketplaceClient），CLI 只是一个 UI 皮肤；这样未来增加 TUI/GUI/VSCode 插件时可重用同一逻辑。
对于 gateway_url / 默认端口等，建议统一通过 openclaw-core::Config 或 openclaw-server::ServerConfig 暴露，CLI 读取配置而不是硬编码。
综合建议（架构层）
减少顶层 pub use * 的暴露面
核心模块（agent, memory, ai, server）可以分为：
Public API（trait/port/高层服务对象）
Internal implementation（适配器/具体 provider/channel/device）
lib.rs 只 re-export Public API，保持模块边界清晰。
更统一的“端口/适配器 + 工厂注册”模式
目前 AI / Vector / Channels / Voice 都有类似的 pattern，但实现方式略不统一（枚举 + match / feature + init_all_factories）。可以抽象出通用套路：
核心 crate 定义 Port + Factory + Registry trait；
每个后端自己注册；开启 feature 即注册生效；新增实现无需改核心。
弱化全局单例，强化 AppContext/DI
像 DEVICE_REGISTRY 这种全局单例，建议仅用在非常底层且无依赖的地方。大多数情况下，使用 AppContext / ServiceFactory 来管理生命周期更一致，也更易测试。