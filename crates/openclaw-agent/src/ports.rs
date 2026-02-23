use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct RecallItem {
    pub entry: MemoryEntry,
    pub score: f32,
}

#[async_trait]
pub trait AIPort: Send + Sync {
    async fn chat(&self, messages: Vec<openclaw_core::Message>) -> Result<String>;

    async fn chat_stream(
        &self,
        messages: Vec<openclaw_core::Message>,
    ) -> Result<Box<dyn futures::Stream<Item = Result<String>> + Send + Sync>>;

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

#[async_trait]
pub trait MemoryPort: Send + Sync {
    async fn add(&self, entry: MemoryEntry) -> Result<()>;

    async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;

    async fn recall(&self, context: &str, limit: usize) -> Result<Vec<RecallItem>>;

    async fn get_context(&self) -> Result<Vec<openclaw_core::Message>>;
}

#[async_trait]
pub trait SecurityPort: Send + Sync {
    async fn check(&self, input: &str) -> Result<SecurityCheckResult>;
}

#[async_trait]
pub trait ToolPort: Send + Sync {
    async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value>;

    async fn list_tools(&self) -> Result<Vec<ToolInfo>>;
}

#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct ScreenInfo {
    pub id: String,
    pub name: String,
    pub resolution: Option<(u32, u32)>,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct LocationInfo {
    pub id: String,
    pub available: bool,
}

#[async_trait]
pub trait DevicePort: Send + Sync {
    async fn list_cameras(&self) -> Result<Vec<CameraInfo>>;
    async fn capture_camera(&self, camera_id: &str, path: &str) -> Result<String>;

    async fn list_screens(&self) -> Result<Vec<ScreenInfo>>;
    async fn capture_screen(&self, screen_id: &str, path: &str) -> Result<String>;

    async fn get_location(&self) -> Result<LocationInfo>;
    async fn start_location_tracking(&self) -> Result<()>;
    async fn stop_location_tracking(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry {
            id: "test-id".to_string(),
            content: "test content".to_string(),
            metadata: HashMap::new(),
        };
        assert_eq!(entry.id, "test-id");
        assert_eq!(entry.content, "test content");
    }

    #[test]
    fn test_security_check_result_allowed() {
        let result = SecurityCheckResult {
            allowed: true,
            reason: None,
        };
        assert!(result.allowed);
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_security_check_result_blocked() {
        let result = SecurityCheckResult {
            allowed: false,
            reason: Some("malicious content".to_string()),
        };
        assert!(!result.allowed);
        assert_eq!(result.reason, Some("malicious content".to_string()));
    }

    #[test]
    fn test_tool_info_creation() {
        let info = ToolInfo {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({"key": "value"}),
        };
        assert_eq!(info.name, "test_tool");
        assert_eq!(info.description, "A test tool");
    }

    #[test]
    fn test_recall_item_creation() {
        let entry = MemoryEntry {
            id: "recall-id".to_string(),
            content: "recalled content".to_string(),
            metadata: HashMap::new(),
        };
        let item = RecallItem { entry, score: 0.95 };
        assert_eq!(item.entry.id, "recall-id");
        assert_eq!(item.score, 0.95);
    }
}
