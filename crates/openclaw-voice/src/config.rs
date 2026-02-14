//! 语音配置管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::types::{SttConfig, TtsConfig, SttProvider, TtsProvider, VoiceConfig};
use openclaw_core::{OpenClawError, Result};

/// 语音配置管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfigManager {
    /// 语音配置
    pub voice: VoiceConfig,
}

impl VoiceConfigManager {
    /// 配置文件路径
    fn get_config_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".openclaw").join("voice.json")
    }

    /// 加载配置
    pub fn load() -> Self {
        let path = Self::get_config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// 保存配置
    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| OpenClawError::Config(format!("创建配置目录失败: {}", e)))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| OpenClawError::Serialization(e))?;
        std::fs::write(&path, &content)
            .map_err(|e| OpenClawError::Config(format!("保存配置失败: {}", e)))?;
        Ok(())
    }

    /// 设置 STT API Key
    pub fn set_stt_api_key(&mut self, provider: SttProvider, api_key: String) {
        match provider {
            SttProvider::OpenAI => {
                self.voice.stt_config.openai_api_key = Some(api_key);
            }
            _ => {}
        }
    }

    /// 设置 TTS API Key
    pub fn set_tts_api_key(&mut self, provider: TtsProvider, api_key: String) {
        match provider {
            TtsProvider::OpenAI => {
                self.voice.tts_config.openai_api_key = Some(api_key);
            }
            _ => {}
        }
    }

    /// 设置 OpenAI Base URL
    pub fn set_openai_base_url(&mut self, base_url: String) {
        self.voice.stt_config.openai_base_url = Some(base_url.clone());
        self.voice.tts_config.openai_base_url = Some(base_url);
    }

    /// 启用/禁用语音功能
    pub fn set_enabled(&mut self, enabled: bool) {
        self.voice.enabled = enabled;
    }
}

impl Default for VoiceConfigManager {
    fn default() -> Self {
        Self {
            voice: VoiceConfig::default(),
        }
    }
}

/// 创建默认配置
pub fn default_voice_config() -> VoiceConfig {
    VoiceConfig {
        stt_provider: SttProvider::OpenAI,
        tts_provider: TtsProvider::OpenAI,
        stt_config: SttConfig::default(),
        tts_config: TtsConfig::default(),
        enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_config_manager_default() {
        let manager = VoiceConfigManager::default();
        assert!(!manager.voice.enabled);
        assert_eq!(manager.voice.stt_provider, SttProvider::OpenAI);
        assert_eq!(manager.voice.tts_provider, TtsProvider::OpenAI);
    }
}
