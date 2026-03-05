//! 噪声抑制模块

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct NoiseSuppressionConfig {
    pub noise_threshold: f32,
    pub smoothing_factor: f32,
    pub sample_rate: u32,
}

impl Default for NoiseSuppressionConfig {
    fn default() -> Self {
        Self {
            noise_threshold: 0.05,
            smoothing_factor: 0.9,
            sample_rate: 16000,
        }
    }
}

pub trait NoiseSuppressor: Send + Sync {
    fn suppress(&self, audio: &mut [i16]);
    fn suppress_f32(&self, audio: &mut [f32]);
    fn update_noise_profile(&self, audio: &[i16]);
    fn reset(&self);
}

pub struct SpectralSubtraction {
    config: NoiseSuppressionConfig,
    noise_profile: Arc<RwLock<NoiseProfile>>,
}

#[derive(Debug, Clone, Default)]
pub struct NoiseProfile {
    pub noise_level: f32,
    pub spectrum: Vec<f32>,
}

impl SpectralSubtraction {
    pub fn new(config: NoiseSuppressionConfig) -> Self {
        Self {
            config,
            noise_profile: Arc::new(RwLock::new(NoiseProfile::default())),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(NoiseSuppressionConfig::default())
    }

    fn calculate_energy(&self, audio: &[f32]) -> f32 {
        if audio.is_empty() {
            return 0.0;
        }
        let sum: f32 = audio.iter().map(|&s| s * s).sum();
        (sum / audio.len() as f32).sqrt()
    }

    fn samples_to_f32(&self, samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
    }

    fn f32_to_i16(&self, samples: &[f32]) -> Vec<i16> {
        samples.iter().map(|&s| (s * i16::MAX as f32) as i16).collect()
    }
}

impl NoiseSuppressor for SpectralSubtraction {
    fn suppress(&self, audio: &mut [i16]) {
        let samples_f32: Vec<f32> = audio.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let mut processed = samples_f32.clone();
        self.suppress_f32(&mut processed);
        
        for (i, sample) in processed.iter().enumerate() {
            if i < audio.len() {
                audio[i] = (*sample * i16::MAX as f32) as i16;
            }
        }
    }

    fn suppress_f32(&self, audio: &mut [f32]) {
        let energy = self.calculate_energy(audio);
        
        if energy < self.config.noise_threshold {
            for sample in audio.iter_mut() {
                *sample *= self.config.smoothing_factor;
            }
        }
    }

    fn update_noise_profile(&self, audio: &[i16]) {
        let samples_f32 = self.samples_to_f32(audio);
        let energy = self.calculate_energy(&samples_f32);
        
        let noise_profile = self.noise_profile.clone();
        tokio::spawn(async move {
            let mut profile = noise_profile.write().await;
            profile.noise_level = energy;
        });
    }

    fn reset(&self) {
        let noise_profile = self.noise_profile.clone();
        tokio::spawn(async move {
            let mut profile = noise_profile.write().await;
            *profile = NoiseProfile::default();
        });
    }
}

pub struct GateNoiseSuppressor {
    config: NoiseSuppressionConfig,
    is_open: Arc<RwLock<bool>>,
}

impl GateNoiseSuppressor {
    pub fn new(config: NoiseSuppressionConfig) -> Self {
        Self {
            config,
            is_open: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(NoiseSuppressionConfig::default())
    }

    fn calculate_energy(&self, audio: &[f32]) -> f32 {
        if audio.is_empty() {
            return 0.0;
        }
        let sum: f32 = audio.iter().map(|&s| s * s).sum();
        (sum / audio.len() as f32).sqrt()
    }

    fn samples_to_f32(&self, samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / i16::MAX as f32).collect()
    }

    fn f32_to_i16(&self, samples: &[f32]) -> Vec<i16> {
        samples.iter().map(|&s| (s * i16::MAX as f32) as i16).collect()
    }
}

impl NoiseSuppressor for GateNoiseSuppressor {
    fn suppress(&self, audio: &mut [i16]) {
        let samples_f32: Vec<f32> = audio.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let mut processed = samples_f32.clone();
        self.suppress_f32(&mut processed);
        
        for (i, sample) in processed.iter().enumerate() {
            if i < audio.len() {
                audio[i] = (*sample * i16::MAX as f32) as i16;
            }
        }
    }

    fn suppress_f32(&self, audio: &mut [f32]) {
        let energy = self.calculate_energy(audio);
        let is_open = energy > self.config.noise_threshold;
        
        {
            if let Ok(mut open) = self.is_open.try_write() {
                *open = is_open;
            }
        }

        if !is_open {
            for sample in audio.iter_mut() {
                *sample *= 0.01;
            }
        }
    }

    fn update_noise_profile(&self, _audio: &[i16]) {
    }

    fn reset(&self) {
        let is_open = self.is_open.clone();
        tokio::spawn(async move {
            let mut open = is_open.write().await;
            *open = false;
        });
    }
}

pub struct NoiseSuppressionBuilder {
    config: NoiseSuppressionConfig,
    suppressor_type: SuppressorType,
}

#[derive(Debug, Clone)]
pub enum SuppressorType {
    SpectralSubtraction,
    Gate,
}

impl NoiseSuppressionBuilder {
    pub fn new() -> Self {
        Self {
            config: NoiseSuppressionConfig::default(),
            suppressor_type: SuppressorType::SpectralSubtraction,
        }
    }

    pub fn noise_threshold(mut self, threshold: f32) -> Self {
        self.config.noise_threshold = threshold;
        self
    }

    pub fn smoothing_factor(mut self, factor: f32) -> Self {
        self.config.smoothing_factor = factor;
        self
    }

    pub fn sample_rate(mut self, rate: u32) -> Self {
        self.config.sample_rate = rate;
        self
    }

    pub fn suppressor_type(mut self, suppressor_type: SuppressorType) -> Self {
        self.suppressor_type = suppressor_type;
        self
    }

    pub fn build(self) -> Box<dyn NoiseSuppressor> {
        match self.suppressor_type {
            SuppressorType::SpectralSubtraction => Box::new(SpectralSubtraction::new(self.config)),
            SuppressorType::Gate => Box::new(GateNoiseSuppressor::new(self.config)),
        }
    }
}

impl Default for NoiseSuppressionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_suppression_config_default() {
        let config = NoiseSuppressionConfig::default();
        assert_eq!(config.noise_threshold, 0.05);
        assert_eq!(config.smoothing_factor, 0.9);
    }

    #[test]
    fn test_spectral_subtraction_new() {
        let suppressor = SpectralSubtraction::new(NoiseSuppressionConfig::default());
        assert!(suppressor.noise_profile.try_read().is_ok());
    }

    #[test]
    fn test_spectral_subtraction_with_default_config() {
        let suppressor = SpectralSubtraction::with_default_config();
        assert!(suppressor.noise_profile.try_read().is_ok());
    }

    #[test]
    fn test_gate_noise_suppressor_new() {
        let suppressor = GateNoiseSuppressor::new(NoiseSuppressionConfig::default());
        assert!(suppressor.is_open.try_read().is_ok());
    }

    #[test]
    fn test_suppress_i16() {
        let suppressor = SpectralSubtraction::with_default_config();
        let mut audio = vec![100, 200, 300, 400, 500];
        suppressor.suppress(&mut audio);
        assert_eq!(audio.len(), 5);
    }

    #[test]
    fn test_suppress_f32() {
        let suppressor = SpectralSubtraction::with_default_config();
        let mut audio = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        suppressor.suppress_f32(&mut audio);
        assert_eq!(audio.len(), 5);
    }

    #[test]
    fn test_noise_suppression_builder() {
        let suppressor = NoiseSuppressionBuilder::new()
            .noise_threshold(0.1)
            .smoothing_factor(0.8)
            .build();
        
        assert!(suppressor.suppress_f32_test().is_some());
    }
}

trait NoiseSuppressorTest {
    fn suppress_f32_test(&self) -> Option<()>;
}

impl NoiseSuppressorTest for Box<dyn NoiseSuppressor> {
    fn suppress_f32_test(&self) -> Option<()> {
        let mut audio = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        self.suppress_f32(&mut audio);
        Some(())
    }
}
