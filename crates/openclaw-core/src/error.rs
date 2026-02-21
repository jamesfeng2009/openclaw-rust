//! 统一错误处理

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OpenClawError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("AI 提供商错误: {0}")]
    AIProvider(String),

    #[error("向量存储错误: {0}")]
    VectorStore(String),

    #[error("消息通道错误: {0}")]
    Channel(String),

    #[error("会话错误: {0}")]
    Session(String),

    #[error("记忆存储错误: {0}")]
    Memory(String),

    #[error("Token 计数错误: {0}")]
    TokenCount(String),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("HTTP 请求错误: {0}")]
    Http(String),

    #[error("网络错误: {0}")]
    Network(String),

    #[error("API 错误: {0}")]
    Api(String),

    #[error("解析错误: {0}")]
    Parse(String),

    #[error("执行错误: {0}")]
    Execution(String),

    #[error("平台不支持: {0}")]
    Platform(String),

    #[error("工具错误: {0}")]
    Tool(String),

    #[error("未知错误: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, OpenClawError>;
