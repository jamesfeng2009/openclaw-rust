//! ACP Error Types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AcpError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Context error: {0}")]
    Context(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("Router error: {0}")]
    Router(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Capability error: {0}")]
    Capability(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

pub type AcpResult<T> = Result<T, AcpError>;
