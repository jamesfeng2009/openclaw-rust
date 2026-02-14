//! 语音合成 (TTS) 模块

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::types::{
    AudioFormat, OpenAIVoice, SynthesisOptions, TtsConfig, TtsModel, TtsProvider,
};

/// 语音合成 Trait
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    /// 获取提供商名称
    fn provider(&self) -> TtsProvider;

    /// 将文本转换为语音
    /// 
    /// # 参数
    /// - `text`: 要转换的文本
    /// - `options`: 合成选项
    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<u8>>;

    /// 将文本转换为语音并保存到文件
    async fn synthesize_to_file(
        &self,
        text: &str,
        output_path: &std::path::Path,
        options: Option<SynthesisOptions>,
    ) -> Result<()> {
        let audio_data = self.synthesize(text, options).await?;
        std::fs::write(output_path, audio_data)
            .map_err(|e| OpenClawError::Config(format!("写入音频文件失败: {}", e)))?;
        Ok(())
    }

    /// 检查是否可用
    async fn is_available(&self) -> bool;

    /// 获取支持的语音列表
    fn available_voices(&self) -> Vec<String>;
}

/// OpenAI TTS
pub struct OpenAITts {
    config: TtsConfig,
    client: Client,
}

impl OpenAITts {
    const API_URL: &'static str = "https://api.openai.com/v1/audio/speech";

    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_api_url(&self) -> String {
        self.config
            .openai_base_url
            .as_ref()
            .map(|base| format!("{}/audio/speech", base.trim_end_matches('/')))
            .unwrap_or_else(|| Self::API_URL.to_string())
    }

    fn get_api_key(&self) -> Result<String> {
        self.config
            .openai_api_key
            .clone()
            .ok_or_else(|| OpenClawError::Config("未配置 OpenAI API Key".to_string()))
    }

    fn get_response_format(&self, format: &AudioFormat) -> &'static str {
        match format {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "opus",
            AudioFormat::Flac => "flac",
            AudioFormat::Pcm => "pcm",
        }
    }
}

#[async_trait]
impl TextToSpeech for OpenAITts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::OpenAI
    }

    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<u8>> {
        let api_key = self.get_api_key()?;
        let url = self.get_api_url();

        let opts = options.unwrap_or_default();
        let voice = opts
            .voice
            .unwrap_or_else(|| self.config.default_voice.as_str().to_string());
        let speed = opts.speed.unwrap_or(self.config.default_speed);
        let format = opts.format.unwrap_or_else(|| self.config.default_format.clone());

        let body = serde_json::json!({
            "model": self.config.tts_model.as_str(),
            "input": text,
            "voice": voice,
            "speed": speed,
            "response_format": self.get_response_format(&format),
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("TTS API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "TTS API 错误: {}",
                error_text
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| OpenClawError::Http(format!("读取音频数据失败: {}", e)))?;

        Ok(audio_data.to_vec())
    }

    async fn is_available(&self) -> bool {
        self.config.openai_api_key.is_some()
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "alloy".to_string(),
            "echo".to_string(),
            "fable".to_string(),
            "onyx".to_string(),
            "nova".to_string(),
            "shimmer".to_string(),
        ]
    }
}

/// Edge TTS (免费)
pub struct EdgeTts {
    client: Client,
}

impl EdgeTts {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for EdgeTts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TextToSpeech for EdgeTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::Edge
    }

    async fn synthesize(
        &self,
        text: &str,
        options: Option<SynthesisOptions>,
    ) -> Result<Vec<u8>> {
        // Edge TTS 需要 WebSocket 连接，这里简化实现
        // 实际使用需要 edge-tts 库或自己实现 WebSocket 协议
        Err(OpenClawError::Config(
            "Edge TTS 需要安装额外依赖，请使用 OpenAI TTS".to_string(),
        ))
    }

    async fn is_available(&self) -> bool {
        // Edge TTS 不需要 API Key，始终可用
        // 但需要检查是否安装了 edge-tts CLI 工具
        false // 暂时禁用，需要额外实现
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "en-US-AriaNeural".to_string(),
            "en-US-GuyNeural".to_string(),
            "zh-CN-XiaoxiaoNeural".to_string(),
            "zh-CN-YunxiNeural".to_string(),
            "zh-CN-YunyangNeural".to_string(),
        ]
    }
}

/// 创建 TTS 实例
pub fn create_tts(provider: TtsProvider, config: TtsConfig) -> Box<dyn TextToSpeech> {
    match provider {
        TtsProvider::OpenAI => Box::new(OpenAITts::new(config)),
        TtsProvider::Edge => Box::new(EdgeTts::new()),
        TtsProvider::Azure => {
            unimplemented!("Azure TTS 尚未实现")
        }
        TtsProvider::Google => {
            unimplemented!("Google TTS 尚未实现")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_provider_default() {
        let provider = TtsProvider::default();
        assert_eq!(provider, TtsProvider::OpenAI);
    }

    #[test]
    fn test_openai_voices() {
        let tts = OpenAITts::new(TtsConfig::default());
        let voices = tts.available_voices();
        assert!(!voices.is_empty());
    }
}
