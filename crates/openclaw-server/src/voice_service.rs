use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_voice::{SpeechToText, TextToSpeech, VoiceAgent, TalkModeConfig};

pub struct VoiceService {
    enabled: Arc<RwLock<bool>>,
    voice_agent: Arc<RwLock<Option<VoiceAgent>>>,
    stt: Arc<RwLock<Option<Arc<dyn SpeechToText>>>>,
}

impl VoiceService {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            voice_agent: Arc::new(RwLock::new(None)),
            stt: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn enable(&self) {
        *self.enabled.write().await = true;
    }

    pub async fn disable(&self) {
        *self.enabled.write().await = false;
    }

    pub async fn toggle(&self) -> bool {
        let mut enabled = self.enabled.write().await;
        *enabled = !*enabled;
        *enabled
    }

    pub async fn init_voice(&self, stt: Arc<dyn SpeechToText>, tts: Arc<dyn TextToSpeech>) {
        let stt_clone = stt.clone();
        *self.stt.write().await = Some(stt);
        let config = TalkModeConfig::default();
        let agent = VoiceAgent::new(stt_clone, tts, config);
        *self.voice_agent.write().await = Some(agent);
    }

    pub async fn speech_to_text(&self, audio_data: &[u8], language: Option<&str>) -> openclaw_core::Result<String> {
        let stt = self.stt.read().await;
        if let Some(stt_provider) = stt.as_ref() {
            let result = stt_provider.transcribe(audio_data, language).await?;
            Ok(result.text)
        } else {
            Err(openclaw_core::OpenClawError::Execution("STT not initialized".to_string()))
        }
    }

    pub async fn start_voice(&self) -> Option<openclaw_core::Result<()>> {
        let agent = self.voice_agent.read().await;
        if let Some(voice_agent) = agent.as_ref() {
            Some(voice_agent.start().await)
        } else {
            None
        }
    }

    pub async fn stop_voice(&self) -> Option<openclaw_core::Result<()>> {
        let agent = self.voice_agent.read().await;
        if let Some(voice_agent) = agent.as_ref() {
            Some(voice_agent.stop().await)
        } else {
            None
        }
    }

    pub async fn speak(&self, text: &str) -> Option<openclaw_core::Result<Vec<u8>>> {
        let agent = self.voice_agent.read().await;
        if let Some(voice_agent) = agent.as_ref() {
            Some(voice_agent.speak(text).await)
        } else {
            None
        }
    }

    pub async fn process_audio(&self, audio_data: &[u8]) -> Option<openclaw_core::Result<String>> {
        let agent = self.voice_agent.read().await;
        if let Some(voice_agent) = agent.as_ref() {
            Some(voice_agent.process_audio(audio_data).await)
        } else {
            None
        }
    }

    pub async fn is_running(&self) -> bool {
        let agent = self.voice_agent.read().await;
        if let Some(voice_agent) = agent.as_ref() {
            voice_agent.is_running().await
        } else {
            false
        }
    }
}

impl Default for VoiceService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voice_service_new() {
        let service = VoiceService::new();
        assert!(!service.is_enabled().await);
    }

    #[tokio::test]
    async fn test_voice_service_enable_disable() {
        let service = VoiceService::new();
        
        service.enable().await;
        assert!(service.is_enabled().await);
        
        service.disable().await;
        assert!(!service.is_enabled().await);
    }

    #[tokio::test]
    async fn test_voice_service_toggle() {
        let service = VoiceService::new();
        
        let enabled = service.toggle().await;
        assert!(enabled);
        
        let enabled = service.toggle().await;
        assert!(!enabled);
    }

    #[tokio::test]
    async fn test_voice_service_speech_to_text_not_initialized() {
        let service = VoiceService::new();
        
        let result = service.speech_to_text(b"test audio", Some("en")).await;
        assert!(result.is_err());
    }
}
