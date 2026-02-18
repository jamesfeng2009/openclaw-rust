use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_voice::{SpeechToText, TextToSpeech, VoiceAgent, TalkModeConfig};

pub struct VoiceService {
    enabled: Arc<RwLock<bool>>,
    voice_agent: Arc<RwLock<Option<VoiceAgent>>>,
}

impl VoiceService {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            voice_agent: Arc::new(RwLock::new(None)),
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
        let config = TalkModeConfig::default();
        let agent = VoiceAgent::new(stt, tts, config);
        *self.voice_agent.write().await = Some(agent);
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
