//! VAD (Voice Activity Detection) 模块

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct VadConfig {
    pub frame_size_ms: u32,
    pub sample_rate: u32,
    pub threshold: f32,
    pub min_speech_duration_ms: u32,
    pub min_silence_duration_ms: u32,
    pub speech_pad_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            frame_size_ms: 30,
            sample_rate: 16000,
            threshold: 0.5,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 400,
            speech_pad_ms: 200,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpeechSegment {
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence: f32,
}

pub trait VoiceActivityDetector: Send + Sync {
    fn is_speaking(&self, audio: &[i16]) -> bool;
    fn is_speaking_f32(&self, audio: &[f32]) -> bool;
    fn get_speech_segments(&self, audio: &[i16], total_duration_ms: u64) -> Vec<SpeechSegment>;
    fn reset(&self);
}

pub struct EnergyVad {
    config: VadConfig,
    state: Arc<RwLock<VadState>>,
}

#[derive(Debug, Clone, PartialEq)]
enum VadState {
    Idle,
    Speaking,
    WaitForEnd,
}

impl EnergyVad {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(VadState::Idle)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(VadConfig::default())
    }

    fn calculate_energy(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|&s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    fn samples_to_f32(&self, samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
    }
}

impl VoiceActivityDetector for EnergyVad {
    fn is_speaking(&self, audio: &[i16]) -> bool {
        let samples_f32 = self.samples_to_f32(audio);
        self.is_speaking_f32(&samples_f32)
    }

    fn is_speaking_f32(&self, audio: &[f32]) -> bool {
        let energy = self.calculate_energy(audio);
        energy > self.config.threshold
    }

    fn get_speech_segments(&self, audio: &[i16], total_duration_ms: u64) -> Vec<SpeechSegment> {
        let samples_f32 = self.samples_to_f32(audio);
        let frame_size = (self.config.sample_rate * self.config.frame_size_ms / 1000) as usize;
        
        if samples_f32.len() < frame_size || frame_size == 0 {
            return vec![];
        }

        let mut segments = Vec::new();
        let mut speech_start: Option<u64> = None;
        let mut silence_frames = 0;
        let silence_threshold = (self.config.min_silence_duration_ms / self.config.frame_size_ms) as usize;
        
        let num_frames = samples_f32.len() / frame_size;
        
        for i in 0..num_frames {
            let start = i * frame_size;
            let end = (start + frame_size).min(samples_f32.len());
            let frame = &samples_f32[start..end];
            
            let is_speech = self.is_speaking_f32(frame);
            let timestamp_ms = (i as u64) * (self.config.frame_size_ms as u64);
            
            match (is_speech, &speech_start) {
                (true, None) => {
                    speech_start = Some(timestamp_ms);
                    silence_frames = 0;
                }
                (false, Some(_)) => {
                    silence_frames += 1;
                    if silence_frames >= silence_threshold {
                        if let Some(start) = speech_start.take() {
                            let end_ms = timestamp_ms - ((silence_threshold as u64) * (self.config.frame_size_ms as u64) / 2);
                            if end_ms > start {
                                segments.push(SpeechSegment {
                                    start_ms: start.saturating_sub(self.config.speech_pad_ms as u64),
                                    end_ms,
                                    confidence: 0.8,
                                });
                            }
                        }
                        silence_frames = 0;
                    }
                }
                _ => {}
            }
        }
        
        if let Some(start) = speech_start {
            segments.push(SpeechSegment {
                start_ms: start.saturating_sub(self.config.speech_pad_ms as u64),
                end_ms: total_duration_ms,
                confidence: 0.8,
            });
        }
        
        segments
    }

    fn reset(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut s = state.write().await;
            *s = VadState::Idle;
        });
    }
}

pub struct VadBuilder {
    config: VadConfig,
}

impl VadBuilder {
    pub fn new() -> Self {
        Self {
            config: VadConfig::default(),
        }
    }

    pub fn frame_size_ms(mut self, size: u32) -> Self {
        self.config.frame_size_ms = size;
        self
    }

    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.config.sample_rate = rate;
        self
    }

    pub fn threshold(mut self, threshold: f32) -> Self {
        self.config.threshold = threshold;
        self
    }

    pub fn min_speech_duration_ms(mut self, duration: u32) -> Self {
        self.config.min_speech_duration_ms = duration;
        self
    }

    pub fn min_silence_duration_ms(mut self, duration: u32) -> Self {
        self.config.min_silence_duration_ms = duration;
        self
    }

    pub fn build(self) -> EnergyVad {
        EnergyVad::new(self.config)
    }
}

impl Default for VadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_config_default() {
        let config = VadConfig::default();
        assert_eq!(config.frame_size_ms, 30);
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.threshold, 0.5);
    }

    #[test]
    fn test_energy_vad_new() {
        let vad = EnergyVad::new(VadConfig::default());
        assert_eq!(vad.config.threshold, 0.5);
    }

    #[test]
    fn test_energy_vad_with_default_config() {
        let vad = EnergyVad::with_default_config();
        assert_eq!(vad.config.threshold, 0.5);
    }

    #[test]
    fn test_vad_builder() {
        let vad = VadBuilder::new()
            .frame_size_ms(20)
            .sample_rate(48000)
            .threshold(0.6)
            .build();
        
        assert_eq!(vad.config.frame_size_ms, 20);
        assert_eq!(vad.config.sample_rate, 48000);
        assert_eq!(vad.config.threshold, 0.6);
    }

    #[test]
    fn test_is_speaking_with_silence() {
        let vad = EnergyVad::with_default_config();
        let silence: Vec<i16> = vec![0; 1600];
        assert!(!vad.is_speaking(&silence));
    }

    #[test]
    fn test_is_speaking_with_speech() {
        let vad = EnergyVad::with_default_config();
        let speech: Vec<i16> = vec![i16::MAX; 1600];
        assert!(vad.is_speaking(&speech));
    }

    #[test]
    fn test_get_speech_segments_empty() {
        let vad = EnergyVad::with_default_config();
        let audio: Vec<i16> = vec![0; 1600];
        let segments = vad.get_speech_segments(&audio, 100);
        assert!(segments.is_empty());
    }

    #[test]
    fn test_get_speech_segments_with_speech() {
        let vad = EnergyVad::with_default_config();
        let mut audio: Vec<i16> = vec![0; 16000];
        for i in 4000..8000 {
            audio[i] = i16::MAX;
        }
        let segments = vad.get_speech_segments(&audio, 1000);
        assert!(!segments.is_empty());
    }
}
