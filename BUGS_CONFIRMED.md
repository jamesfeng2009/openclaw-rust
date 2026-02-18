# 其他潜在问题 — 逐行确认报告

## 1. openclaw-device：`output_path.to_str().unwrap()`

**结论：均为生产代码，路径含非 UTF-8 时可能 panic，需按错误返回或 lossy 处理。**

### camera.rs

| 行号 | 所在函数 | 路径来源 | 确认 |
|------|----------|----------|------|
| 35 | `capture_photo` (macos) | `output_dir.join(format!("photo_{}.jpg", timestamp))`，`output_dir = /tmp/openclaw` | 生产；`to_str()` 在非 UTF-8 路径上为 `None` → unwrap panic |
| 83 | `capture_photo` (linux) | 同上 | 同上 |
| 143 | `capture_photo` (windows) | `std::env::temp_dir().join("openclaw").join(format!(...))` | 生产；Windows 临时目录可为非 UTF-8（如用户名为非 UTF-8）→ 有实际风险 |
| 213 | `start_recording` (macos) | `output_dir.join(format!("video_{}.mov", timestamp))` | 生产；同上 |
| 268 | `start_recording` (linux, spawn_blocking) | `output_path_clone.to_str().unwrap()`，同目录规则 | 生产；同上 |

### screen.rs

| 行号 | 所在函数 | 路径来源 | 确认 |
|------|----------|----------|------|
| 31 | `screenshot` (macos) | `output_dir.join(format!("screen_{}.png", timestamp))`，`/tmp/openclaw` | 生产；同上 |
| 91 | `screenshot` (linux, import) | 同上 | 生产；同上 |
| 96 | `screenshot` (linux, gnome-screenshot) | 同上 | 生产；同上 |
| 100 | `screenshot` (linux, scrot) | 同上 | 生产；同上 |
| 160 | `screenshot` (windows) | `env::temp_dir().join("openclaw").join(...)` | 生产；非 UTF-8 临时目录有实际风险 |
| 234 | `start_recording` (macos, spawn_blocking) | `output_path_clone`，同目录 | 生产；同上 |
| 302 | `start_recording` (linux, spawn_blocking) | 同上 | 生产；同上 |
| 353 | `start_recording` (windows, spawn_blocking) | `output_path_clone.to_str().unwrap().replace(...)` | 生产；同上 |

**说明**：当前路径均由代码用 ASCII + 数字拼接，在常见环境下多为 UTF-8；但 `PathBuf` 不保证 UTF-8，且 Windows 上 `env::temp_dir()` 可能含非 UTF-8 字符，故属真实（尤其 Windows）风险。

---

## 2. openclaw-vector/store/memory.rs：`.unwrap()`

**结论：前半部分为生产 API 的锁/结果处理，后半部分为测试内 unwrap。**

### 生产代码（impl VectorStore for MemoryStore）

| 行号 | 代码 | 说明 | 确认 |
|------|------|------|------|
| 90 | `self.data.write().unwrap()` | `RwLock::write()` 返回 `LockResult`；`unwrap()` 仅在锁被 poison 时 panic（即之前某次持有锁时发生 panic） | 生产；属“毒化传播”的常规写法，可接受；若不想传播可 `map_err` 或恢复 |
| 96 | 同上 (upsert_batch) | 同上 | 同上 |
| 105 | `self.data.read().unwrap()` | 同上 (read) | 同上 |
| 145 | 同上 (get) | 同上 | 同上 |
| 150 | `self.data.write().unwrap()` (delete) | 同上 | 同上 |
| 156 | 同上 (delete_by_filter) | 同上 | 同上 |
| 171 | 同上 (stats, read) | 同上 | 同上 |
| 180 | 同上 (clear, write) | 同上 | 同上 |

文件头注释为「用于测试和开发」，上述 unwrap 为生产 API 的一部分，但仅在线程 panic 导致锁毒化时才会触发，属于已知模式。

### 测试代码（#[cfg(test)] mod tests）

| 行号 | 代码 | 说明 | 确认 |
|------|------|------|------|
| 198 | `store.upsert(item).await.unwrap()` | 测试中若 upsert 失败则测试 panic | 仅影响测试稳定性 |
| 202 | `store.search(query).await.unwrap()` | 测试中若 search 失败则测试 panic | 仅影响测试稳定性 |

---

## 3. openclaw-tools/skill_bundle.rs：`.unwrap()`

| 行号 | 代码 | 所在位置 | 确认 |
|------|------|----------|------|
| 239 | `archive.by_index(i).unwrap()` | `pub async fn from_archive(archive_path: &Path)` 内循环 | **生产代码**。损坏或异常 zip 可能导致 `by_index` 返回 `Err`，此处 unwrap 会直接 panic，建议改为 `?` 或 `map_err` 返回 `BundleError`。 |
| 877 | `manager.search_marketplace("web").await.unwrap()` | `#[tokio::test] test_search_marketplace_fallback` | 测试；网络/平台失败时测试 panic，仅影响测试稳定性 |
| 892 | `manager.search_marketplace("").await.unwrap()` | `#[tokio::test] test_search_marketplace_empty_query` | 同上 |
| 901 | `manager.get_categories().await.unwrap()` | `#[tokio::test] test_get_categories` | 同上 |

---

## 4. openclaw-memory/workspace.rs：`.unwrap()`

| 行号 | 代码 | 所在位置 | 确认 |
|------|------|----------|------|
| 709 | `NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()` | `#[test] test_daily_memory_path` | 测试；`from_ymd_opt(2024,1,15)` 恒为 `Some`，此处 unwrap 安全，非 bug。 |
| 772 | `workspace.initialize().unwrap()` | `#[test] test_transcripts_operations` | 测试；若初始化失败（权限、磁盘等）则测试 panic，仅影响测试稳定性。 |

---

## 5. openclaw-memory/file_tracker.rs：`.unwrap()`

| 行号 | 代码 | 所在位置 | 确认 |
|------|------|----------|------|
| 199 | `fs::create_dir_all(&temp_dir).unwrap()` | `#[cfg(test)] mod tests` 内 `test_file_tracker` | 测试；创建临时目录失败则测试 panic，仅影响测试稳定性 |
| 202 | `fs::write(&test_file, "Hello World").unwrap()` | 同上 | 测试；同上 |
| 210 | `tracker.track_file(&test_file).unwrap()` | 同上 | 测试；track_file 返回 Err 时测试 panic，仅影响测试稳定性 |

---

## 汇总

| 类别 | 位置 | 是否生产 | 风险/说明 |
|------|------|----------|-----------|
| 路径非 UTF-8 导致 panic | device camera.rs 5 处、screen.rs 8 处 | 是 | 生产 bug；建议改为返回 `Result` 或使用 `to_string_lossy()` 等 |
| 锁毒化 / 锁 unwrap | vector/store/memory.rs 8 处 | 是 | 生产 API，仅在毒化时 panic，可接受；可选改进为不传播 |
| zip 解压 unwrap | tools/skill_bundle.rs 239 | 是 | 生产 bug；损坏 zip 会 panic，建议用 `?` 或显式错误类型 |
| 测试内 unwrap | memory.rs 198,202；skill_bundle 877,892,901；workspace 772；file_tracker 199,202,210 | 否 | 仅影响测试稳定性；可改为 `expect` 或断言 + 错误信息以便排查 |
| 测试内安全 unwrap | workspace 709 | 否 | 非 bug |
