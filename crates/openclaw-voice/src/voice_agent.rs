use std::sync::Arc;
use tokio::sync::RwLock;

use crate::stt::SpeechToText;
use crate::talk_mode::{TalkMode, TalkModeConfig};
use crate::tts::TextToSpeech;
use crate::dialogue::{DialogueContext, DialogueContextManager, IntentRecognizer, KeywordIntentRecognizer};
use crate::{EnergyVad, SimpleAgc, VoiceActivityDetector, AgcProcessor, NoiseSuppressor, SpectralSubtraction, VadConfig};
use openclaw_core::Result;

pub struct VoiceAgent {
    stt: Arc<dyn SpeechToText>,
    tts: Arc<dyn TextToSpeech>,
    talk_mode: TalkMode,
    running: Arc<RwLock<bool>>,
    dialogue_manager: DialogueContextManager,
    current_context: Arc<RwLock<Option<DialogueContext>>>,
    intent_recognizer: Arc<dyn IntentRecognizer>,
    vad: Arc<dyn VoiceActivityDetector>,
    agc: Arc<dyn AgcProcessor>,
    noise_suppressor: Arc<dyn NoiseSuppressor>,
}

pub struct VoiceAgentBuilder {
    stt: Option<Arc<dyn SpeechToText>>,
    tts: Option<Arc<dyn TextToSpeech>>,
    config: Option<TalkModeConfig>,
    dialogue_manager: Option<DialogueContextManager>,
    intent_recognizer: Option<Arc<dyn IntentRecognizer>>,
    vad: Option<Arc<dyn VoiceActivityDetector>>,
    agc: Option<Arc<dyn AgcProcessor>>,
    noise_suppressor: Option<Arc<dyn NoiseSuppressor>>,
}

impl VoiceAgentBuilder {
    pub fn new() -> Self {
        Self {
            stt: None,
            tts: None,
            config: None,
            dialogue_manager: Some(DialogueContextManager::new(100)),
            intent_recognizer: Some(Arc::new(KeywordIntentRecognizer::new())),
            vad: Some(Arc::new(EnergyVad::with_default_config())),
            agc: Some(Arc::new(SimpleAgc::with_default_config())),
            noise_suppressor: Some(Arc::new(SpectralSubtraction::with_default_config())),
        }
    }

    pub fn stt(mut self, stt: Arc<dyn SpeechToText>) -> Self {
        self.stt = Some(stt);
        self
    }

    pub fn tts(mut self, tts: Arc<dyn TextToSpeech>) -> Self {
        self.tts = Some(tts);
        self
    }

    pub fn config(mut self, config: TalkModeConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn intent_recognizer(mut self, recognizer: Arc<dyn IntentRecognizer>) -> Self {
        self.intent_recognizer = Some(recognizer);
        self
    }

    pub fn vad(mut self, vad: Arc<dyn VoiceActivityDetector>) -> Self {
        self.vad = Some(vad);
        self
    }

    pub fn agc(mut self, agc: Arc<dyn AgcProcessor>) -> Self {
        self.agc = Some(agc);
        self
    }

    pub fn noise_suppressor(mut self, suppressor: Arc<dyn NoiseSuppressor>) -> Self {
        self.noise_suppressor = Some(suppressor);
        self
    }

    pub fn build(self) -> Result<VoiceAgent> {
        let stt = self.stt.ok_or_else(|| openclaw_core::OpenClawError::Config("STT not configured".to_string()))?;
        let tts = self.tts.ok_or_else(|| openclaw_core::OpenClawError::Config("TTS not configured".to_string()))?;
        let config = self.config.unwrap_or_default();

        Ok(VoiceAgent {
            stt,
            tts,
            talk_mode: TalkMode::new(config),
            running: Arc::new(RwLock::new(false)),
            dialogue_manager: self.dialogue_manager.unwrap_or_else(|| DialogueContextManager::new(100)),
            current_context: Arc::new(RwLock::new(None)),
            intent_recognizer: self.intent_recognizer.unwrap_or_else(|| Arc::new(KeywordIntentRecognizer::new())),
            vad: self.vad.unwrap_or_else(|| Arc::new(EnergyVad::with_default_config())),
            agc: self.agc.unwrap_or_else(|| Arc::new(SimpleAgc::with_default_config())),
            noise_suppressor: self.noise_suppressor.unwrap_or_else(|| Arc::new(SpectralSubtraction::with_default_config())),
        })
    }
}

impl Default for VoiceAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
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
            dialogue_manager: DialogueContextManager::new(100),
            current_context: Arc::new(RwLock::new(None)),
            intent_recognizer: Arc::new(KeywordIntentRecognizer::new()),
            vad: Arc::new(EnergyVad::with_default_config()),
            agc: Arc::new(SimpleAgc::with_default_config()),
            noise_suppressor: Arc::new(SpectralSubtraction::with_default_config()),
        }
    }

    pub async fn create_conversation(&self, conversation_id: String) -> Result<DialogueContext> {
        let context = self.dialogue_manager.create_context(conversation_id).await;
        let mut current = self.current_context.write().await;
        *current = Some(context.clone());
        Ok(context)
    }

    pub async fn get_current_context(&self) -> Option<DialogueContext> {
        let current = self.current_context.read().await;
        current.clone()
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
        let mut audio: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        self.noise_suppressor.suppress(&mut audio);
        self.agc.process(&mut audio);

        let processed_audio: Vec<u8> = audio
            .iter()
            .flat_map(|&s| s.to_le_bytes())
            .collect();

        let result = self.stt.transcribe(&processed_audio, None).await?;
        let text = result.text.clone();

        if let Some(context) = self.get_current_context().await {
            context.add_user_turn(text.clone()).await;
        }

        self.talk_mode.on_transcription(text.clone()).await?;
        Ok(text)
    }

    pub async fn recognize_intent(&self, text: &str) -> Result<crate::dialogue::IntentRecognitionResult> {
        let context = self.get_current_context()
            .await
            .unwrap_or_else(|| DialogueContext::new("default".to_string()));
        
        self.intent_recognizer.recognize(text, &context).await
    }

    pub fn is_speaking(&self, audio_data: &[u8]) -> bool {
        let audio: Vec<i16> = audio_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        self.vad.is_speaking(&audio)
    }

    pub async fn speak(&self, text: &str) -> Result<Vec<u8>> {
        if let Some(context) = self.get_current_context().await {
            context.add_assistant_turn(text.to_string()).await;
        }

        let audio = self.tts.synthesize(text, None).await?;
        self.talk_mode.on_ai_response(text.to_string()).await?;
        Ok(audio)
    }
}
