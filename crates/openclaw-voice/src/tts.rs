//! 语音合成 (TTS) 模块

use async_trait::async_trait;
use base64::Engine;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;

use crate::types::{AudioFormat, SynthesisOptions, TtsConfig, TtsProvider};

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
    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>>;

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

    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>> {
        let api_key = self.get_api_key()?;
        let url = self.get_api_url();

        let opts = options.unwrap_or_default();
        let voice = opts
            .voice
            .unwrap_or_else(|| self.config.default_voice.as_str().to_string());
        let speed = opts.speed.unwrap_or(self.config.default_speed);
        let format = opts
            .format
            .unwrap_or_else(|| self.config.default_format.clone());

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

    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>> {
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

/// ElevenLabs TTS
pub struct ElevenLabsTts {
    config: TtsConfig,
    client: Client,
}

impl ElevenLabsTts {
    const API_URL: &'static str = "https://api.elevenlabs.io/v1/text-to-speech";

    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_api_key(&self) -> Result<String> {
        self.config
            .elevenlabs_api_key
            .clone()
            .ok_or_else(|| OpenClawError::Config("未配置 ElevenLabs API Key".to_string()))
    }
}

impl Default for ElevenLabsTts {
    fn default() -> Self {
        Self::new(TtsConfig::default())
    }
}

#[async_trait]
impl TextToSpeech for ElevenLabsTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::ElevenLabs
    }

    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>> {
        let api_key = self.get_api_key()?;
        let model = &self.config.elevenlabs_model;

        let voice_id = options
            .as_ref()
            .and_then(|o| o.voice.clone())
            .unwrap_or_else(|| "rachel".to_string());

        let request_body = serde_json::json!({
            "text": text,
            "model_id": model,
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.8
            }
        });

        let url = format!("{}/{}", Self::API_URL, voice_id);

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OpenClawError::Config(format!("ElevenLabs 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Config(format!(
                "ElevenLabs API 错误 ({}): {}",
                status, error_text
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| OpenClawError::Config(format!("读取音频数据失败: {}", e)))?;

        Ok(audio_data.to_vec())
    }

    async fn is_available(&self) -> bool {
        self.config.elevenlabs_api_key.is_some()
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "rachel".to_string(),
            "adam".to_string(),
            "sam".to_string(),
            "bella".to_string(),
            "josh".to_string(),
            "arnold".to_string(),
            "cherry".to_string(),
            "domi".to_string(),
            "elliot".to_string(),
            "finsbury".to_string(),
            "danielle".to_string(),
            "olivia".to_string(),
            "onyx".to_string(),
            "pearl".to_string(),
            "shimmer".to_string(),
        ]
    }
}

/// 创建 TTS 实例
pub fn create_tts(provider: TtsProvider, config: TtsConfig) -> Box<dyn TextToSpeech> {
    match provider {
        TtsProvider::OpenAI => Box::new(OpenAITts::new(config.clone())),
        TtsProvider::Edge => Box::new(EdgeTts::new()),
        TtsProvider::Azure => Box::new(AzureTts::new(config)),
        TtsProvider::Google => Box::new(GoogleTts::new(config)),
        TtsProvider::ElevenLabs => Box::new(ElevenLabsTts::new(config)),
        TtsProvider::Custom(_) => Box::new(OpenAITts::new(config)),
    }
}

/// Azure TTS
pub struct AzureTts {
    config: TtsConfig,
    client: Client,
}

impl AzureTts {
    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_endpoint(&self) -> Result<String> {
        let region = self
            .config
            .azure_region
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("Azure region 未配置".to_string()))?;
        Ok(format!(
            "https://{}.tts.speech.microsoft.com/cognitiveservices/v1",
            region
        ))
    }

    fn get_api_key(&self) -> Result<String> {
        self.config
            .azure_api_key
            .clone()
            .ok_or_else(|| OpenClawError::Config("Azure API Key 未配置".to_string()))
    }
}

#[async_trait]
impl TextToSpeech for AzureTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::Azure
    }

    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>> {
        let api_key = self.get_api_key()?;
        let endpoint = self.get_endpoint()?;
        let opts = options.unwrap_or_default();

        let voice = opts
            .voice
            .unwrap_or_else(|| "zh-CN-XiaoxiaoNeural".to_string());
        let speed = opts.speed.unwrap_or(1.0);
        let pitch = opts.pitch.unwrap_or(0.0);

        let ssml = format!(
            r#"<speak version='1.0' xml:lang='zh-CN'><voice name='{}'><prosody rate='{}%' pitch='{}%'>{}</prosody></voice></speak>"#,
            voice,
            (speed - 1.0) * 100.0,
            pitch * 10.0,
            text
        );

        let response = self
            .client
            .post(&endpoint)
            .header("Ocp-Apim-Subscription-Key", api_key)
            .header("Content-Type", "application/ssml+xml")
            .header(
                "X-Microsoft-OutputFormat",
                "audio-24khz-48kbitrate-mono-mp3",
            )
            .body(ssml)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Azure TTS 请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Azure TTS 错误: {} - {}",
                status, error_text
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| OpenClawError::Http(format!("读取音频数据失败: {}", e)))?;

        Ok(audio_data.to_vec())
    }

    async fn is_available(&self) -> bool {
        self.config.azure_api_key.is_some() && self.config.azure_region.is_some()
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "zh-CN-XiaoxiaoNeural".to_string(),
            "zh-CN-YunxiNeural".to_string(),
            "zh-CN-YunyangNeural".to_string(),
            "en-US-AriaNeural".to_string(),
            "en-US-GuyNeural".to_string(),
            "ja-JP-NanamiNeural".to_string(),
            "ko-JI-YoungjiNeural".to_string(),
        ]
    }
}

/// Google Cloud TTS
pub struct GoogleTts {
    config: TtsConfig,
    client: Client,
}

impl GoogleTts {
    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    fn get_api_key(&self) -> Result<String> {
        self.config
            .google_api_key
            .clone()
            .ok_or_else(|| OpenClawError::Config("Google API Key 未配置".to_string()))
    }
}

#[async_trait]
impl TextToSpeech for GoogleTts {
    fn provider(&self) -> TtsProvider {
        TtsProvider::Google
    }

    async fn synthesize(&self, text: &str, options: Option<SynthesisOptions>) -> Result<Vec<u8>> {
        let api_key = self.get_api_key()?;
        let opts = options.unwrap_or_default();

        let voice = opts.voice.unwrap_or_else(|| "en-US-Neural2-F".to_string());
        let speed = opts.speed.unwrap_or(1.0);
        let pitch = opts.pitch.unwrap_or(0.0);

        let (language_code, voice_name) = if voice.contains('-') {
            let parts: Vec<&str> = voice.split('-').collect();
            if parts.len() >= 2 {
                (format!("{}-{}", parts[0], parts[1]), voice.clone())
            } else {
                ("en-US".to_string(), voice)
            }
        } else {
            (
                "en-US".to_string(),
                format!("en-US-Neural2-{}", if voice == "male" { "M" } else { "F" }),
            )
        };

        let request_body = serde_json::json!({
            "input": { "text": text },
            "voice": {
                "languageCode": language_code,
                "name": voice_name
            },
            "audioConfig": {
                "audioEncoding": "MP3",
                "speakingRate": speed,
                "pitch": pitch
            }
        });

        let url = format!(
            "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
            api_key
        );

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Google TTS 请求失败: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Google TTS 错误: {} - {}",
                status, error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        let audio_content = json
            .get("audioContent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OpenClawError::Config("响应中未找到 audioContent".to_string()))?;

        let audio_data = base64::engine::general_purpose::STANDARD
            .decode(audio_content)
            .map_err(|e| OpenClawError::Config(format!("Base64 解码失败: {}", e)))?;

        Ok(audio_data)
    }

    async fn is_available(&self) -> bool {
        self.config.google_api_key.is_some()
    }

    fn available_voices(&self) -> Vec<String> {
        vec![
            "en-US-Neural2-F".to_string(),
            "en-US-Neural2-M".to_string(),
            "zh-CN-Neural2-A".to_string(),
            "ja-JP-Neural2-B".to_string(),
            "ko-KR-Neural2-A".to_string(),
        ]
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
