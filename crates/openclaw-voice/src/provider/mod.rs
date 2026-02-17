//! 语音提供商模块
//! 支持内置提供商和用户自定义提供商（零提供商架构）

mod registry;

pub use registry::{CustomProviderConfig, ProviderRegistry};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::types::{
    AudioFormat, OpenAIVoice, SttConfig, SynthesisOptions, TranscriptionResult, TtsConfig,
};

/// 自定义 TTS 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTtsConfig {
    /// 提供商名称
    pub name: String,
    /// API 端点 URL
    pub endpoint: String,
    /// HTTP 方法
    #[serde(default = "default_http_method")]
    pub method: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体模板 (支持 {{text}}, {{voice}}, {{speed}}, {{format}})
    pub request_template: String,
    /// 响应处理方式
    pub response_type: CustomResponseType,
}

fn default_http_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomResponseType {
    /// 直接返回音频二进制
    Binary,
    /// 返回 JSON，包含 audio 字段
    Json { audio_field: String },
}

/// 自定义 STT 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomSttConfig {
    /// 提供商名称
    pub name: String,
    /// API 端点 URL
    pub endpoint: String,
    /// HTTP 方法
    #[serde(default = "default_http_method")]
    pub method: String,
    /// 请求头
    pub headers: HashMap<String, String>,
    /// 请求体字段名 (用于 multipart)
    pub audio_field: String,
    /// 响应处理方式
    pub response_type: CustomSttResponseType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CustomSttResponseType {
    /// 返回纯文本
    Text,
    /// 返回 JSON，包含 text 字段
    Json { text_field: String },
}

/// 自定义 TTS 提供商实现
pub struct CustomTtsProvider {
    config: CustomTtsConfig,
    client: Client,
}

impl CustomTtsProvider {
    pub fn new(config: CustomTtsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn build_request_body(&self, text: &str, options: &SynthesisOptions) -> String {
        let mut template = self.config.request_template.clone();

        template = template.replace("{{text}}", text);

        if let Some(voice) = &options.voice {
            template = template.replace("{{voice}}", voice);
        } else {
            template = template.replace("{{voice}}", "alloy");
        }

        if let Some(speed) = options.speed {
            template = template.replace("{{speed}}", &speed.to_string());
        } else {
            template = template.replace("{{speed}}", "1.0");
        }

        if let Some(format) = &options.format {
            template = template.replace("{{format}}", format.as_extension());
        } else {
            template = template.replace("{{format}}", "mp3");
        }

        template
    }
}

#[async_trait]
impl crate::tts::TextToSpeech for CustomTtsProvider {
    fn provider(&self) -> crate::types::TtsProvider {
        crate::types::TtsProvider::Custom(self.config.name.clone())
    }

    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> openclaw_core::Result<Vec<u8>> {
        let opts = options.unwrap_or_default();
        let body = self.build_request_body(text, &opts);

        let mut request = match self.config.method.to_uppercase().as_str() {
            "GET" => self.client.get(&self.config.endpoint),
            "POST" | "PUT" => self.client.post(&self.config.endpoint),
            _ => {
                return Err(openclaw_core::OpenClawError::Config(format!(
                    "不支持的 HTTP 方法: {}",
                    self.config.method
                )));
            }
        };

        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        if self.config.method.to_uppercase() != "GET" {
            request = request.header("Content-Type", "application/json");
            request = request.body(body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Http(format!("请求失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(openclaw_core::OpenClawError::AIProvider(format!(
                "API 错误: {}",
                response.status()
            )));
        }

        match self.config.response_type {
            CustomResponseType::Binary => {
                let bytes = response.bytes().await.map_err(|e| {
                    openclaw_core::OpenClawError::Http(format!("读取响应失败: {}", e))
                })?;
                Ok(bytes.to_vec())
            }
            CustomResponseType::Json { ref audio_field } => {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    openclaw_core::OpenClawError::Http(format!("解析 JSON 失败: {}", e))
                })?;

                let audio_base64 =
                    json.get(audio_field)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            openclaw_core::OpenClawError::Config("响应中未找到音频字段".to_string())
                        })?;

                let audio_data = base64::decode(audio_base64).map_err(|e| {
                    openclaw_core::OpenClawError::Config(format!("Base64 解码失败: {}", e))
                })?;

                Ok(audio_data)
            }
        }
    }

    async fn is_available(&self) -> bool {
        !self.config.endpoint.is_empty()
    }

    fn available_voices(&self) -> Vec<String> {
        vec!["default".to_string()]
    }
}

/// 自定义 STT 提供商实现
pub struct CustomSttProvider {
    config: CustomSttConfig,
    client: Client,
}

impl CustomSttProvider {
    pub fn new(config: CustomSttConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl crate::stt::SpeechToText for CustomSttProvider {
    fn provider(&self) -> crate::types::SttProvider {
        crate::types::SttProvider::Custom(self.config.name.clone())
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        _language: Option<&str>,
    ) -> openclaw_core::Result<TranscriptionResult> {
        let field_name = self.config.audio_field.clone();

        let part = reqwest::multipart::Part::bytes(audio_data.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| {
                openclaw_core::OpenClawError::Http(format!("创建 multipart 失败: {}", e))
            })?;

        let form = reqwest::multipart::Form::new().part(field_name, part);

        let endpoint = self.config.endpoint.clone();
        let method = self.config.method.clone();
        let headers = self.config.headers.clone();

        let mut request = match method.to_uppercase().as_str() {
            "POST" => self.client.post(&endpoint),
            _ => {
                return Err(openclaw_core::OpenClawError::Config(format!(
                    "不支持的 HTTP 方法: {}",
                    method
                )));
            }
        };

        for (key, value) in &headers {
            request = request.header(key, value);
        }

        request = request.multipart(form);

        let response = request
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Http(format!("请求失败: {}", e)))?;

        if !response.status().is_success() {
            return Err(openclaw_core::OpenClawError::AIProvider(format!(
                "API 错误: {}",
                response.status()
            )));
        }

        match self.config.response_type {
            CustomSttResponseType::Text => {
                let text = response.text().await.map_err(|e| {
                    openclaw_core::OpenClawError::Http(format!("读取响应失败: {}", e))
                })?;

                Ok(TranscriptionResult {
                    text,
                    language: None,
                    confidence: None,
                    duration: None,
                })
            }
            CustomSttResponseType::Json { ref text_field } => {
                let json: serde_json::Value = response.json().await.map_err(|e| {
                    openclaw_core::OpenClawError::Http(format!("解析 JSON 失败: {}", e))
                })?;

                let text = json
                    .get(text_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok(TranscriptionResult {
                    text,
                    language: None,
                    confidence: None,
                    duration: None,
                })
            }
        }
    }

    async fn is_available(&self) -> bool {
        !self.config.endpoint.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AudioFormat;

    #[test]
    fn test_custom_tts_config_default() {
        let config = CustomTtsConfig {
            name: "test".to_string(),
            endpoint: "https://api.example.com/tts".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_template: r#"{"text": "{{text}}", "voice": "{{voice}}"}"#.to_string(),
            response_type: CustomResponseType::Binary,
        };

        assert_eq!(config.name, "test");
        assert_eq!(config.method, "POST");
    }

    #[test]
    fn test_custom_tts_provider_build_request_body() {
        let config = CustomTtsConfig {
            name: "test".to_string(),
            endpoint: "https://api.example.com/tts".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_template: r#"{"text": "{{text}}", "voice": "{{voice}}", "speed": "{{speed}}", "format": "{{format}}"}"#.to_string(),
            response_type: CustomResponseType::Binary,
        };

        let provider = CustomTtsProvider::new(config);

        let options = SynthesisOptions {
            voice: Some("alloy".to_string()),
            speed: Some(1.2),
            format: Some(AudioFormat::Mp3),
            ..Default::default()
        };

        let body = provider.build_request_body("Hello world", &options);

        assert!(body.contains("Hello world"));
        assert!(body.contains("alloy"));
        assert!(body.contains("1.2"));
        assert!(body.contains("mp3"));
    }

    #[test]
    fn test_custom_tts_provider_build_request_body_defaults() {
        let config = CustomTtsConfig {
            name: "test".to_string(),
            endpoint: "https://api.example.com/tts".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_template: r#"{"text": "{{text}}", "voice": "{{voice}}", "speed": "{{speed}}", "format": "{{format}}"}"#.to_string(),
            response_type: CustomResponseType::Binary,
        };

        let provider = CustomTtsProvider::new(config);

        let options = SynthesisOptions::default();

        let body = provider.build_request_body("Test", &options);

        assert!(body.contains("Test"));
        assert!(body.contains("alloy"));
        assert!(body.contains("1"));
        assert!(body.contains("mp3"));
    }

    #[test]
    fn test_custom_response_type_json() {
        let response_type = CustomResponseType::Json {
            audio_field: "audio_data".to_string(),
        };

        match response_type {
            CustomResponseType::Json { audio_field } => {
                assert_eq!(audio_field, "audio_data");
            }
            _ => panic!("Expected Json variant"),
        }
    }

    #[test]
    fn test_custom_stt_config_default() {
        let config = CustomSttConfig {
            name: "test_stt".to_string(),
            endpoint: "https://api.example.com/stt".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            audio_field: "audio".to_string(),
            response_type: CustomSttResponseType::Text,
        };

        assert_eq!(config.name, "test_stt");
        assert_eq!(config.audio_field, "audio");
    }

    #[test]
    fn test_custom_stt_response_type_json() {
        let response_type = CustomSttResponseType::Json {
            text_field: "transcript".to_string(),
        };

        match response_type {
            CustomSttResponseType::Json { text_field } => {
                assert_eq!(text_field, "transcript");
            }
            _ => panic!("Expected Json variant"),
        }
    }
}
