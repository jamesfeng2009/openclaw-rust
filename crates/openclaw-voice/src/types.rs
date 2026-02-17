//! 语音模块类型定义

use serde::{Deserialize, Serialize};

use crate::provider::CustomProviderConfig;

/// STT 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SttProvider {
    /// OpenAI Whisper API
    OpenAI,
    /// 本地 Whisper 模型
    LocalWhisper,
    /// Azure Speech
    Azure,
    /// Google Cloud Speech
    Google,
    /// 自定义提供商 (用户配置)
    Custom(String),
}

impl Default for SttProvider {
    fn default() -> Self {
        Self::OpenAI
    }
}

/// TTS 提供商
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TtsProvider {
    /// OpenAI TTS API
    OpenAI,
    /// Edge TTS (免费)
    Edge,
    /// Azure Speech
    Azure,
    /// Google Cloud TTS
    Google,
    /// ElevenLabs TTS
    ElevenLabs,
    /// 自定义提供商 (用户配置)
    Custom(String),
}

impl Default for TtsProvider {
    fn default() -> Self {
        Self::OpenAI
    }
}

/// 语音识别结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// 识别的文本
    pub text: String,
    /// 语言（自动检测）
    pub language: Option<String>,
    /// 置信度 (0.0 - 1.0)
    pub confidence: Option<f32>,
    /// 持续时间（秒）
    pub duration: Option<f64>,
}

/// 语音合成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisOptions {
    /// 语音名称
    pub voice: Option<String>,
    /// 语速 (0.25 - 4.0)
    pub speed: Option<f32>,
    /// 音调 (仅部分提供商支持)
    pub pitch: Option<f32>,
    /// 输出格式
    pub format: Option<AudioFormat>,
}

impl Default for SynthesisOptions {
    fn default() -> Self {
        Self {
            voice: None,
            speed: Some(1.0),
            pitch: None,
            format: Some(AudioFormat::Mp3),
        }
    }
}

/// 音频格式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    #[default]
    Mp3,
    Wav,
    Ogg,
    Flac,
    Pcm,
}

impl AudioFormat {
    pub fn as_extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::Ogg => "ogg",
            AudioFormat::Flac => "flac",
            AudioFormat::Pcm => "pcm",
        }
    }

    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Ogg => "audio/ogg",
            AudioFormat::Flac => "audio/flac",
            AudioFormat::Pcm => "audio/pcm",
        }
    }
}

/// OpenAI TTS 可用语音
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIVoice {
    Alloy,
    Echo,
    Fable,
    Onyx,
    Nova,
    Shimmer,
}

impl OpenAIVoice {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIVoice::Alloy => "alloy",
            OpenAIVoice::Echo => "echo",
            OpenAIVoice::Fable => "fable",
            OpenAIVoice::Onyx => "onyx",
            OpenAIVoice::Nova => "nova",
            OpenAIVoice::Shimmer => "shimmer",
        }
    }
}

impl Default for OpenAIVoice {
    fn default() -> Self {
        Self::Alloy
    }
}

/// OpenAI Whisper 可用模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperModel {
    Whisper1,
}

impl WhisperModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            WhisperModel::Whisper1 => "whisper-1",
        }
    }
}

impl Default for WhisperModel {
    fn default() -> Self {
        Self::Whisper1
    }
}

/// OpenAI TTS 可用模型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TtsModel {
    Tts1,
    Tts1Hd,
}

impl TtsModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsModel::Tts1 => "tts-1",
            TtsModel::Tts1Hd => "tts-1-hd",
        }
    }
}

impl Default for TtsModel {
    fn default() -> Self {
        Self::Tts1
    }
}

/// Talk Mode 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TalkModeState {
    /// 空闲
    Idle,
    /// 监听中
    Listening,
    /// 处理中
    Processing,
    /// 播放回复中
    Speaking,
}

/// 语音配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// STT 提供商
    pub stt_provider: SttProvider,
    /// TTS 提供商
    pub tts_provider: TtsProvider,
    /// STT 配置
    #[serde(flatten)]
    pub stt_config: SttConfig,
    /// TTS 配置
    #[serde(flatten)]
    pub tts_config: TtsConfig,
    /// 是否启用
    pub enabled: bool,
    /// 自定义提供商配置
    #[serde(default)]
    pub custom_providers: Option<CustomProviderConfig>,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_provider: SttProvider::OpenAI,
            tts_provider: TtsProvider::OpenAI,
            stt_config: SttConfig::default(),
            tts_config: TtsConfig::default(),
            enabled: false,
            custom_providers: None,
        }
    }
}

/// STT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    /// OpenAI API Key
    pub openai_api_key: Option<String>,
    /// OpenAI Base URL
    pub openai_base_url: Option<String>,
    /// Whisper 模型
    #[serde(default)]
    pub whisper_model: WhisperModel,
    /// 语言提示
    pub language: Option<String>,
    /// 本地模型路径
    pub local_model_path: Option<String>,
    /// Azure Speech API Key
    pub azure_api_key: Option<String>,
    /// Azure Speech 区域
    pub azure_region: Option<String>,
    /// Google Cloud API Key
    pub google_api_key: Option<String>,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            openai_base_url: None,
            whisper_model: WhisperModel::default(),
            language: None,
            local_model_path: None,
            azure_api_key: None,
            azure_region: None,
            google_api_key: None,
        }
    }
}

/// TTS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// OpenAI API Key
    pub openai_api_key: Option<String>,
    /// OpenAI Base URL
    pub openai_base_url: Option<String>,
    /// TTS 模型
    #[serde(default)]
    pub tts_model: TtsModel,
    /// 默认语音
    #[serde(default)]
    pub default_voice: OpenAIVoice,
    /// 默认语速
    #[serde(default = "default_speed")]
    pub default_speed: f32,
    /// 默认格式
    #[serde(default)]
    pub default_format: AudioFormat,
    /// ElevenLabs API Key
    pub elevenlabs_api_key: Option<String>,
    /// ElevenLabs Model ID
    #[serde(default = "default_elevenlabs_model")]
    pub elevenlabs_model: String,
    /// Azure Speech API Key
    pub azure_api_key: Option<String>,
    /// Azure Speech 区域
    pub azure_region: Option<String>,
    /// Google Cloud API Key
    pub google_api_key: Option<String>,
}

fn default_speed() -> f32 {
    1.0
}

fn default_elevenlabs_model() -> String {
    "eleven_multilingual_v2".to_string()
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            openai_base_url: None,
            tts_model: TtsModel::default(),
            default_voice: OpenAIVoice::default(),
            default_speed: 1.0,
            default_format: AudioFormat::Mp3,
            elevenlabs_api_key: None,
            elevenlabs_model: "eleven_multilingual_v2".to_string(),
            azure_api_key: None,
            azure_region: None,
            google_api_key: None,
        }
    }
}
