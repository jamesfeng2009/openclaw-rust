//! 流式音频处理模块

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone)]
pub struct AudioChunk {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u16,
    pub timestamp_ms: u64,
    pub is_speech: bool,
}

impl AudioChunk {
    pub fn new(data: Vec<u8>, sample_rate: u32, channels: u16, timestamp_ms: u64) -> Self {
        Self {
            data,
            sample_rate,
            channels,
            timestamp_ms,
            is_speech: false,
        }
    }

    pub fn to_samples_i16(&self) -> Vec<i16> {
        self.data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    pub fn to_samples_f32(&self) -> Vec<f32> {
        self.to_samples_i16()
            .iter()
            .map(|&s| s as f32 / i16::MAX as f32)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    SpeechStart { timestamp_ms: u64 },
    SpeechChunk { chunk: AudioChunk },
    SpeechEnd { final_text: String, timestamp_ms: u64 },
    Silence { duration_ms: u64 },
    Noise { level: f32 },
    Error { message: String },
}

pub trait AudioStreamProcessor: Send + Sync {
    fn process_chunk(&self, chunk: AudioChunk) -> StreamEvent;
    fn reset(&self);
}

pub struct StreamProcessorConfig {
    pub vad_threshold: f32,
    pub silence_threshold: f32,
    pub min_speech_duration_ms: u64,
    pub max_silence_duration_ms: u64,
}

impl Default for StreamProcessorConfig {
    fn default() -> Self {
        Self {
            vad_threshold: 0.5,
            silence_threshold: 0.02,
            min_speech_duration_ms: 100,
            max_silence_duration_ms: 1500,
        }
    }
}

pub struct DefaultStreamProcessor {
    config: StreamProcessorConfig,
    state: Arc<RwLock<ProcessorState>>,
    event_tx: broadcast::Sender<StreamEvent>,
}

#[derive(Debug, Clone, PartialEq)]
enum ProcessorState {
    Idle,
    Listening,
    Speaking,
    WaitingForSpeech,
}

impl DefaultStreamProcessor {
    pub fn new(config: StreamProcessorConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            config,
            state: Arc::new(RwLock::new(ProcessorState::Listening)),
            event_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.event_tx.subscribe()
    }

    pub async fn get_state(&self) -> ProcessorState {
        self.state.read().await.clone()
    }

    fn calculate_energy(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|&s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }
}

impl AudioStreamProcessor for DefaultStreamProcessor {
    fn process_chunk(&self, chunk: AudioChunk) -> StreamEvent {
        let samples = chunk.to_samples_f32();
        let energy = self.calculate_energy(&samples);

        if energy > self.config.vad_threshold {
            let _ = self.event_tx.send(StreamEvent::SpeechChunk { chunk: chunk.clone() });
            StreamEvent::SpeechChunk { chunk }
        } else if energy > self.config.silence_threshold {
            let _ = self.event_tx.send(StreamEvent::Noise { level: energy });
            StreamEvent::Noise { level: energy }
        } else {
            let _ = self.event_tx.send(StreamEvent::Silence { duration_ms: 0 });
            StreamEvent::Silence { duration_ms: 0 }
        }
    }

    fn reset(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut s = state.write().await;
            *s = ProcessorState::Listening;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_chunk_creation() {
        let data = vec![0u8; 1024];
        let chunk = AudioChunk::new(data.clone(), 16000, 1, 0);
        assert_eq!(chunk.sample_rate, 16000);
        assert_eq!(chunk.channels, 1);
    }

    #[test]
    fn test_audio_chunk_to_samples() {
        let data = vec![0u8, 0, 1, 0, 2, 0];
        let chunk = AudioChunk::new(data, 16000, 1, 0);
        let samples = chunk.to_samples_i16();
        assert_eq!(samples.len(), 3);
    }

    #[test]
    fn test_stream_processor_config_default() {
        let config = StreamProcessorConfig::default();
        assert_eq!(config.vad_threshold, 0.5);
        assert_eq!(config.silence_threshold, 0.02);
    }

    #[tokio::test]
    async fn test_default_stream_processor_new() {
        let processor = DefaultStreamProcessor::new(StreamProcessorConfig::default());
        assert_eq!(processor.get_state().await, ProcessorState::Listening);
    }
}
