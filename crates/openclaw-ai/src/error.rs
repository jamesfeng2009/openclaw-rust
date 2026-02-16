pub type Result<T> = std::result::Result<T, OpenClawError>;

#[derive(Debug, thiserror::Error)]
pub enum OpenClawError {
    #[error("Tool error: {0}")]
    Tool(String),
    
    #[error("Provider error: {0}")]
    Provider(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("Unknown error: {0}")]
    Unknown(String),
}
