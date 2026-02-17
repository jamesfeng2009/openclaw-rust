use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_core::Result;
use openclaw_voice::{SpeechToText, TalkMode, TalkModeConfig, TextToSpeech, VoiceAgent};

pub struct AgentVoice {
    voice_agent: Option<VoiceAgent>,
    stt: Option<Arc<dyn SpeechToText>>,
    tts: Option<Arc<dyn TextToSpeech>>,
    talk_mode: Option<TalkMode>,
    enabled: Arc<RwLock<bool>>,
}

impl AgentVoice {
    pub fn new() -> Self {
        Self {
            voice_agent: None,
            stt: None,
            tts: None,
            talk_mode: None,
            enabled: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_stt(mut self, stt: Arc<dyn SpeechToText>) -> Self {
        self.stt = Some(stt);
        self
    }

    pub fn with_tts(mut self, tts: Arc<dyn TextToSpeech>) -> Self {
        self.tts = Some(tts);
        self
    }

    pub fn with_talk_mode(mut self, talk_mode: TalkMode) -> Self {
        self.talk_mode = Some(talk_mode);
        self
    }

    pub fn build(self) -> Self {
        let mut this = self;
        if let (Some(stt), Some(tts)) = (this.stt.clone(), this.tts.clone()) {
            let talk_config = TalkModeConfig::default();

            this.voice_agent = Some(VoiceAgent::new(stt, tts, talk_config));
        }
        this
    }

    pub async fn enable(&self) -> Result<()> {
        if let Some(talk_mode) = &self.talk_mode {
            talk_mode.start().await?;
        }
        *self.enabled.write().await = true;
        Ok(())
    }

    pub async fn disable(&self) -> Result<()> {
        if let Some(talk_mode) = &self.talk_mode {
            talk_mode.stop().await?;
        }
        *self.enabled.write().await = false;
        Ok(())
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn process_voice_input(&self, audio_data: &[u8]) -> Result<String> {
        let agent = self.voice_agent.as_ref().ok_or_else(|| {
            openclaw_core::OpenClawError::Config("Voice agent not initialized".to_string())
        })?;

        agent.process_audio(audio_data).await
    }

    pub async fn synthesize_voice(&self, text: &str) -> Result<Vec<u8>> {
        let agent = self.voice_agent.as_ref().ok_or_else(|| {
            openclaw_core::OpenClawError::Config("Voice agent not initialized".to_string())
        })?;

        agent.speak(text).await
    }

    pub fn talk_mode(&self) -> Option<&TalkMode> {
        self.talk_mode.as_ref()
    }

    pub fn stt(&self) -> Option<&Arc<dyn SpeechToText>> {
        self.stt.as_ref()
    }

    pub fn tts(&self) -> Option<&Arc<dyn TextToSpeech>> {
        self.tts.as_ref()
    }
}

impl Default for AgentVoice {
    fn default() -> Self {
        Self::new()
    }
}
