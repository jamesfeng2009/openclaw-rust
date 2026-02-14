# API Key 配置指南

## 🚀 快速开始

### 1. 设置 API Key

```bash
# 设置 OpenAI API Key
openclaw-rust api-key set openai sk-proj-xxxxx

# 设置 Anthropic API Key
openclaw-rust api-key set anthropic sk-ant-xxxxx

# 设置 Gemini API Key
openclaw-rust api-key set gemini AIzaSyxxxxx

# 设置国内提供商
openclaw-rust api-key set glm your-glm-api-key
openclaw-rust api-key set qwen your-qwen-api-key
openclaw-rust api-key set deepseek your-deepseek-api-key
openclaw-rust api-key set kimi your-kimi-api-key
openclaw-rust api-key set minimax your-minimax-api-key
```

### 2. 查看配置

```bash
# 列出所有提供商
openclaw-rust api-key list

# 查看特定提供商
openclaw-rust api-key get openai

# 导出配置（隐藏敏感信息）
openclaw-rust api-key export
```

### 3. 管理提供商

```bash
# 设置默认提供商
openclaw-rust api-key default openai

# 删除提供商配置
openclaw-rust api-key remove deepseek

# 验证 API Key 格式
openclaw-rust api-key validate openai sk-test
```

---

## 📁 配置文件位置

默认配置文件路径：`~/.openclaw/user_config.json`

### 配置文件示例

```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_name": "default",
  "default_provider": "openai",
  "providers": {
    "openai": {
      "name": "openai",
      "api_key": "sk-proj-xxxxx",
      "base_url": null,
      "default_model": "gpt-4o-mini",
      "enabled": true,
      "quota": null
    },
    "anthropic": {
      "name": "anthropic",
      "api_key": "sk-ant-xxxxx",
      "base_url": null,
      "default_model": "claude-3-5-sonnet-20241022",
      "enabled": true,
      "quota": null
    }
  },
  "preferences": {
    "language": "zh-CN",
    "timezone": "Asia/Shanghai",
    "temperature": 0.7,
    "max_tokens": 4096,
    "stream_response": true,
    "notifications": {
      "enabled": true,
      "on_error": true,
      "on_quota_warning": true,
      "quota_warning_threshold": 0.8
    }
  },
  "created_at": "2026-02-14T10:30:00Z",
  "updated_at": "2026-02-14T10:30:00Z"
}
```

---

## 🎯 高级配置

### 自定义 Base URL

```bash
# 使用自定义 OpenAI 兼容端点
openclaw api-key set custom-provider your-api-key \
  --url https://your-custom-endpoint.com/v1 \
  --model your-default-model
```

### 设置配额限制

配置文件中可以设置配额：

```json
{
  "providers": {
    "openai": {
      "name": "openai",
      "api_key": "sk-xxxxx",
      "default_model": "gpt-4o-mini",
      "enabled": true,
      "quota": {
        "daily_requests": 100,
        "monthly_tokens": 1000000,
        "used_requests": 10,
        "used_tokens": 5000,
        "reset_date": "2026-03-01T00:00:00Z"
      }
    }
  }
}
```

---

## 🔐 安全最佳实践

### 1. 文件权限

```bash
# 设置配置文件权限（仅当前用户可读写）
chmod 600 ~/.openclaw/user_config.json
```

### 2. 环境变量（推荐）

创建 `.env` 文件（加入 `.gitignore`）：

```bash
# .env
OPENAI_API_KEY=sk-proj-xxxxx
ANTHROPIC_API_KEY=sk-ant-xxxxx
GLM_API_KEY=xxxxx
QWEN_API_KEY=xxxxx
```

然后在应用中优先读取环境变量：

```rust
use openclaw_core::UserConfigManager;

let manager = UserConfigManager::new(None)?;

// 优先从环境变量读取
if let Ok(key) = std::env::var("OPENAI_API_KEY") {
    manager.set_api_key("openai".to_string(), key, None)?;
}
```

### 3. API Key 格式验证

```bash
# 验证 API Key 格式
openclaw-rust api-key validate openai sk-test
openclaw-rust api-key validate anthropic sk-ant-test
```

### 2. 环境变量（推荐）

创建 `.env` 文件（加入 `.gitignore`）：

```bash
# .env
OPENAI_API_KEY=sk-proj-xxxxx
ANTHROPIC_API_KEY=sk-ant-xxxxx
GLM_API_KEY=xxxxx
QWEN_API_KEY=xxxxx
```

然后在应用中优先读取环境变量：

```rust
use openclaw_core::UserConfigManager;

let manager = UserConfigManager::new(None)?;

// 优先从环境变量读取
if let Ok(key) = std::env::var("OPENAI_API_KEY") {
    manager.set_api_key("openai".to_string(), key, None)?;
}
```

### 3. API Key 格式验证

```bash
# 验证 API Key 格式
openclaw api-key validate openai sk-test
openclaw api-key validate anthropic sk-ant-test
```

---

## 📊 支持的提供商

| 提供商 | 名称 | API Key 格式 | 默认模型 |
|--------|------|-------------|---------|
| OpenAI | `openai` | `sk-*` | gpt-4o-mini |
| Anthropic | `anthropic` | `sk-ant-*` | claude-3-5-sonnet |
| Google Gemini | `gemini` | 39 字符 | gemini-2.0-flash |
| DeepSeek | `deepseek` | `sk-*` | deepseek-chat |
| 智谱 GLM | `glm` | 任意 | glm-4-flash |
| 通义千问 | `qwen` | 任意 | qwen-plus |
| Kimi | `kimi` | 任意 | moonshot-v1-8k |
| Minimax | `minimax` | 任意 | abab6.5s-chat |

---

## 🎨 使用示例

### 在代码中使用

```rust
use openclaw_core::UserConfigManager;
use openclaw_ai::providers::OpenAIProvider;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载用户配置
    let manager = UserConfigManager::new(None)?;
    
    // 获取 OpenAI API Key
    let api_key = manager.get_api_key("openai")
        .expect("OpenAI API Key not configured");
    
    // 创建 provider
    let config = ProviderConfig::new("openai", api_key);
    let provider = OpenAIProvider::new(config);
    
    // 使用 provider
    let response = provider.chat(request).await?;
    
    Ok(())
}
```

---

## 🔧 故障排除

### 问题：配置文件找不到

```bash
# 初始化配置
openclaw-rust init
```

### 问题：API Key 格式错误

```bash
# 验证格式
openclaw-rust api-key validate openai your-key

# 查看已配置的 key（部分隐藏）
openclaw-rust api-key get openai
```

### 问题：权限错误

```bash
# 修复文件权限
chmod 600 ~/.openclaw/user_config.json
chown $USER:$USER ~/.openclaw/user_config.json
```

---

## 📝 更多资源

- [完整文档](https://github.com/openclaw/openclaw-rust)
- [问题反馈](https://github.com/openclaw/openclaw-rust/issues)
- [贡献指南](https://github.com/openclaw/openclaw-rust/blob/main/CONTRIBUTING.md)
