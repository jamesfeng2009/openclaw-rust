# OpenClaw-Rust 已确认 Bug 列表

以下为逐项确认的 bug，仅记录问题不包含修复方案。

---

## 一、逻辑 / 数据一致性

### 1. openclaw-memory/manager.rs — 长期记忆 payload 与检索字段

- **位置**：`archive_to_long_term` 写入的 payload 与 `retrieve` / `hybrid_search` 读取的 key 必须一致。
- **现状**：若归档使用 `"text_preview"` 而检索使用 `payload.get("content")`，则长期记忆预览始终为空；若已统一为 `"content"` 则无此问题（请以当前代码为准核对）。

### 2. openclaw-memory/pruning.rs — `limit_session_count` 受保护会话多时删不够

- **位置**：约 168–193 行。
- **问题**：先取“最旧 excess 个”再按保护规则过滤；若其中多数被保护则实际删除数 < excess，删除后会话数仍可能 > max_sessions。
- **结果**：无法保证“会话数不超过 max_sessions”。

### 3. openclaw-memory/pruning.rs — `prune_session_messages` 与注释不符

- **位置**：约 221–238 行。
- **问题**：注释为“按重要性排序，保留重要消息”，实现未按重要性排序，仅按原顺序遍历并删除先遇到的非保护消息。
- **结果**：删除的是“顺序靠前的非保护消息”，不一定是“最不重要的”。

### 4. openclaw-agent/sessions.rs — `get_or_create_session` 的 key 与 `Session.key()` 一致性与参数

- **问题**：查找 key 必须与 `Session.key()` 完全一致；若使用 `session_key_from_parts(scope, channel_type, account_id, peer_id)` 且 `create_session` 也接收 `account_id`，则一致；否则会找不到已有会话或创建重复会话。
- **建议**：确认当前实现是否已统一 key 构建并暴露 `account_id` 参数。

---

## 二、生产代码中的 panic / unwrap / expect

### 5. openclaw-core/config_loader.rs — 测试中依赖固定数据结构

- **位置**：约 389 行。
- **问题**：`config.ai.providers.iter().find(|p| p.name == "openai").unwrap()` 在测试中；若测试数据无 "openai" 会 panic。
- **范围**：仅测试，但测试脆弱。

### 6. openclaw-agent/integration.rs — 创建 Provider 失败时 panic

- **位置**：约 73、85、104 行。
- **问题**：`create_openrouter_provider`、`create_ollama_provider`、`create_provider` 在 `ProviderFactory::create` 返回 `Err` 时执行 `panic!(...)`。
- **结果**：生产代码路径下（如配置错误、网络不可用）会导致进程退出。
- **建议**：返回 `Result` 或 `Arc<dyn AIProvider>` 的 fallback，由调用方处理错误。

### 7. openclaw-agent/memory_pipeline.rs — Option 的 unwrap

- **位置**：约 163–164 行。
- **问题**：在 `sync_workspace` 中先判断 `workspace.is_none() || file_tracker.is_none()` 后仍使用 `self.workspace.as_ref().unwrap()`、`self.file_tracker.as_mut().unwrap()`。
- **结果**：逻辑上安全但风格不佳；若后续修改分支逻辑易引入 panic。建议改为 `if let Some(ws) = &self.workspace` 等。

### 8. openclaw-voice/provider/mod.rs — 枚举匹配 panic

- **位置**：约 386、415 行。
- **问题**：`_ => panic!("Expected Json variant")`，在收到非预期 JSON 结构时进程退出。
- **结果**：生产路径下异常响应可能导致服务崩溃。

### 9. openclaw-ai/failover.rs — partial_cmp unwrap

- **位置**：约 357、376 行。
- **问题**：`a_latency.partial_cmp(&b_latency).unwrap()` 等；若出现 NaN 会 panic。
- **结果**：延迟统计异常时可能触发 panic。

### 10. openclaw-security/pipeline.rs — 测试中 panic

- **位置**：约 280 行。
- **问题**：测试里 `_ => panic!("Expected Block result for high threat input")`。
- **范围**：仅测试，但测试假设固定行为。

---

## 三、设备 / 路径 / 编码

### 11. openclaw-device (camera.rs / screen.rs) — 路径非 UTF-8 时 panic

- **位置**：camera.rs 多处、screen.rs 多处。
- **问题**：`output_path.to_str().unwrap()`；路径含非 UTF-8 时 `to_str()` 为 `None`，unwrap 会 panic。
- **风险**：Windows 临时目录或用户名含非 UTF-8 时易触发。
- **建议**：改为返回 `Result` 或使用 `to_string_lossy()` 等安全方式。

### 12. openclaw-tools/skill_bundle.rs — 解压时 panic

- **位置**：约 239 行，`from_archive` 循环内。
- **问题**：`archive.by_index(i).unwrap()`；损坏或异常 zip 会返回 `Err`，导致 panic。
- **建议**：使用 `?` 或 `map_err` 返回 `BundleError`。

---

## 四、会话 / 记忆与主流程

### 13. 对话未写入记忆（Agent process 不调用 memory.add）

- **位置**：openclaw-agent BaseAgent::process。
- **问题**：process 只使用 `memory.get_context()` 读工作记忆，**从未调用 `memory.add()`** 写入当前用户消息或助手回复。
- **结果**：通过 API 的对话不会进入 MemoryManager，多轮上下文仅依赖当次请求的 working memory（且 working memory 未被更新），会话历史无法持久化到记忆层。

### 14. 通过 API 动态创建的 Agent 未注入依赖

- **位置**：openclaw-server api.rs `create_agent`。
- **问题**：`POST /api/agents` 使用 `BaseAgent::from_type(...)` 创建新 Agent 并 `register_agent`，**未调用** `inject_dependencies`（AI/Memory/Security）。
- **结果**：动态创建的 Agent 没有 AI provider，`process` 会返回 “No AI provider configured”。

---

## 五、测试与示例中的 unwrap/panic

- **openclaw-memory**：manager/recall 等测试中 `.await.unwrap()`；测试失败时直接 panic。
- **openclaw-vector/store/memory.rs**：生产 impl 中 `RwLock::read/write().unwrap()` 仅在锁毒化时 panic，属常见写法；测试中 `store.upsert(...).unwrap()` 等仅影响测试稳定性。
- **openclaw-agent/sessions.rs**：测试中多处 `.unwrap()`。
- **openclaw-voice/wake.rs**：测试中 `result.unwrap()`，若检测不到唤醒词会 panic。
- **openclaw-memory (workspace, file_tracker, bm25)**：测试中 `unwrap()`，仅影响测试稳定性。
- **openclaw-device/registry**：测试中 `unwrap()`。
- **openclaw-core (user_config, group_context, config)**：测试中 `unwrap()`。
- **openclaw-cli/channel_cmd**：`parse_key_value(...).unwrap()`，若在非测试路径调用且输入不合法会 panic。

---

## 六、其他潜在问题

### 15. openclaw-memory/working.rs — 压缩 overflow 计算

- **位置**：约 31–36 行。
- **问题**：`overflow = items.len().saturating_sub(self.config.max_messages / 2)`；当 `max_messages` 为奇数时整数除法可能使单次压缩量偏大；且与 token 上限的协同策略需与设计一致。

### 16. openclaw-memory/hybrid_search.rs — 硬编码向量维度

- **位置**：约 95–96 行。
- **问题**：`get_all_items` 使用 `vec![0.0; 128]` 作为 dummy 向量；若实际 embedding 维度非 128（如 1536），部分 VectorStore 实现可能报错或行为异常。

### 17. openclaw-security/input_filter.rs — 检测到威胁仍返回 allowed: true

- **位置**：约 144–149 行。
- **问题**：检测到黑名单或正则命中时仍设置 `allowed: true`，仅填充 `threat_level`、`sanitized_input` 等。
- **结果**：若调用方仅根据 `allowed` 放行，危险输入会被放行；需依赖 `check_strict` 或明确文档说明语义。

---

以上为当前已确认的 bug 与风险点列表；若某条已在代码中修复，请以实际代码为准并从列表中勾销。
