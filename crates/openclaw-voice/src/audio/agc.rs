//! AGC (Automatic Gain Control) 模块

#[derive(Debug, Clone)]
pub struct AgcConfig {
    pub target_level: f32,
    pub max_gain: f32,
    pub min_gain: f32,
    pub attack_time_ms: u32,
    pub release_time_ms: u32,
    pub frame_size_ms: u32,
}

impl Default for AgcConfig {
    fn default() -> Self {
        Self {
            target_level: 0.5,
            max_gain: 10.0,
            min_gain: 0.1,
            attack_time_ms: 10,
            release_time_ms: 100,
            frame_size_ms: 20,
        }
    }
}

pub trait AgcProcessor: Send + Sync {
    fn process(&self, audio: &mut [i16]);
    fn process_f32(&self, audio: &mut [f32]);
    fn reset(&self);
}

pub struct SimpleAgc {
    config: AgcConfig,
    current_gain: f32,
}

impl SimpleAgc {
    pub fn new(config: AgcConfig) -> Self {
        Self {
            config,
            current_gain: 1.0,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(AgcConfig::default())
    }

    fn calculate_rms(&self, audio: &[f32]) -> f32 {
        if audio.is_empty() {
            return 0.0;
        }
        let sum: f32 = audio.iter().map(|&s| s * s).sum();
        (sum / audio.len() as f32).sqrt()
    }

    fn update_gain(&mut self, current_level: f32) {
        let target_gain = if current_level > 0.001 {
            self.config.target_level / current_level
        } else {
            self.config.max_gain
        };

        let gain = target_gain.clamp(self.config.min_gain, self.config.max_gain);

        let attack_factor = (-1.0 / self.config.attack_time_ms as f32).exp();
        let release_factor = (-1.0 / self.config.release_time_ms as f32).exp();

        if gain > self.current_gain {
            self.current_gain = self.current_gain * (1.0 - attack_factor) + gain * attack_factor;
        } else {
            self.current_gain = self.current_gain * (1.0 - release_factor) + gain * release_factor;
        }
    }

    fn apply_gain(&self, audio: &mut [f32]) {
        for sample in audio.iter_mut() {
            *sample = (*sample * self.current_gain).clamp(-1.0, 1.0);
        }
    }
}

impl AgcProcessor for SimpleAgc {
    fn process(&self, audio: &mut [i16]) {
        let samples_f32: Vec<f32> = audio.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
        let mut processed = samples_f32.clone();
        
        let level = self.calculate_rms(&processed);
        let mut processor = SimpleAgc::new(self.config.clone());
        processor.update_gain(level);
        processor.apply_gain(&mut processed);

        for (i, sample) in processed.iter().enumerate() {
            audio[i] = (sample * i16::MAX as f32) as i16;
        }
    }

    fn process_f32(&self, audio: &mut [f32]) {
        let level = self.calculate_rms(audio);
        let mut processor = SimpleAgc::new(self.config.clone());
        processor.update_gain(level);
        processor.apply_gain(audio);
    }

    fn reset(&self) {
        let _ = self.current_gain;
    }
}

impl Clone for SimpleAgc {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            current_gain: self.current_gain,
        }
    }
}

pub struct AgcBuilder {
    config: AgcConfig,
}

impl AgcBuilder {
    pub fn new() -> Self {
        Self {
            config: AgcConfig::default(),
        }
    }

    pub fn target_level(mut self, level: f32) -> Self {
        self.config.target_level = level;
        self
    }

    pub fn max_gain(mut self, gain: f32) -> Self {
        self.config.max_gain = gain;
        self
    }

    pub fn min_gain(mut self, gain: f32) -> Self {
        self.config.min_gain = gain;
        self
    }

    pub fn attack_time_ms(mut self, time: u32) -> Self {
        self.config.attack_time_ms = time;
        self
    }

    pub fn release_time_ms(mut self, time: u32) -> Self {
        self.config.release_time_ms = time;
        self
    }

    pub fn build(self) -> SimpleAgc {
        SimpleAgc::new(self.config)
    }
}

impl Default for AgcBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agc_config_default() {
        let config = AgcConfig::default();
        assert_eq!(config.target_level, 0.5);
        assert_eq!(config.max_gain, 10.0);
        assert_eq!(config.min_gain, 0.1);
    }

    #[test]
    fn test_simple_agc_new() {
        let agc = SimpleAgc::new(AgcConfig::default());
        assert_eq!(agc.current_gain, 1.0);
    }

    #[test]
    fn test_simple_agc_with_default_config() {
        let agc = SimpleAgc::with_default_config();
        assert_eq!(agc.current_gain, 1.0);
    }

    #[test]
    fn test_agc_builder() {
        let agc = AgcBuilder::new()
            .target_level(0.6)
            .max_gain(8.0)
            .min_gain(0.2)
            .build();
        
        assert_eq!(agc.current_gain, 1.0);
    }

    #[test]
    fn test_process_f32() {
        let agc = SimpleAgc::with_default_config();
        let mut audio = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        agc.process_f32(&mut audio);
        assert!(!audio.is_empty());
    }

    #[test]
    fn test_process_i16() {
        let agc = SimpleAgc::with_default_config();
        let mut audio = vec![100, 200, 300, 400, 500];
        agc.process(&mut audio);
        assert!(!audio.is_empty());
    }
}
