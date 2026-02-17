use std::sync::Arc;
use tokio::sync::RwLock;

use crate::stt::SpeechToText;
use crate::talk_mode::{TalkMode, TalkModeConfig};
use crate::tts::TextToSpeech;
use openclaw_core::Result;

pub struct VoiceAgent {
    stt: Arc<dyn SpeechToText>,
    tts: Arc<dyn TextToSpeech>,
    talk_mode: TalkMode,
    running: Arc<RwLock<bool>>,
}

impl VoiceAgent {
    pub fn new(
        stt: Arc<dyn SpeechToText>,
        tts: Arc<dyn TextToSpeech>,
        config: TalkModeConfig,
    ) -> Self {
        Self {
            stt,
            tts,
            talk_mode: TalkMode::new(config),
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn talk_mode(&self) -> &TalkMode {
        &self.talk_mode
    }

    pub async fn start(&self) -> Result<()> {
        self.talk_mode.start().await
    }

    pub async fn stop(&self) -> Result<()> {
        self.talk_mode.stop().await
    }

    pub async fn is_running(&self) -> bool {
        self.talk_mode.is_running().await
    }

    pub async fn process_audio(&self, audio_data: &[u8]) -> Result<String> {
        let result = self.stt.transcribe(audio_data, None).await?;
        let text = result.text.clone();
        self.talk_mode.on_transcription(text.clone()).await?;
        Ok(text)
    }

    pub async fn speak(&self, text: &str) -> Result<Vec<u8>> {
        let audio = self.tts.synthesize(text, None).await?;
        self.talk_mode.on_ai_response(text.to_string()).await?;
        Ok(audio)
    }
}
