//! 语音识别 (STT) 模块

mod local;

use async_trait::async_trait;
use base64::Engine;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::types::{SttConfig, SttProvider, TranscriptionResult};

pub use local::{LocalWhisperConfig, LocalWhisperStt, WhisperModelInfo, WhisperModelType};

/// 语音识别 Trait
#[async_trait]
pub trait SpeechToText: Send + Sync {
    /// 获取提供商名称
    fn provider(&self) -> SttProvider;

    /// 转录音频数据
    /// 
    /// # 参数
    /// - `audio_data`: 音频数据 (WAV/MP3/M4A 等格式)
    /// - `language`: 语言提示 (可选)
    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<TranscriptionResult>;

    /// 转录音频文件
    async fn transcribe_file(
        &self,
        file_path: &std::path::Path,
        language: Option<&str>,
    ) -> Result<TranscriptionResult> {
        let audio_data = std::fs::read(file_path)
            .map_err(|e| OpenClawError::Config(format!("读取音频文件失败: {}", e)))?;
        self.transcribe(&audio_data, language).await
    }

    /// 检查是否可用
    async fn is_available(&self) -> bool;
}

/// OpenAI Whisper STT
pub struct OpenAIWhisperStt {
    config: SttConfig,
    client: Client,
}

impl OpenAIWhisperStt {
    const API_URL: &'static str = "https://api.openai.com/v1/audio/transcriptions";

    pub fn new(config: SttConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_api_url(&self) -> String {
        self.config
            .openai_base_url
            .as_ref()
            .map(|base| format!("{}/audio/transcriptions", base.trim_end_matches('/')))
            .unwrap_or_else(|| Self::API_URL.to_string())
    }

    fn get_api_key(&self) -> Result<String> {
        self.config
            .openai_api_key
            .clone()
            .ok_or_else(|| OpenClawError::Config("未配置 OpenAI API Key".to_string()))
    }
}

#[async_trait]
impl SpeechToText for OpenAIWhisperStt {
    fn provider(&self) -> SttProvider {
        SttProvider::OpenAI
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<TranscriptionResult> {
        let api_key = self.get_api_key()?;
        let url = self.get_api_url();

        // 构建 multipart 表单
        let mut form = reqwest::multipart::Form::new()
            .text("model", self.config.whisper_model.as_str().to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.mp3")
                    .mime_str("audio/mpeg")
                    .map_err(|e| OpenClawError::Http(format!("创建 multipart 失败: {}", e)))?,
            );

        // 添加语言参数
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        } else if let Some(lang) = &self.config.language {
            form = form.text("language", lang.clone());
        }

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Whisper API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Whisper API 错误: {}",
                error_text
            )));
        }

        let result: WhisperResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(TranscriptionResult {
            text: result.text,
            language: result.language,
            duration: result.duration,
            confidence: None,
        })
    }

    async fn is_available(&self) -> bool {
        self.config.openai_api_key.is_some()
    }
}

/// OpenAI Whisper API 响应
#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: String,
    language: Option<String>,
    duration: Option<f64>,
}

/// Azure Speech STT
 pub struct AzureStt {
     config: SttConfig,
     client: Client,
 }

 impl AzureStt {
     pub fn new(config: SttConfig) -> Self {
         Self {
             config,
             client: Client::new(),
         }
     }

     fn get_endpoint(&self) -> Result<String> {
         let region = self.config.azure_region.as_ref()
             .ok_or_else(|| OpenClawError::Config("Azure region 未配置".to_string()))?;
         Ok(format!(
             "https://{}.stt.speech.microsoft.com/speech/recognition/conversation/cognitiveservices/v1",
             region
         ))
     }

     fn get_api_key(&self) -> Result<String> {
         self.config.azure_api_key.clone()
             .ok_or_else(|| OpenClawError::Config("Azure API Key 未配置".to_string()))
     }
 }

#[async_trait]
impl SpeechToText for AzureStt {
    fn provider(&self) -> SttProvider {
        SttProvider::Azure
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<TranscriptionResult> {
        let api_key = self.get_api_key()?;
        let endpoint = self.get_endpoint()?;
        
        let lang = language.or(self.config.language.as_deref()).unwrap_or("zh-CN");

        let form = reqwest::multipart::Form::new()
            .text("language", lang.to_string())
            .text("format", "detailed")
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| OpenClawError::Http(format!("创建 multipart 失败: {}", e)))?,
            );

        let response = self.client
            .post(&endpoint)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .multipart(form)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Azure STT 请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Azure STT 错误: {} - {}",
                status, error_text
            )));
        }

        let result: AzureSttResponse = response.json().await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(TranscriptionResult {
            text: result.DisplayText,
            language: Some(lang.to_string()),
            duration: result.Duration.map(|d| d as f64 / 1000.0),
            confidence: None,
        })
    }

    async fn is_available(&self) -> bool {
        self.config.azure_api_key.is_some() && self.config.azure_region.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct AzureSttResponse {
    DisplayText: String,
    Duration: Option<i64>,
    Offset: Option<i64>,
}

/// Google Cloud STT
pub struct GoogleStt {
    config: SttConfig,
    client: Client,
}

impl GoogleStt {
    pub fn new(config: SttConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_api_key(&self) -> Result<String> {
        self.config.google_api_key.clone()
            .ok_or_else(|| OpenClawError::Config("Google API Key 未配置".to_string()))
    }
}

#[async_trait]
impl SpeechToText for GoogleStt {
    fn provider(&self) -> SttProvider {
        SttProvider::Google
    }

    async fn transcribe(
        &self,
        audio_data: &[u8],
        language: Option<&str>,
    ) -> Result<TranscriptionResult> {
        let api_key = self.get_api_key()?;
        let lang = language.or(self.config.language.as_deref()).unwrap_or("zh-CN");

        let audio_base64 = base64::engine::general_purpose::STANDARD.encode(audio_data);

        let request_body = serde_json::json!({
            "config": {
                "encoding": "LINEAR16",
                "sampleRateHertz": 16000,
                "languageCode": lang
            },
            "audio": {
                "content": audio_base64
            }
        });

        let url = format!(
            "https://speech.googleapis.com/v1/speech:recognize?key={}",
            api_key
        );

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Google STT 请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Google STT 错误: {} - {}",
                status, error_text
            )));
        }

        let result: GoogleSttResponse = response.json().await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        let mut text = String::new();
        if let Some(results) = result.results {
            for result in results {
                if let Some(alternatives) = result.alternatives {
                    if let Some(alt) = alternatives.first() {
                        text = alt.transcript.clone();
                        break;
                    }
                }
            }
        }

        Ok(TranscriptionResult {
            text,
            language: Some(lang.to_string()),
            duration: None,
            confidence: None,
        })
    }

    async fn is_available(&self) -> bool {
        self.config.google_api_key.is_some()
    }
}

#[derive(Debug, Deserialize)]
struct GoogleSttResponse {
    results: Option<Vec<GoogleSttResult>>,
}

#[derive(Debug, Deserialize)]
struct GoogleSttResult {
    alternatives: Option<Vec<GoogleSttAlternative>>,
}

#[derive(Debug, Deserialize)]
struct GoogleSttAlternative {
    transcript: String,
    confidence: Option<f32>,
}

/// 创建 STT 实例
pub fn create_stt(provider: SttProvider, config: SttConfig) -> Box<dyn SpeechToText> {
    match provider {
        SttProvider::OpenAI => Box::new(OpenAIWhisperStt::new(config)),
        SttProvider::LocalWhisper => {
            let local_config = LocalWhisperConfig {
                model_path: config.local_model_path.clone().unwrap_or_default(),
                language: config.language.clone(),
                ..Default::default()
            };
            Box::new(LocalWhisperStt::new(local_config))
        }
        SttProvider::Azure => Box::new(AzureStt::new(config)),
        SttProvider::Google => Box::new(GoogleStt::new(config)),
        SttProvider::Custom(_) => Box::new(OpenAIWhisperStt::new(config)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stt_provider_default() {
        let provider = SttProvider::default();
        assert_eq!(provider, SttProvider::OpenAI);
    }
}
