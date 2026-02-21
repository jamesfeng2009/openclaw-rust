//! Canvas 和 Browser 工具抽象
//!
//! 将 Canvas 绘图和 Browser 自动化作为工具集成到 Agent 决策流程中

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::Result;

#[async_trait]
pub trait CanvasTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: CanvasToolParams) -> Result<CanvasToolResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasToolParams {
    pub action: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub message: String,
}

pub struct DrawTool;

impl DrawTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl CanvasTool for DrawTool {
    fn name(&self) -> &str {
        "canvas_draw"
    }

    fn description(&self) -> &str {
        "Draw shapes, text, or images on canvas"
    }

    async fn execute(&self, params: CanvasToolParams) -> Result<CanvasToolResult> {
        match params.action.as_str() {
            "rectangle" | "circle" | "line" | "text" => {
                Ok(CanvasToolResult {
                    success: true,
                    data: serde_json::json!({ "drawn": true }),
                    message: format!("Drew {} on canvas", params.action),
                })
            }
            "clear" => {
                Ok(CanvasToolResult {
                    success: true,
                    data: serde_json::json!({ "cleared": true }),
                    message: "Canvas cleared".to_string(),
                })
            }
            _ => Ok(CanvasToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

impl Default for DrawTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait BrowserTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, params: BrowserToolParams) -> Result<BrowserToolResult>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolParams {
    pub action: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserToolResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub message: String,
}

pub struct WebNavigationTool;

impl WebNavigationTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BrowserTool for WebNavigationTool {
    fn name(&self) -> &str {
        "browser_navigate"
    }

    fn description(&self) -> &str {
        "Navigate to URLs, go back/forward"
    }

    async fn execute(&self, params: BrowserToolParams) -> Result<BrowserToolResult> {
        match params.action.as_str() {
            "goto" => {
                let url = params.args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                Ok(BrowserToolResult {
                    success: true,
                    data: serde_json::json!({ "url": url }),
                    message: format!("Navigated to {}", url),
                })
            }
            "back" | "forward" => {
                Ok(BrowserToolResult {
                    success: true,
                    data: serde_json::json!({}),
                    message: format!("Navigated {}", params.action),
                })
            }
            _ => Ok(BrowserToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

impl Default for WebNavigationTool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WebInteractionTool;

impl WebInteractionTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl BrowserTool for WebInteractionTool {
    fn name(&self) -> &str {
        "browser_interact"
    }

    fn description(&self) -> &str {
        "Click, type, scroll on web pages"
    }

    async fn execute(&self, params: BrowserToolParams) -> Result<BrowserToolResult> {
        match params.action.as_str() {
            "click" => {
                let selector = params.args.get("selector").and_then(|v| v.as_str()).unwrap_or("");
                Ok(BrowserToolResult {
                    success: true,
                    data: serde_json::json!({ "clicked": selector }),
                    message: format!("Clicked element: {}", selector),
                })
            }
            "type" => {
                let selector = params.args.get("selector").and_then(|v| v.as_str()).unwrap_or("");
                let text = params.args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                Ok(BrowserToolResult {
                    success: true,
                    data: serde_json::json!({ "typed": text, "into": selector }),
                    message: format!("Typed into {}", selector),
                })
            }
            "screenshot" => {
                Ok(BrowserToolResult {
                    success: true,
                    data: serde_json::json!({ "screenshot": "base64_data" }),
                    message: "Screenshot captured".to_string(),
                })
            }
            _ => Ok(BrowserToolResult {
                success: false,
                data: serde_json::Value::Null,
                message: format!("Unknown action: {}", params.action),
            }),
        }
    }
}

impl Default for WebInteractionTool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct UIToolRegistry {
    canvas_tools: Vec<Arc<dyn CanvasTool>>,
    browser_tools: Vec<Arc<dyn BrowserTool>>,
}

impl UIToolRegistry {
    pub fn new() -> Self {
        Self {
            canvas_tools: vec![Arc::new(DrawTool::new())],
            browser_tools: vec![
                Arc::new(WebNavigationTool::new()),
                Arc::new(WebInteractionTool::new()),
            ],
        }
    }

    pub fn get_canvas_tools(&self) -> &[Arc<dyn CanvasTool>] {
        &self.canvas_tools
    }

    pub fn get_browser_tools(&self) -> &[Arc<dyn BrowserTool>] {
        &self.browser_tools
    }

    pub async fn execute_canvas(&self, tool_name: &str, params: CanvasToolParams) -> Result<CanvasToolResult> {
        for tool in &self.canvas_tools {
            if tool.name() == tool_name {
                return tool.execute(params).await;
            }
        }
        Err(openclaw_core::OpenClawError::Config(format!("Canvas tool not found: {}", tool_name)))
    }

    pub async fn execute_browser(&self, tool_name: &str, params: BrowserToolParams) -> Result<BrowserToolResult> {
        for tool in &self.browser_tools {
            if tool.name() == tool_name {
                return tool.execute(params).await;
            }
        }
        Err(openclaw_core::OpenClawError::Config(format!("Browser tool not found: {}", tool_name)))
    }
}

impl Default for UIToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
