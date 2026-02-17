# 通道使用指南

## 钉钉 (DingTalk)

### 1. 创建钉钉机器人

#### 步骤1：在钉钉群中添加机器人

1. 打开钉钉群聊
2. 点击群设置（右上角 ...）
3. 选择"智能群助手"
4. 点击"添加机器人"
5. 选择"自定义"机器人

#### 步骤2：配置机器人

1. 填写机器人名称
2. 选择安全设置（三选一）：
   - **自定义关键词**：消息中包含关键词才能发送
   - **加签**：推荐使用，更安全
   - **IP地址（段）**：限制发送IP

3. 点击"完成"，获取 Webhook 地址

#### 步骤3：配置 OpenClaw Rust

编辑 `~/.openclaw-rust/openclaw.json`:

```json
{
  "channels": {
    "dingtalk": {
      "webhook": "https://oapi.dingtalk.com/robot/send?access_token=你的token",
      "secret": "加签密钥（如果启用了加签）",
      "enabled": true
    }
  }
}
```

### 2. 使用示例

#### 发送文本消息

```bash
curl -X POST https://oapi.dingtalk.com/robot/send?access_token=xxx \
  -H 'Content-Type: application/json' \
  -d '{
    "msgtype": "text",
    "text": {
        "content": "我就是我, 是不一样的烟火"
    }
  }'
```

#### 发送 Markdown 消息

```bash
curl -X POST https://oapi.dingtalk.com/robot/send?access_token=xxx \
  -H 'Content-Type: application/json' \
  -d '{
    "msgtype": "markdown",
    "markdown": {
        "title": "杭州天气",
        "text": "#### 杭州天气\n> 9度，西北风1级，空气良89，相对温度73%\n\n> ![screenshot](https://img.alicdn.com/tfs/TB1NwmBEL9TBuNjy1zbXXXpepXa-2400-1218.png)\n> ###### 10点20分发布 [天气](https://www.dingtalk.com) \n"
    }
  }'
```

#### 发送链接消息

```bash
curl -X POST https://oapi.dingtalk.com/robot/send?access_token=xxx \
  -H 'Content-Type: application/json' \
  -d '{
    "msgtype": "link",
    "link": {
        "text": "这个即将发布的新版本，创始人xx称它为红树林。而在此之前，每当提到创新赋能，大家都会想到创新赋能，大概就是这个意思。",
        "title": "时代的火车向前开",
        "picUrl": "",
        "messageUrl": "https://www.dingtalk.com/s?__biz=MzA4NjMwMTA2Ng==&mid=2650316842&idx=1&sn=9da0f26d7b4a1e7c7a3a1e7c7a3a1e7c"
    }
  }'
```

### 3. 钉钉消息类型

| 消息类型 | msgtype | 说明 |
|---------|---------|------|
| 文本消息 | text | 纯文本 |
| Markdown | markdown | Markdown 格式 |
| 链接消息 | link | 带链接和图片 |
| ActionCard | actionCard | 交互卡片 |
| FeedCard | feedCard | 多条图文 |

---

## 企业微信 (WeCom)

### 1. 创建企业微信机器人

#### 步骤1：在群聊中添加机器人

1. 打开企业微信群聊
2. 点击群聊右上角"..."
3. 选择"群机器人"
4. 点击"添加机器人"
5. 填写机器人名称

#### 步骤2：获取 Webhook 地址

创建完成后，会显示 Webhook 地址：

```
https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx-xxx-xxx-xxx
```

#### 步骤3：配置 OpenClaw Rust

编辑 `~/.openclaw-rust/openclaw.json`:

```json
{
  "channels": {
    "wecom": {
      "webhook": "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=你的key",
      "enabled": true
    }
  }
}
```

### 2. 使用示例

#### 发送文本消息

```bash
curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx' \
   -H 'Content-Type: application/json' \
   -d '
   {
        "msgtype": "text",
        "text": {
            "content": "你好，我是机器人"
        }
   }'
```

#### 发送 Markdown 消息

```bash
curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx' \
   -H 'Content-Type: application/json' \
   -d '
   {
        "msgtype": "markdown",
        "markdown": {
            "content": "实时新增用户反馈<font color=\"warning\">132例</font>，请相关同事注意。\n>类型:<font color=\"comment\">用户反馈</font>\n>普通用户反馈:<font color=\"comment\">117例</font>\n>VIP用户反馈:<font color=\"comment\">15例</font>"
        }
   }'
```

#### 发送图文消息

```bash
curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx' \
   -H 'Content-Type: application/json' \
   -d '
   {
        "msgtype": "news",
        "news": {
           "articles" : [
               {
                   "title" : "中秋节礼品领取",
                   "description" : "今年中秋节公司有豪礼相送",
                   "url" : "www.qq.com",
                   "picurl" : "http://res.mail.qq.com/node/ww/wwopenmng/images/independent/doc/test_pic_msg1.png"
               }
            ]
        }
   }'
```

#### 发送图片消息

```bash
# 首先需要计算图片的 MD5 和 Base64
# 然后发送

curl 'https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx' \
   -H 'Content-Type: application/json' \
   -d '
   {
        "msgtype": "image",
        "image": {
            "base64": "图片的base64编码",
            "md5": "图片的md5值"
        }
   }'
```

### 3. 企业微信消息类型

| 消息类型 | msgtype | 说明 |
|---------|---------|------|
| 文本消息 | text | 纯文本，支持@成员 |
| Markdown | markdown | Markdown 格式 |
| 图片消息 | image | 图片（Base64） |
| 图文消息 | news | 多条图文 |
| 文件消息 | file | 文件（需要 media_id） |

---

## 飞书 (Feishu) - 即将支持

飞书支持更强大的机器人功能，包括：
- 丰富的卡片消息
- 交互式组件
- 完整的 Bot API

---

## 配置示例

### 完整配置文件示例

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 18789,
    "log_level": "info"
  },
  "ai": {
    "default_provider": "openai",
    "providers": []
  },
  "channels": {
    "dingtalk": {
      "webhook": "https://oapi.dingtalk.com/robot/send?access_token=xxx",
      "secret": "SECxxx",
      "enabled": true
    },
    "wecom": {
      "webhook": "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=xxx",
      "enabled": true
    }
  }
}
```

---

## 常见问题

### 1. 钉钉加签失败

**问题：** 返回错误 "sign not match"

**解决方案：**
- 检查 secret 是否正确
- 确保时间戳在 1 小时内
- 检查签名算法是否正确

### 2. 企业微信消息发送失败

**问题：** 返回错误 "invalid webhook"

**解决方案：**
- 检查 webhook 地址是否正确
- 确认机器人未被删除
- 检查网络连接

### 3. 消息频率限制

**钉钉：** 
- 每分钟最多 20 条消息
- 超出限制会返回错误

**企业微信：**
- 每分钟最多 20 条消息
- 超出限制会返回错误

---

## 最佳实践

### 1. 消息格式化

- 使用 Markdown 提升可读性
- 合理使用 @ 提醒
- 图片和文字结合

### 2. 错误处理

- 捕获 API 错误
- 实现重试机制
- 记录日志

### 3. 安全性

- 不要硬编码 webhook
- 使用环境变量或配置文件
- 定期更新密钥

---

## API 参考

### 钉钉 API 文档

- [机器人开发文档](https://open.dingtalk.com/document/orgapp/overview-of-group-robots)
- [消息类型说明](https://open.dingtalk.com/document/orgapp/types-of-messages-sent-by-robots)

### 企业微信 API 文档

- [群机器人配置说明](https://developer.work.weixin.qq.com/document/path/91770)
- [消息类型说明](https://developer.work.weixin.qq.com/document/path/91770)

---

## 更新日志

### v0.1.0 (2026-02-14)

- ✅ 新增钉钉通道支持
- ✅ 新增企业微信通道支持
- ✅ 支持文本、Markdown、图片、图文等消息类型
