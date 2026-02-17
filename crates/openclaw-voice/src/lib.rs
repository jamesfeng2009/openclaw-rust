//! OpenClaw Voice - 语音识别与合成模块
//!
//! 提供统一的语音接口，支持多种 STT/TTS 提供商
//!
//! ## 功能
//! - 语音识别 (STT) - 将语音转换为文本
//! - 语音合成 (TTS) - 将文本转换为语音
//! - 持续对话模式 (Talk Mode)
//! - 语音唤醒 (Voice Wake)
//!
//! ## 支持的提供商
//! - OpenAI Whisper (STT)
//! - OpenAI TTS
//! - Edge TTS (免费)
//! - 本地 Whisper (可选)
//! - Vosk (本地唤醒)

pub mod stt;
pub mod tts;
pub mod talk_mode;
pub mod audio;
pub mod types;
pub mod config;
pub mod wake;
pub mod voice_agent;
pub mod provider;

pub use stt::*;
pub use tts::*;
pub use talk_mode::*;
pub use audio::AudioUtils;
pub use types::*;
pub use config::*;
pub use wake::*;
pub use voice_agent::*;
pub use provider::{ProviderRegistry, CustomProviderConfig, CustomTtsConfig, CustomSttConfig};
