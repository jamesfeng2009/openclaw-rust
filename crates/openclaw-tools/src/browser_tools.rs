//! 浏览器工具模块

use crate::types::*;
use async_trait::async_trait;
use openclaw_browser::{BrowserPool, BrowserConfig, Selector, ScreenshotOptions, PdfOptions};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// 浏览器工具错误
#[derive(Debug, Error)]
pub enum BrowserToolError {
    #[error("浏览器实例不存在: {0}")]
    BrowserNotFound(String),

    #[error("页面不存在: {0}")]
    PageNotFound(String),

    #[error("操作失败: {0}")]
    OperationFailed(String),

    #[error("内部错误: {0}")]
    Internal(#[from] anyhow::Error),
}

/// 浏览器工具集
pub struct BrowserTools {
    pool: Arc<BrowserPool>,
}

impl BrowserTools {
    pub fn new(pool: Arc<BrowserPool>) -> Self {
        Self { pool }
    }

    /// 获取工具定义列表
    pub fn get_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                id: "browser_navigate".to_string(),
                name: "浏览器导航".to_string(),
                description: "导航到指定 URL".to_string(),
                parameters: ToolParameters {
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("browser_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "浏览器实例 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("page_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "页面 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("url".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "目标 URL".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props
                    },
                    required: vec!["browser_id".to_string(), "page_id".to_string(), "url".to_string()],
                },
                category: ToolCategory::Browser,
                enabled: true,
            },
            ToolDefinition {
                id: "browser_click".to_string(),
                name: "浏览器点击".to_string(),
                description: "点击页面元素".to_string(),
                parameters: ToolParameters {
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("browser_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "浏览器实例 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("page_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "页面 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("selector".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "CSS 选择器".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props
                    },
                    required: vec!["browser_id".to_string(), "page_id".to_string(), "selector".to_string()],
                },
                category: ToolCategory::Browser,
                enabled: true,
            },
            ToolDefinition {
                id: "browser_type".to_string(),
                name: "浏览器输入".to_string(),
                description: "在输入框中输入文本".to_string(),
                parameters: ToolParameters {
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("browser_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "浏览器实例 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("page_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "页面 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("selector".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "CSS 选择器".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("text".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "要输入的文本".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props
                    },
                    required: vec!["browser_id".to_string(), "page_id".to_string(), "selector".to_string(), "text".to_string()],
                },
                category: ToolCategory::Browser,
                enabled: true,
            },
            ToolDefinition {
                id: "browser_screenshot".to_string(),
                name: "浏览器截图".to_string(),
                description: "截取页面截图".to_string(),
                parameters: ToolParameters {
                    properties: {
                        let mut props = HashMap::new();
                        props.insert("browser_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "浏览器实例 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("page_id".to_string(), ParameterProperty {
                            param_type: "string".to_string(),
                            description: "页面 ID".to_string(),
                            enum_values: vec![],
                            default: None,
                        });
                        props.insert("full_page".to_string(), ParameterProperty {
                            param_type: "boolean".to_string(),
                            description: "是否截取整个页面".to_string(),
                            enum_values: vec![],
                            default: Some(Value::Bool(false)),
                        });
                        props
                    },
                    required: vec!["browser_id".to_string(), "page_id".to_string()],
                },
                category: ToolCategory::Browser,
                enabled: true,
            },
        ]
    }

    /// 执行浏览器工具
    pub async fn execute(
        &self,
        tool_id: &str,
        params: &HashMap<String, Value>,
        _context: &ToolContext,
    ) -> Result<ToolResult, BrowserToolError> {
        match tool_id {
            "browser_navigate" => {
                let browser_id = params.get("browser_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 browser_id".to_string()))?;
                let page_id = params.get("page_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 page_id".to_string()))?;
                let url = params.get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 url".to_string()))?;

                let browser = self.pool.get_browser(&browser_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::BrowserNotFound(browser_id.to_string()))?;
                let page = browser.get_page(&page_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::PageNotFound(page_id.to_string()))?;

                page.goto(url, None).await
                    .map_err(|e| BrowserToolError::OperationFailed(e.to_string()))?;

                Ok(ToolResult::success(Value::String(format!("已导航到 {}", url))))
            }

            "browser_click" => {
                let browser_id = params.get("browser_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 browser_id".to_string()))?;
                let page_id = params.get("page_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 page_id".to_string()))?;
                let selector = params.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 selector".to_string()))?;

                let browser = self.pool.get_browser(&browser_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::BrowserNotFound(browser_id.to_string()))?;
                let page = browser.get_page(&page_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::PageNotFound(page_id.to_string()))?;

                page.click(&Selector::css(selector), None).await
                    .map_err(|e| BrowserToolError::OperationFailed(e.to_string()))?;

                Ok(ToolResult::success(Value::String("点击成功".to_string())))
            }

            "browser_type" => {
                let browser_id = params.get("browser_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 browser_id".to_string()))?;
                let page_id = params.get("page_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 page_id".to_string()))?;
                let selector = params.get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 selector".to_string()))?;
                let text = params.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 text".to_string()))?;

                let browser = self.pool.get_browser(&browser_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::BrowserNotFound(browser_id.to_string()))?;
                let page = browser.get_page(&page_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::PageNotFound(page_id.to_string()))?;

                page.type_text(&Selector::css(selector), text, None).await
                    .map_err(|e| BrowserToolError::OperationFailed(e.to_string()))?;

                Ok(ToolResult::success(Value::String(format!("已输入: {}", text))))
            }

            "browser_screenshot" => {
                let browser_id = params.get("browser_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 browser_id".to_string()))?;
                let page_id = params.get("page_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| BrowserToolError::OperationFailed("缺少 page_id".to_string()))?;
                let full_page = params.get("full_page")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let browser = self.pool.get_browser(&browser_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::BrowserNotFound(browser_id.to_string()))?;
                let page = browser.get_page(&page_id.to_string())
                    .await
                    .ok_or_else(|| BrowserToolError::PageNotFound(page_id.to_string()))?;

                let options = ScreenshotOptions {
                    full_page,
                    ..Default::default()
                };

                let base64 = page.screenshot_base64(Some(options)).await
                    .map_err(|e| BrowserToolError::OperationFailed(e.to_string()))?;

                Ok(ToolResult::success(Value::String(base64)))
            }

            _ => Err(BrowserToolError::OperationFailed(format!("未知工具: {}", tool_id))),
        }
    }
}
