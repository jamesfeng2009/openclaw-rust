# OpenClaw Rust 项目扫描清单（openclaw-* 模块）

更新时间：2026-02-27

## 1. 总览结论

- **主入口**：`openclaw-cli` 提供二进制 `openclaw-rust`，通过 `gateway` 子命令启动 `openclaw-server::Gateway`。
- **主流程组装点**：`openclaw-server::gateway_service::Gateway::new/start` + `openclaw-server::service_factory::DefaultServiceFactory` + `openclaw-server::orchestrator::ServiceOrchestrator`。
- **HTTP API 集成情况**：`openclaw-server/src/api.rs` 已经把以下模块接入路由：
  - `/voice/*`（voice）
  - `/api/channels/*`（channels）
  - `/api/agents/*`、`/chat`（agent/orchestrator）
  - `create_device_router`（device）
  - 可选 `canvas` / `browser`（取决于是否传入 `canvas_manager`、`browser_config`）
- **关键发现（高优先级）**：ACP（`openclaw-acp` + `openclaw-server::AcpService`）目前**看起来未真正接入启动链路**。
  - 证据：
    - `DefaultServiceFactory::create_acp_service()` 已实现
    - `ChannelMessageHandler` 提供 `create_channel_handler_with_acp()`
    - 但在 `openclaw-server/src` 内 **未找到** `create_channel_handler_with_acp(` 的调用点，`create_acp_service(` 也只有定义没有实际调用点
  - 影响：`acp.yaml` 即使配置了 `enabled=true` 也大概率不会生效；群聊提及路由/多 agent 协作不会被启用。

## 2. Workspace 模块清单

Workspace members（`Cargo.toml`）：

- `openclaw-core`
- `openclaw-ai`
- `openclaw-memory`
- `openclaw-vector`
- `openclaw-channels`
- `openclaw-agent`
- `openclaw-voice`
- `openclaw-server`
- `openclaw-cli`
- `openclaw-canvas`
- `openclaw-browser`
- `openclaw-sandbox`
- `openclaw-tools`
- `openclaw-device`
- `openclaw-security`
- `openclaw-testing`
- `openclaw-acp`

## 3. 模块依赖与调用关系（简化图）

### 3.1 依赖方向（由上层到下层）

- `openclaw-cli` -> `openclaw-server` -> `openclaw-agent` -> `openclaw-* (ai/memory/vector/tools/security/voice/channels/device/canvas/browser/acp/sandbox)`

### 3.2 关键依赖（按 crate）

- `openclaw-core`
  - **职责**：核心类型、错误、配置。
  - **集成**：全仓库基础依赖。

- `openclaw-ai`
  - **依赖**：`openclaw-core`、`openclaw-tools`、`openclaw-sandbox`
  - **职责**：AI Provider 抽象与实现工厂。
  - **集成**：由 `openclaw-server::DefaultServiceFactory::create_ai_provider()` 创建后注入 orchestrator。

- `openclaw-vector`
  - **依赖**：`openclaw-core`
  - **职责**：向量存储抽象；通过 feature 选择 lancedb/qdrant/pgvector/milvus/sqlite。
  - **集成**：`Gateway::new()` 调用 `openclaw_vector::init_all_factories()`；`VectorStoreRegistry::register_from_config()` 选择后端。

- `openclaw-memory`
  - **依赖**：`openclaw-core`、`openclaw-vector`、`openclaw-ai`
  - **职责**：分层记忆（working/short/long + hybrid_search）。
  - **集成**：`DefaultServiceFactory::create_memory_backend()` 创建并放入 `AppContext.memory_backend`；随后通过 ports adapter 注入 agent。

- `openclaw-tools`
  - **依赖**：`openclaw-core`、`openclaw-browser`、`openclaw-sandbox`
  - **职责**：工具生态（builtin/mcp/wasm/webhook/skill）。
  - **集成**：`DefaultServiceFactory::create_tool_registry()` 创建并注入 agent。

- `openclaw-device`
  - **依赖**：`openclaw-core`、`openclaw-tools`
  - **职责**：设备能力抽象与能力节点。
  - **集成**：
    - CLI 启动时 `openclaw_device::init_device(true)`
    - Server 中 `DeviceManager` + `UnifiedDeviceManager` 通过 `DevicePortAdapter` 注入 agent
    - `openclaw-server/src/device_api.rs` 作为 API 暴露

- `openclaw-security`
  - **依赖**：`openclaw-core`
  - **职责**：安全管线（输入过滤、验证器、权限）。
  - **集成**：`DefaultServiceFactory::create_security_pipeline()` 创建并注入 agent。

- `openclaw-channels`
  - **依赖**：`openclaw-core`
  - **职责**：多渠道接入（telegram/discord/dingtalk/wecom/feishu/...）。
  - **集成**：`ServiceOrchestrator::start()` 中 `register_default_channels()` 并启动 `ChannelManager`。

- `openclaw-voice`
  - **依赖**：`openclaw-core`
  - **职责**：STT/TTS/talk_mode 等。
  - **集成**：
    - `Gateway::start()` -> `init_voice_service()`
    - `openclaw-server/src/api.rs` 暴露 `/voice/stt` `/voice/tts`

- `openclaw-canvas`
  - **依赖**：`openclaw-core`
  - **职责**：画布协作与状态管理。
  - **集成**：若 `Gateway` 给 `create_router()` 传入 `canvas_manager`，则 merge `create_canvas_router()`。

- `openclaw-browser`
  - **依赖**：`openclaw-core`
  - **职责**：headless browser 控制（chromiumoxide）。
  - **集成**：若 `Gateway` 解析到 `browser_config`，则 merge `create_browser_router()`。

- `openclaw-sandbox`
  - **依赖**：`openclaw-core`、`openclaw-security`
  - **职责**：安全沙箱、WASM runtime、credential 相关 feature。
  - **集成**：`AppContext.sandbox_manager` 可选启用；同时 `openclaw-ai`、`openclaw-tools` 依赖它。

- `openclaw-agent`
  - **依赖**：几乎所有功能 crate（ai/memory/vector/security/voice/channels/tools/device/canvas/browser/acp/sandbox）。
  - **职责**：agent 运行、ports 注入、evo/graph 等。
  - **集成**：由 `openclaw-server::ServiceOrchestrator` 统一调度，并在 `Gateway::start()` 注入 ports。

- `openclaw-server`
  - **依赖**：上述全部服务。
  - **职责**：HTTP/WebSocket Gateway，服务编排，API 暴露。
  - **集成**：被 `openclaw-cli` 调用。

- `openclaw-testing`
  - **职责**：mock AI/provider/device/config 等。
  - **集成**：
    - 运行期未见调用（预期只用于测试）
    - 代码层 `grep` 未发现 `openclaw_testing` 被引用（“无结果”）
  - **建议**：明确其用途：
    - 若仅测试使用，应把引用保持在 dev-deps 或在文档/CI 中说明；
    - 若 intended runtime 提供模拟后端，则需在 server/cli 增加 feature/flag 入口。

- `openclaw-acp`
  - **职责**：Agent Collaboration Protocol（路由、上下文、能力）。
  - **集成**：
    - server 已实现 `AcpService` 和 channel handler 的 ACP 分支
    - 但目前**没有看到**它被真正实例化并注入 channel handler

## 4. “独立未引用/未集成”模块判断

- **基本不独立**：绝大多数 `openclaw-*` 都在 `openclaw-server` 或 `openclaw-agent` 的依赖链中。
- **疑似未集成（重要）**：`openclaw-acp` 的运行时接入缺失（只实现了能力但未启用）。
- **可能仅测试用途**：`openclaw-testing`（未发现运行时引用）。

## 5. 潜在 bug / 风险点（按模块归类）

> 风险扫描依据：全仓库 `unwrap/expect/panic/todo!/unimplemented!` 等匹配到 **592 处 / 115 文件**。以下为“更可能影响线上稳定性/安全”的高价值项（示例级别），后续建议继续细化为逐文件逐行清单。

### 5.1 openclaw-server

- **`api.rs` 中 `StatusCode::from_u16(self.code).unwrap_or(...)`**
  - **风险**：如果 `code` 非法会自动变 500，行为可接受，但建议：
    - 统一错误码来源；
    - 或在构造 `ApiError` 时限制范围。

- **ACP 未接入导致的“配置无效”风险**
  - `ServerConfig` 已支持 `acp.yaml`，`ServiceFactory` 已支持 `create_acp_service()`，但未调用。

### 5.2 openclaw-security

- **`validator.rs` 大量 `Regex::new(...).unwrap()`**
  - **风险**：这些 regex 都是常量字符串，正常不会失败，但一旦未来修改 pattern（或引入来自配置的 pattern）会产生启动时 panic。
  - **建议修复（中优先级）**：
    - 用 `lazy_static`/`OnceLock` + `expect("...")` 携带更明确错误信息（至少易定位）；或
    - 改为 `Result` 初始化，让 `SecurityPipeline::new()` 返回可处理错误。

### 5.3 openclaw-server: channel_message_handler

- **`Regex::new(...).unwrap()`**（提及解析/清洗）
  - **风险**：同上；另外 regex 每次 `handle()` 都 new，会产生额外开销。
  - **建议修复（中优先级）**：把 regex 预编译缓存为静态（`OnceLock<Regex>`）。

### 5.4 openclaw-memory

- **`MemoryManager::retrieve()` 中对 payload 字段 `.unwrap_or("")`**
  - **风险**：不会 panic，但可能导致 recall 结果为空串，影响召回质量，且难以观测。
  - **建议修复（低~中优先级）**：
    - 增加字段缺失时的 tracing debug/warn；
    - 或把 `content` 作为 schema 强约束。

## 6. 修复建议（按优先级）

### P0（必须优先）

1. **把 ACP 真正接入启动链路**
   - **目标**：当 `config.acp.enabled=true` 时：
     - 创建 `AcpService`
     - 将 orchestrator 的 channel handler 从 `create_channel_handler(...)` 替换为 `create_channel_handler_with_acp(...)`
     - 并按 `acp.yaml` 注册 agents + router rules
   - **建议落点**：
     - `Gateway::new()` 或 `Gateway::start()`：调用 `self.factory.create_acp_service(&self.config.acp).await?`
     - 若 ACP service 存在，则在 `ServiceOrchestrator::start()` 构造 handler 时使用 `create_channel_handler_with_acp()`
   - **注意点**：
     - 需要决定 ACP service 的生命周期归属：
       - 放在 `AppContext`（最合理，供 API/handler 复用），或
       - 放在 `ServiceOrchestrator` 内部字段

### P1（高价值稳定性/性能）

2. **将高频 `Regex::new(...).unwrap()` 改为预编译静态 + 明确错误信息**
   - `openclaw-server/src/channel_message_handler.rs`
   - `openclaw-security/src/validator.rs`

3. **把 `openclaw-testing` 的定位明确化**
   - 如果仅用于测试：
     - 确保只在 `dev-dependencies` 或 `cfg(test)` 里引用（避免误导为 runtime 依赖）
   - 如果希望提供 mock runtime：
     - 在 `openclaw-cli` 增加 flag（如 `--mock-ai`）并在 `ServiceFactory` 层切换 provider。

### P2（结构与可维护性）

4. **梳理 `openclaw-agent::Orchestrator` 与 `openclaw-server::ServiceOrchestrator` 的边界**
   - 当前存在两个“orchestrator”概念：
     - `openclaw-agent/src/orchestrator.rs`（团队任务编排）
     - `openclaw-server/src/orchestrator.rs`（服务编排 + ports 注入 + sessions/channels/canvas）
   - 建议：
     - 明确命名（例如 server 侧为 `ServiceOrchestrator` 已做，但对外文档/API 层也要一致）
     - 统一 ports 注入由哪一层负责，避免未来重复注入/状态不一致。

## 7. 下一步可执行检查（可选）

- 生成更细粒度清单：对风险扫描的 592 处匹配按模块分组，输出“文件:行号:片段”。
- 跑一轮 `cargo test` / `cargo clippy` 并收集 warning（需要你允许我运行命令）。

---

## 状态

- 已完成：模块枚举、主流程/依赖关系梳理、集成度判断、发现 ACP 未接入、完成初版风险扫描与修复建议。
