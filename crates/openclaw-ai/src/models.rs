//! AI 模型配置和元数据

use serde::{Deserialize, Serialize};

/// 模型元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMeta {
    /// 模型 ID
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 提供商
    pub provider: String,
    /// 上下文窗口大小
    pub context_window: usize,
    /// 最大输出 token
    pub max_output: usize,
    /// 是否支持视觉
    pub supports_vision: bool,
    /// 是否支持工具调用
    pub supports_tools: bool,
    /// 输入价格 (美元/百万 token)
    pub input_price: f64,
    /// 输出价格 (美元/百万 token)
    pub output_price: f64,
}

impl ModelMeta {
    pub fn new(id: impl Into<String>, provider: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: String::new(),
            provider: provider.into(),
            context_window: 4096,
            max_output: 4096,
            supports_vision: false,
            supports_tools: false,
            input_price: 0.0,
            output_price: 0.0,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_context(mut self, context: usize, max_output: usize) -> Self {
        self.context_window = context;
        self.max_output = max_output;
        self
    }

    pub fn with_vision(mut self, supports: bool) -> Self {
        self.supports_vision = supports;
        self
    }

    pub fn with_tools(mut self, supports: bool) -> Self {
        self.supports_tools = supports;
        self
    }

    pub fn with_pricing(mut self, input: f64, output: f64) -> Self {
        self.input_price = input;
        self.output_price = output;
        self
    }
}

/// 获取所有支持的模型
pub fn get_all_models() -> Vec<ModelMeta> {
    let mut models = Vec::new();
    
    // OpenAI 最新模型
    models.extend(get_openai_models());
    
    // Anthropic 最新模型
    models.extend(get_anthropic_models());
    
    // Google Gemini
    models.extend(get_gemini_models());
    
    // DeepSeek
    models.extend(get_deepseek_models());
    
    // Qwen 通义千问
    models.extend(get_qwen_models());
    
    // GLM 智谱
    models.extend(get_glm_models());
    
    // Minimax
    models.extend(get_minimax_models());
    
    // Kimi 月之暗面
    models.extend(get_kimi_models());
    
    models
}

/// OpenAI 最新模型 (2025)
pub fn get_openai_models() -> Vec<ModelMeta> {
    vec![
        // GPT-4o 系列
        ModelMeta::new("gpt-4o", "openai")
            .with_name("GPT-4o")
            .with_context(128_000, 16_384)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(2.5, 10.0),
        
        ModelMeta::new("gpt-4o-mini", "openai")
            .with_name("GPT-4o Mini")
            .with_context(128_000, 16_384)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.15, 0.6),
        
        ModelMeta::new("gpt-4o-audio-preview", "openai")
            .with_name("GPT-4o Audio Preview")
            .with_context(128_000, 16_384)
            .with_tools(true)
            .with_pricing(2.5, 10.0),

        // GPT-4 系列
        ModelMeta::new("gpt-4-turbo", "openai")
            .with_name("GPT-4 Turbo")
            .with_context(128_000, 4_096)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(10.0, 30.0),

        // o1 推理系列
        ModelMeta::new("o1", "openai")
            .with_name("o1")
            .with_context(200_000, 100_000)
            .with_tools(true)
            .with_pricing(15.0, 60.0),
        
        ModelMeta::new("o1-mini", "openai")
            .with_name("o1 Mini")
            .with_context(128_000, 65_536)
            .with_pricing(1.5, 6.0),
        
        ModelMeta::new("o3-mini", "openai")
            .with_name("o3 Mini")
            .with_context(200_000, 100_000)
            .with_tools(true)
            .with_pricing(1.1, 4.4),
    ]
}

/// Anthropic 最新模型 (2025)
pub fn get_anthropic_models() -> Vec<ModelMeta> {
    vec![
        // Claude 4 系列
        ModelMeta::new("claude-4-opus", "anthropic")
            .with_name("Claude 4 Opus")
            .with_context(200_000, 16_384)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(15.0, 75.0),
        
        ModelMeta::new("claude-4-sonnet", "anthropic")
            .with_name("Claude 4 Sonnet")
            .with_context(200_000, 16_384)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(3.0, 15.0),

        // Claude 3.7 系列
        ModelMeta::new("claude-3-7-sonnet", "anthropic")
            .with_name("Claude 3.7 Sonnet")
            .with_context(200_000, 16_384)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(3.0, 15.0),

        // Claude 3.5 系列
        ModelMeta::new("claude-3-5-sonnet-20241022", "anthropic")
            .with_name("Claude 3.5 Sonnet v2")
            .with_context(200_000, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(3.0, 15.0),
        
        ModelMeta::new("claude-3-5-haiku-20241022", "anthropic")
            .with_name("Claude 3.5 Haiku")
            .with_context(200_000, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.8, 4.0),
    ]
}

/// Google Gemini 最新模型
pub fn get_gemini_models() -> Vec<ModelMeta> {
    vec![
        // Gemini 2.0 系列
        ModelMeta::new("gemini-2.0-flash", "google")
            .with_name("Gemini 2.0 Flash")
            .with_context(1_048_576, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.1, 0.4),
        
        ModelMeta::new("gemini-2.0-pro", "google")
            .with_name("Gemini 2.0 Pro")
            .with_context(1_048_576, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(1.25, 5.0),

        // Gemini 1.5 系列
        ModelMeta::new("gemini-1.5-pro", "google")
            .with_name("Gemini 1.5 Pro")
            .with_context(2_097_152, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(1.25, 5.0),
        
        ModelMeta::new("gemini-1.5-flash", "google")
            .with_name("Gemini 1.5 Flash")
            .with_context(1_048_576, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.075, 0.3),
    ]
}

/// DeepSeek 最新模型
pub fn get_deepseek_models() -> Vec<ModelMeta> {
    vec![
        ModelMeta::new("deepseek-chat", "deepseek")
            .with_name("DeepSeek Chat")
            .with_context(64_000, 4_096)
            .with_tools(true)
            .with_pricing(0.14, 0.28),
        
        ModelMeta::new("deepseek-reasoner", "deepseek")
            .with_name("DeepSeek Reasoner")
            .with_context(64_000, 4_096)
            .with_tools(true)
            .with_pricing(0.55, 2.19),
    ]
}

/// Qwen 通义千问最新模型
pub fn get_qwen_models() -> Vec<ModelMeta> {
    vec![
        // Qwen 2.5 系列
        ModelMeta::new("qwen-max", "qwen")
            .with_name("Qwen Max")
            .with_context(32_768, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(2.0, 8.0),
        
        ModelMeta::new("qwen-plus", "qwen")
            .with_name("Qwen Plus")
            .with_context(131_072, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.8, 2.0),
        
        ModelMeta::new("qwen-turbo", "qwen")
            .with_name("Qwen Turbo")
            .with_context(131_072, 8_192)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(0.3, 0.6),
        
        ModelMeta::new("qwen-vl-max", "qwen")
            .with_name("Qwen VL Max")
            .with_context(32_768, 8_192)
            .with_vision(true)
            .with_pricing(3.0, 9.0),
    ]
}

/// GLM 智谱最新模型
pub fn get_glm_models() -> Vec<ModelMeta> {
    vec![
        // GLM-4 系列
        ModelMeta::new("glm-4-plus", "glm")
            .with_name("GLM-4 Plus")
            .with_context(128_000, 4_096)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(50.0, 50.0), // 人民币/百万token
        
        ModelMeta::new("glm-4-air", "glm")
            .with_name("GLM-4 Air")
            .with_context(128_000, 4_096)
            .with_tools(true)
            .with_pricing(1.0, 1.0),
        
        ModelMeta::new("glm-4-flash", "glm")
            .with_name("GLM-4 Flash")
            .with_context(128_000, 4_096)
            .with_tools(true)
            .with_pricing(0.1, 0.1),

        // GLM-Z1 推理模型
        ModelMeta::new("glm-z1-air", "glm")
            .with_name("GLM-Z1 Air")
            .with_context(128_000, 4_096)
            .with_pricing(0.35, 0.35),
    ]
}

/// Minimax 最新模型
pub fn get_minimax_models() -> Vec<ModelMeta> {
    vec![
        ModelMeta::new("abab6.5s-chat", "minimax")
            .with_name("ABAB 6.5s Chat")
            .with_context(245_000, 4_096)
            .with_vision(true)
            .with_tools(true)
            .with_pricing(2.0, 2.0),
        
        ModelMeta::new("abab6.5g-chat", "minimax")
            .with_name("ABAB 6.5g Chat")
            .with_context(8_192, 4_096)
            .with_pricing(0.3, 0.3),
        
        ModelMeta::new("abab5.5-chat", "minimax")
            .with_name("ABAB 5.5 Chat")
            .with_context(16_384, 4_096)
            .with_pricing(0.3, 0.3),
    ]
}

/// Kimi 月之暗面最新模型
pub fn get_kimi_models() -> Vec<ModelMeta> {
    vec![
        ModelMeta::new("moonshot-v1-128k", "kimi")
            .with_name("Kimi v1 128K")
            .with_context(128_000, 4_096)
            .with_tools(true)
            .with_pricing(14.0, 14.0),
        
        ModelMeta::new("moonshot-v1-32k", "kimi")
            .with_name("Kimi v1 32K")
            .with_context(32_000, 4_096)
            .with_tools(true)
            .with_pricing(10.0, 10.0),
        
        ModelMeta::new("moonshot-v1-8k", "kimi")
            .with_name("Kimi v1 8K")
            .with_context(8_000, 4_096)
            .with_tools(true)
            .with_pricing(12.0, 12.0),
    ]
}

/// 根据模型 ID 获取模型元数据
pub fn get_model_by_id(id: &str) -> Option<ModelMeta> {
    get_all_models().into_iter().find(|m| m.id == id)
}

/// 获取指定提供商的模型列表
pub fn get_models_by_provider(provider: &str) -> Vec<ModelMeta> {
    get_all_models().into_iter().filter(|m| m.provider == provider).collect()
}
