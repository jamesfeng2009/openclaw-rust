//! 音频处理工具

use openclaw_core::{OpenClawError, Result};

/// 音频格式信息
#[derive(Debug, Clone)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
    pub duration_seconds: f64,
}

/// 音频录制器
pub struct AudioRecorder {
    sample_rate: u32,
    channels: u16,
}

impl AudioRecorder {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// 开始录音
    pub async fn start_recording(&self) -> Result<AudioRecording> {
        // TODO: 实现实际的音频录制
        // 需要使用 cpal 库进行跨平台音频输入
        Err(OpenClawError::Config(
            "音频录制功能开发中".to_string(),
        ))
    }
}

/// 音频录制会话
pub struct AudioRecording {
    sample_rate: u32,
    channels: u16,
    data: Vec<u8>,
}

impl AudioRecording {
    /// 停止录音并获取数据
    pub async fn stop(self) -> Result<Vec<u8>> {
        Ok(self.data)
    }

    /// 获取当前录音时长（秒）
    pub fn duration(&self) -> f64 {
        let samples = self.data.len() as f64 / (self.channels as f64 * 2.0); // 16-bit = 2 bytes
        samples / self.sample_rate as f64
    }
}

/// 音频播放器
pub struct AudioPlayer {
    sample_rate: u32,
    channels: u16,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            sample_rate: 24000,
            channels: 1,
        }
    }

    /// 播放音频数据
    pub async fn play(&self, audio_data: &[u8]) -> Result<()> {
        // TODO: 实现实际的音频播放
        // 需要使用 cpal 库进行跨平台音频输出
        Err(OpenClawError::Config(
            "音频播放功能开发中".to_string(),
        ))
    }

    /// 播放音频文件
    pub async fn play_file(&self, file_path: &std::path::Path) -> Result<()> {
        let data = std::fs::read(file_path)
            .map_err(|e| OpenClawError::Config(format!("读取音频文件失败: {}", e)))?;
        self.play(&data).await
    }

    /// 停止播放
    pub async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

/// 音频工具函数
pub struct AudioUtils;

impl AudioUtils {
    /// 计算 RMS 音量
    pub fn calculate_rms(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        (sum / samples.len() as f64).sqrt() as f32 / i16::MAX as f32
    }

    /// 检测是否静音
    pub fn is_silence(samples: &[i16], threshold: f32) -> bool {
        Self::calculate_rms(samples) < threshold
    }

    /// 重采样音频
    pub fn resample(
        input: &[i16],
        input_rate: u32,
        output_rate: u32,
        channels: u16,
    ) -> Result<Vec<i16>> {
        if input_rate == output_rate {
            return Ok(input.to_vec());
        }

        // 使用 rubato 进行高质量重采样
        let ratio = output_rate as f64 / input_rate as f64;
        let output_len = (input.len() as f64 * ratio) as usize;

        let mut output = Vec::with_capacity(output_len);
        for i in 0..output_len {
            let src_idx = (i as f64 / ratio) as usize;
            output.push(input.get(src_idx).copied().unwrap_or(0));
        }

        Ok(output)
    }

    /// 将音频数据转换为 WAV 格式
    pub fn to_wav(
        samples: &[i16],
        sample_rate: u32,
        channels: u16,
    ) -> Result<Vec<u8>> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .map_err(|e| OpenClawError::Config(format!("创建 WAV 写入器失败: {}", e)))?;

            for &sample in samples {
                writer
                    .write_sample(sample)
                    .map_err(|e| OpenClawError::Config(format!("写入 WAV 数据失败: {}", e)))?;
            }

            writer
                .finalize()
                .map_err(|e| OpenClawError::Config(format!("完成 WAV 文件失败: {}", e)))?;
        }

        Ok(cursor.into_inner())
    }

    /// 从 WAV 数据读取音频
    pub fn from_wav(wav_data: &[u8]) -> Result<(Vec<i16>, AudioInfo)> {
        let cursor = std::io::Cursor::new(wav_data);
        let reader = hound::WavReader::new(cursor)
            .map_err(|e| OpenClawError::Config(format!("读取 WAV 文件失败: {}", e)))?;

        let spec = reader.spec();
        let samples: Vec<i16> = reader
            .into_samples()
            .filter_map(|s| s.ok())
            .collect();

        let duration_seconds = samples.len() as f64 / (spec.sample_rate as f64 * spec.channels as f64);

        Ok((
            samples,
            AudioInfo {
                sample_rate: spec.sample_rate,
                channels: spec.channels,
                bits_per_sample: spec.bits_per_sample,
                duration_seconds,
            },
        ))
    }

    /// 拼接音频数据
    pub fn concat(audio_chunks: &[Vec<i16>]) -> Vec<i16> {
        let total_len: usize = audio_chunks.iter().map(|c| c.len()).sum();
        let mut result = Vec::with_capacity(total_len);

        for chunk in audio_chunks {
            result.extend(chunk);
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms() {
        let samples = vec![1000, -1000, 1000, -1000];
        let rms = AudioUtils::calculate_rms(&samples);
        assert!(rms > 0.0 && rms < 1.0);
    }

    #[test]
    fn test_is_silence() {
        let silence = vec![0, 0, 0, 0];
        assert!(AudioUtils::is_silence(&silence, 0.01));

        let loud = vec![10000, -10000, 10000, -10000];
        assert!(!AudioUtils::is_silence(&loud, 0.01));
    }

    #[test]
    fn test_to_wav() {
        let samples = vec![1000, -1000, 1000, -1000];
        let wav = AudioUtils::to_wav(&samples, 16000, 1).unwrap();
        assert!(!wav.is_empty());

        // 验证 WAV 头
        assert_eq!(&wav[0..4], b"RIFF");
    }
}
