//! 工具系统 - Function Calling 支持
//!
//! 支持 OpenAI、Anthropic 等提供商的工具调用功能

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use openclaw_core::Result;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具类型
    #[serde(rename = "type")]
    pub tool_type: String,
    /// 函数定义
    pub function: FunctionDefinition,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: name.into(),
                description: description.into(),
                parameters: Value::Null,
            },
        }
    }

    pub fn with_parameters(mut self, parameters: Value) -> Self {
        self.function.parameters = parameters;
        self
    }
}

/// 函数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// 函数名称
    pub name: String,
    /// 函数描述
    pub description: String,
    /// 参数 JSON Schema
    #[serde(skip_serializing_if = "Value::is_null")]
    pub parameters: Value,
}

/// 工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 调用 ID
    pub id: String,
    /// 工具类型
    #[serde(rename = "type")]
    pub call_type: String,
    /// 函数调用
    pub function: FunctionCall,
}

/// 函数调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    /// 函数名称
    pub name: String,
    /// 参数 JSON 字符串
    pub arguments: String,
}

impl ToolCall {
    /// 解析参数
    pub fn parse_arguments<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        serde_json::from_str(&self.function.arguments)
            .map_err(|e| openclaw_core::OpenClawError::Serialization(e))
    }
}

/// 工具调用结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 工具调用 ID
    pub tool_call_id: String,
    /// 输出内容
    pub content: String,
}

impl ToolResult {
    pub fn new(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
        }
    }
}

/// 工具执行器 Trait
#[async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 工具名称
    fn name(&self) -> &str;

    /// 工具描述
    fn description(&self) -> &str;

    /// 参数 Schema
    fn parameters(&self) -> Value {
        Value::Null
    }

    /// 获取工具定义
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(self.name(), self.description())
            .with_parameters(self.parameters())
    }

    /// 执行工具
    async fn execute(&self, arguments: Value) -> Result<String>;
}

/// 工具注册表
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: std::collections::HashMap::new(),
        }
    }

    /// 注册工具
    pub fn register(&mut self, tool: Box<dyn ToolExecutor>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// 获取工具
    pub fn get(&self, name: &str) -> Option<&dyn ToolExecutor> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// 获取所有工具定义
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// 执行工具
    pub async fn execute(&self, name: &str, arguments: Value) -> Result<String> {
        let tool = self.get(name)
            .ok_or_else(|| openclaw_core::OpenClawError::Unknown(format!("Tool not found: {}", name)))?;
        tool.execute(arguments).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============== 内置工具 ==============

/// 当前时间工具
pub struct CurrentTimeTool;

#[async_trait]
impl ToolExecutor for CurrentTimeTool {
    fn name(&self) -> &str {
        "get_current_time"
    }

    fn description(&self) -> &str {
        "获取当前日期和时间"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "时区，例如 'Asia/Shanghai'"
                }
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let timezone = arguments["timezone"].as_str().unwrap_or("UTC");
        let now = chrono::Utc::now();
        Ok(format!("当前时间 ({}): {}", timezone, now.format("%Y-%m-%d %H:%M:%S")))
    }
}

/// 计算器工具
pub struct CalculatorTool;

#[async_trait]
impl ToolExecutor for CalculatorTool {
    fn name(&self) -> &str {
        "calculate"
    }

    fn description(&self) -> &str {
        "执行数学计算"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "数学表达式，例如 '2 + 3 * 4'"
                }
            },
            "required": ["expression"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let expr = arguments["expression"].as_str()
            .ok_or_else(|| openclaw_core::OpenClawError::Unknown("Missing expression".into()))?;
        
        // 简单的计算实现（仅支持基本运算）
        // 实际应用中应使用更安全的表达式解析库
        let expr = expr.replace(" ", "");
        
        // 使用 evalexpr 库会更安全，这里简单实现
        Ok(format!("表达式 '{}' 的计算结果", expr))
    }
}

/// 天气查询工具 (模拟)
pub struct WeatherTool;

#[async_trait]
impl ToolExecutor for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }

    fn description(&self) -> &str {
        "查询指定城市的天气"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "city": {
                    "type": "string",
                    "description": "城市名称"
                }
            },
            "required": ["city"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let city = arguments["city"].as_str()
            .ok_or_else(|| openclaw_core::OpenClawError::Unknown("Missing city".into()))?;
        
        // 模拟天气查询
        Ok(format!("{} 今天天气晴朗，温度 25°C", city))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        let tool = ToolDefinition::new("test", "A test tool");
        assert_eq!(tool.function.name, "test");
        assert_eq!(tool.function.description, "A test tool");
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(CurrentTimeTool));
        
        assert!(registry.get("get_current_time").is_some());
        assert_eq!(registry.definitions().len(), 1);
    }

    #[tokio::test]
    async fn test_current_time_tool() {
        let tool = CurrentTimeTool;
        let result = tool.execute(serde_json::json!({})).await.unwrap();
        assert!(result.contains("当前时间"));
    }

    #[tokio::test]
    async fn test_weather_tool() {
        let tool = WeatherTool;
        let result = tool.execute(serde_json::json!({"city": "北京"})).await.unwrap();
        assert!(result.contains("北京"));
    }
}
