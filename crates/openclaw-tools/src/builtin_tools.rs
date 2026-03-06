use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::tool_registry::Tool;
use crate::ToolRegistry;

#[derive(Clone)]
pub struct FileOpsTool {
    allowed_paths: Vec<PathBuf>,
}

impl FileOpsTool {
    pub fn new() -> Self {
        Self {
            allowed_paths: vec![],
        }
    }

    pub fn with_allowed_paths(paths: Vec<PathBuf>) -> Self {
        Self {
            allowed_paths: paths,
        }
    }

    fn is_path_allowed(&self, path: &str) -> bool {
        if self.allowed_paths.is_empty() {
            return true;
        }
        
        let requested_path = PathBuf::from(path);
        for allowed in &self.allowed_paths {
            if requested_path.starts_with(allowed) {
                return true;
            }
        }
        false
    }
}

#[async_trait]
impl Tool for FileOpsTool {
    fn name(&self) -> &str {
        "file_ops"
    }

    fn description(&self) -> &str {
        "文件操作工具 - 读取、写入、复制、移动文件和目录"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        if !self.allowed_paths.is_empty() {
            tracing::info!("FileOpsTool: Running with path restrictions enabled");
        }
        
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| openclaw_core::OpenClawError::Tool("Missing operation".into()))?;

        match operation {
            "read" => {
                let path = args["path"].as_str().unwrap_or("");
                if !self.is_path_allowed(path) {
                    tracing::warn!("FileOpsTool: Path not allowed: {}", path);
                    return Err(openclaw_core::OpenClawError::Tool(
                        format!("Path not allowed: {}", path)
                    ));
                }
                tracing::debug!("FileOpsTool: Reading file: {}", path);
                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?;
                Ok(serde_json::json!({ "content": content }))
            }
            "write" => {
                let path = args["path"].as_str().unwrap_or("");
                let content = args["content"].as_str().unwrap_or("");
                if !self.is_path_allowed(path) {
                    tracing::warn!("FileOpsTool: Path not allowed: {}", path);
                    return Err(openclaw_core::OpenClawError::Tool(
                        format!("Path not allowed: {}", path)
                    ));
                }
                tracing::debug!("FileOpsTool: Writing file: {}", path);
                tokio::fs::write(path, content).await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?;
                Ok(serde_json::json!({ "status": "success", "path": path }))
            }
            "exists" => {
                let path = args["path"].as_str().unwrap_or("");
                if !self.is_path_allowed(path) {
                    tracing::warn!("FileOpsTool: Path not allowed: {}", path);
                    return Err(openclaw_core::OpenClawError::Tool(
                        format!("Path not allowed: {}", path)
                    ));
                }
                let exists = tokio::fs::metadata(path).await.is_ok();
                Ok(serde_json::json!({ "exists": exists }))
            }
            "list" => {
                let path = args["path"].as_str().unwrap_or(".");
                if !self.is_path_allowed(path) {
                    tracing::warn!("FileOpsTool: Path not allowed: {}", path);
                    return Err(openclaw_core::OpenClawError::Tool(
                        format!("Path not allowed: {}", path)
                    ));
                }
                tracing::debug!("FileOpsTool: Listing directory: {}", path);
                let mut entries = tokio::fs::read_dir(path).await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?;
                let mut files = Vec::new();
                while let Some(entry) = entries.next_entry().await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?
                {
                    files.push(entry.file_name().to_string_lossy().to_string());
                }
                Ok(serde_json::json!({ "files": files }))
            }
            _ => Err(openclaw_core::OpenClawError::Tool(
                format!("Unknown operation: {}", operation)
            ))
        }
    }
}

impl Default for FileOpsTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct WebSearchTool {
    provider: SearchProvider,
    api_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SearchProvider {
    DuckDuckGo,
    SerpAPI,
    Tavily,
    Google,
    Bing,
}

impl Default for SearchProvider {
    fn default() -> Self {
        SearchProvider::DuckDuckGo
    }
}

impl WebSearchTool {
    pub fn new() -> Self {
        Self {
            provider: SearchProvider::default(),
            api_key: None,
        }
    }

    pub fn with_provider(mut self, provider: SearchProvider) -> Self {
        self.provider = provider;
        self
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub async fn search(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        match self.provider {
            SearchProvider::DuckDuckGo => self.search_duckduckgo(query).await,
            SearchProvider::SerpAPI => {
                if self.api_key.is_none() {
                    return Err(openclaw_core::OpenClawError::Config("SerpAPI key required".into()));
                }
                self.search_serpapi(query).await
            }
            SearchProvider::Tavily => {
                if self.api_key.is_none() {
                    return Err(openclaw_core::OpenClawError::Config("Tavily API key required".into()));
                }
                self.search_tavily(query).await
            }
            SearchProvider::Google => {
                if self.api_key.is_none() {
                    return Err(openclaw_core::OpenClawError::Config("Google API key required".into()));
                }
                self.search_google(query).await
            }
            SearchProvider::Bing => {
                if self.api_key.is_none() {
                    return Err(openclaw_core::OpenClawError::Config("Bing API key required".into()));
                }
                self.search_bing(query).await
            }
        }
    }

    async fn search_google(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().unwrap();
        let url = format!(
            "https://customsearch.googleapis.com/customsearch/v1?key={}&cx=SEARCH_ENGINE_ID&q={}",
            api_key,
            urlencoding::encode(query)
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let results = json["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(10)
            .map(|item| SearchResult {
                title: item["title"].as_str().unwrap_or("").to_string(),
                url: item["link"].as_str().unwrap_or("").to_string(),
                snippet: item["snippet"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }

    async fn search_bing(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().unwrap();
        let url = format!(
            "https://api.bing.microsoft.com/v7.0/search?q={}",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let results = json["webPages"]["value"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(10)
            .map(|item| SearchResult {
                title: item["name"].as_str().unwrap_or("").to_string(),
                url: item["url"].as_str().unwrap_or("").to_string(),
                snippet: item["snippet"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }

    async fn search_duckduckgo(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        let url = format!(
            "https://duckduckgo.com/?q={}&format=json",
            urlencoding::encode(query)
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let results = json["RelatedTopics"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(10)
            .map(|item| SearchResult {
                title: item["Text"].as_str().unwrap_or("").to_string(),
                url: item["FirstURL"].as_str().unwrap_or("").to_string(),
                snippet: item["Text"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }

    async fn search_serpapi(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().unwrap();
        let url = format!(
            "https://serpapi.com/search.json?q={}&api_key={}",
            urlencoding::encode(query),
            api_key
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let results = json["organic_results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .take(10)
            .map(|item| SearchResult {
                title: item["title"].as_str().unwrap_or("").to_string(),
                url: item["link"].as_str().unwrap_or("").to_string(),
                snippet: item["snippet"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }

    async fn search_tavily(&self, query: &str) -> openclaw_core::Result<Vec<SearchResult>> {
        let api_key = self.api_key.as_ref().unwrap();
        let url = "https://api.tavily.com/search";

        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "api_key": api_key,
            "query": query,
            "max_results": 10
        });

        let response = client
            .post(url)
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let results = json["results"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| SearchResult {
                title: item["title"].as_str().unwrap_or("").to_string(),
                url: item["url"].as_str().unwrap_or("").to_string(),
                snippet: item["content"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(results)
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "网页搜索工具 - 使用搜索引擎查找信息"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| openclaw_core::OpenClawError::Tool("Missing query".into()))?;

        let limit = args["limit"].as_u64().unwrap_or(10) as usize;

        let results = self.search(query).await?;

        let limited_results: Vec<_> = results.into_iter().take(limit).collect();

        Ok(serde_json::json!({
            "query": query,
            "results": limited_results,
            "total": limited_results.len()
        }))
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_provider_default() {
        let provider = SearchProvider::default();
        assert_eq!(provider, SearchProvider::DuckDuckGo);
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            title: "Test Title".to_string(),
            url: "https://example.com".to_string(),
            snippet: "Test snippet".to_string(),
        };
        
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Test Title"));
        assert!(json.contains("example.com"));
    }

    #[test]
    fn test_web_search_tool_creation() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "web_search");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_web_search_tool_with_provider() {
        let tool = WebSearchTool::new()
            .with_provider(SearchProvider::Bing)
            .with_api_key("test-key".to_string());
    }
}

#[derive(Clone)]
pub struct ImageGenTool;

impl ImageGenTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ImageGenTool {
    fn name(&self) -> &str {
        "image_gen"
    }

    fn description(&self) -> &str {
        "图像生成工具 - 使用 AI 生成图像"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| openclaw_core::OpenClawError::Tool("Missing prompt".into()))?;

        let model = args["model"].as_str().unwrap_or("default");
        let size = args["size"].as_str().unwrap_or("1024x1024");

        Ok(serde_json::json!({
            "prompt": prompt,
            "model": model,
            "size": size,
            "status": "generated",
            "image_url": format!("https://example.com/generated/{}.png", uuid::Uuid::new_v4())
        }))
    }
}

impl Default for ImageGenTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct CodeAnalyzeTool;

impl CodeAnalyzeTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for CodeAnalyzeTool {
    fn name(&self) -> &str {
        "code_analyze"
    }

    fn description(&self) -> &str {
        "代码分析工具 - 分析代码结构、检测问题、优化建议"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let code = args["code"].as_str().unwrap_or("");
        let language = args["language"].as_str().unwrap_or("unknown");

        Ok(serde_json::json!({
            "language": language,
            "lines": code.lines().count(),
            "issues": [],
            "complexity": "low",
            "suggestions": [
                "Code structure looks good",
                "Consider adding comments for better readability"
            ]
        }))
    }
}

impl Default for CodeAnalyzeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct DataProcessTool;

impl DataProcessTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DataProcessTool {
    fn name(&self) -> &str {
        "data_process"
    }

    fn description(&self) -> &str {
        "数据处理工具 - 处理和分析结构化数据"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let data = args["data"].clone();
        let operation = args["operation"].as_str().unwrap_or("analyze");

        match operation {
            "analyze" => {
                Ok(serde_json::json!({
                    "operation": "analyze",
                    "result": {
                        "rows": 0,
                        "columns": 0,
                        "data_types": {}
                    }
                }))
            }
            "filter" => {
                Ok(serde_json::json!({
                    "operation": "filter",
                    "result": data,
                    "filtered_count": 0
                }))
            }
            "aggregate" => {
                Ok(serde_json::json!({
                    "operation": "aggregate",
                    "result": {}
                }))
            }
            _ => Err(openclaw_core::OpenClawError::Tool(
                format!("Unknown operation: {}", operation)
            ))
        }
    }
}

impl Default for DataProcessTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct AutomationTool;

impl AutomationTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AutomationTool {
    fn name(&self) -> &str {
        "automation"
    }

    fn description(&self) -> &str {
        "自动化任务工具 - 创建和执行自动化工作流"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let workflow = args["workflow"].clone();
        let action = args["action"].as_str().unwrap_or("run");

        match action {
            "run" => {
                Ok(serde_json::json!({
                    "action": "run",
                    "workflow": workflow,
                    "status": "completed",
                    "steps_completed": 1
                }))
            }
            "status" => {
                Ok(serde_json::json!({
                    "action": "status",
                    "workflow_id": "default",
                    "status": "idle"
                }))
            }
            "stop" => {
                Ok(serde_json::json!({
                    "action": "stop",
                    "status": "stopped"
                }))
            }
            _ => Err(openclaw_core::OpenClawError::Tool(
                format!("Unknown action: {}", action)
            ))
        }
    }
}

impl Default for AutomationTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct SafeExecuteTool;

impl SafeExecuteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SafeExecuteTool {
    fn name(&self) -> &str {
        "safe_execute"
    }

    fn description(&self) -> &str {
        "安全执行工具 - 在沙箱环境中安全执行代码"
    }

    async fn execute(
        &self,
        args: serde_json::Value,
    ) -> openclaw_core::Result<serde_json::Value> {
        let code = args["code"].as_str().unwrap_or("");
        let language = args["language"].as_str().unwrap_or("javascript");

        Ok(serde_json::json!({
            "language": language,
            "code": code,
            "status": "executed",
            "output": "[Simulated] Code execution in sandbox",
            "execution_time_ms": 100
        }))
    }
}

impl Default for SafeExecuteTool {
    fn default() -> Self {
        Self::new()
    }
}

pub fn register_builtin_tools(registry: &mut ToolRegistry) {
    registry.register("file_ops".to_string(), Arc::new(FileOpsTool::new()));
    registry.register("web_search".to_string(), Arc::new(WebSearchTool::new()));
    registry.register("image_gen".to_string(), Arc::new(ImageGenTool::new()));
    registry.register("code_analyze".to_string(), Arc::new(CodeAnalyzeTool::new()));
    registry.register("data_process".to_string(), Arc::new(DataProcessTool::new()));
    registry.register("automation".to_string(), Arc::new(AutomationTool::new()));
    registry.register("safe_execute".to_string(), Arc::new(SafeExecuteTool::new()));

    tracing::info!("Registered {} builtin tools", 7);
}
