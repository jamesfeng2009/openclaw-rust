# OpenClaw Rust

🤖 **OpenClaw Rust** - 你的个人 AI 助手 (Rust 实现)

一个功能丰富、模块化的 AI 助手平台，支持多种 AI 提供商、语音交互、实时协作画布、浏览器控制等功能。

## ✨ 功能特性

### 🧠 AI 能力
- **多提供商支持**: OpenAI, Anthropic (Claude), Google (Gemini), DeepSeek, 通义千问, 智谱 GLM, Moonshot (Kimi)
- **流式响应**: 支持流式输出，实时显示 AI 回复
- **工具调用**: 支持 Function Calling
- **嵌入向量**: 支持文本嵌入生成

### 🧠 记忆系统
- **三层记忆架构**:
  - 工作记忆: 最近消息，高优先级
  - 短期记忆: 压缩摘要，中等优先级
  - 长期记忆: 向量存储，持久化检索

### 🤖 多智能体
- **Agent 类型**: Orchestrator, Researcher, Coder, Writer, Conversationalist
- **任务编排**: 自动任务分解和多 Agent 协作

### 📡 消息通道
支持 10+ 个主流平台的消息收发:
- 国际: Telegram, Discord, Slack, Microsoft Teams, WhatsApp, **Signal**
- 国内: 钉钉, 企业微信, 飞书
- macOS: **iMessage** (Apple 消息服务)

### 🎙️ 语音交互
- **STT (语音识别)**: OpenAI Whisper, 本地 Whisper
- **TTS (语音合成)**: OpenAI TTS, Edge TTS
- **持续对话模式**: 实时语音交互
- **语音唤醒**: 支持唤醒词检测

### 🎨 实时画布
- **A2UI 可视化工作空间**: 完整的图形类型系统
- **实时协作**: 用户加入/离开, 光标同步, WebSocket 事件广播
- **绘图工具**: 路径绘制, 形状创建, 历史记录, 元素选择

### 🌐 浏览器控制
- **Chrome/Chromium 实例控制**: 启动/关闭浏览器池
- **页面操作**: 导航, 点击, 输入, 截图, PDF 生成
- **Puppeteer 风格 API**: 基于 chromiumoxide

### 🔐 安全沙箱
- **Docker 沙箱运行**: 容器创建, 资源限制, 安全配置
- **权限管理系统**: 角色, 用户, ACL, 权限检查

### 🛠️ 工具生态
- **浏览器工具**: 导航, 点击, 输入, 截图
- **定时任务**: 任务创建, 执行, 管理
- **Webhook 系统**: 事件触发, 签名验证
- **技能平台**: 触发器, 工具绑定, 执行管理
- **技能捆绑**: 技能包, 工作区技能, 技能市场

### 🖥️ CLI 工具
- **wizard**: 交互式设置向导
- **doctor**: 系统健康检查与自动修复
- **gateway**: 启动 HTTP/WebSocket 服务
- **daemon**: 后台守护进程服务 (支持开机自启动)

## 📦 项目结构

```
openclaw-rust/
├── crates/
│   ├── openclaw-core      # 核心类型和配置
│   ├── openclaw-ai        # AI 提供商抽象层
│   ├── openclaw-memory    # 分层记忆系统
│   ├── openclaw-vector    # 向量存储抽象层
│   ├── openclaw-channels  # 消息通道集成
│   ├── openclaw-agent     # 多智能体系统
│   ├── openclaw-voice     # 语音识别与合成
│   ├── openclaw-server    # HTTP/WebSocket 服务
│   ├── openclaw-canvas    # 实时协作画布
│   ├── openclaw-browser   # 浏览器控制
│   ├── openclaw-sandbox   # Docker 沙箱
│   ├── openclaw-tools     # 工具生态
│   └── openclaw-cli       # 命令行工具
├── Cargo.toml
└── README.md
```

## 🚀 快速开始

### 安装依赖

```bash
# 克隆项目
git clone https://github.com/openclaw/openclaw-rust.git
cd openclaw-rust

# 构建项目
cargo build --release
```

### 运行设置向导

```bash
# 交互式配置
cargo run -- wizard

# 快速模式 (跳过可选步骤)
cargo run -- wizard --quick

# 强制覆盖现有配置
cargo run -- wizard --force
```

### 系统健康检查

```bash
# 检查系统状态
cargo run -- doctor

# 自动修复问题
cargo run -- doctor --fix

# 详细输出
cargo run -- doctor --verbose
```

### 启动服务

```bash
# 启动 Gateway 服务
cargo run -- gateway

# 指定端口和主机
cargo run -- gateway --port 8080 --host 0.0.0.0

# 启用详细日志
cargo run -- gateway --verbose
```

### Daemon 后台服务

```bash
# 启动后台守护进程
cargo run -- daemon start

# 查看守护进程状态
cargo run -- daemon status

# 停止守护进程
cargo run -- daemon stop

# 安装为系统服务 (开机自启动)
cargo run -- daemon install

# 卸载系统服务
cargo run -- daemon uninstall
```

## 📡 API 端点

### 基础 API

| 端点 | 方法 | 功能 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/chat` | POST | 聊天对话 |
| `/models` | GET | 列出可用模型 |
| `/stats` | GET | 获取统计信息 |

### 画布 API

| 端点 | 方法 | 功能 |
|------|------|------|
| `/canvas` | POST | 创建画布 |
| `/canvas` | GET | 列出画布 |
| `/canvas/{id}` | GET | 获取画布 |
| `/canvas/{id}` | DELETE | 删除画布 |
| `/canvas/{id}/elements` | POST | 添加元素 |
| `/canvas/{id}/ws` | GET | WebSocket 实时协作 |

### 浏览器 API

| 端点 | 方法 | 功能 |
|------|------|------|
| `/browser` | POST | 创建浏览器实例 |
| `/browser/{id}/page` | POST | 创建新页面 |
| `/page/{id}/goto` | POST | 导航到 URL |
| `/page/{id}/click` | POST | 点击元素 |
| `/page/{id}/type` | POST | 输入文本 |
| `/page/{id}/screenshot` | POST | 截图 |
| `/page/{id}/pdf` | POST | 生成 PDF |

## ⚙️ 配置

配置文件位于 `~/.openclaw/openclaw.json`:

```json
{
  "user_name": "User",
  "default_provider": "openai",
  "default_model": "gpt-4o",
  "api_keys": {
    "OPENAI_API_KEY": "sk-..."
  },
  "enabled_features": ["chat", "voice"],
  "voice_enabled": true,
  "voice_provider": "openai",
  "browser_headless": true,
  "sandbox_enabled": false
}
```

### 环境变量

```bash
# API 密钥
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."

# 服务配置
export OPENCLAW_PORT=18789
export OPENCLAW_HOST=0.0.0.0
```

## 🔧 开发

### 运行测试

```bash
cargo test
```

### 代码检查

```bash
cargo clippy
cargo fmt --check
```

### 文档生成

```bash
cargo doc --open
```

## 📋 系统要求

- Rust 1.93+
- Docker (可选，用于沙箱功能)
- Chrome/Chromium (可选，用于浏览器控制)

## 🤝 贡献

欢迎贡献！请查看 [贡献指南](CONTRIBUTING.md)。

## 📄 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

## 🙏 致谢

- [async-openai](https://github.com/64bit/async-openai) - OpenAI API 客户端
- [chromiumoxide](https://github.com/mattsse/chromiumoxide) - Chrome DevTools Protocol 客户端
- [bollard](https://github.com/fussybeaver/bollard) - Docker API 客户端
- [axum](https://github.com/tokio-rs/axum) - Web 框架

---

**OpenClaw Rust** - 让 AI 助手更简单、更强大
