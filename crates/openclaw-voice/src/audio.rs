//! 音频处理工具

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use openclaw_core::{OpenClawError, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

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
    pub fn start_recording(&self) -> Result<AudioRecording> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| OpenClawError::Config("找不到音频输入设备".to_string()))?;

        let config = device
            .default_input_config()
            .map_err(|e| OpenClawError::Config(format!("获取音频配置失败: {}", e)))?;

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();
        
        let is_recording = Arc::new(AtomicBool::new(true));
        let recorded_data: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        
        let is_recording_clone = is_recording.clone();
        let recorded_data_clone = recorded_data.clone();

        let err_fn = |err| eprintln!("音频录制错误: {}", err);

        let stream_config: cpal::StreamConfig = config.clone().into();
        
        let stream = match config.sample_format() {
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            let bytes: Vec<u8> = data
                                .iter()
                                .flat_map(|&sample| sample.to_le_bytes())
                                .collect();
                            recorded_data_clone.lock().unwrap().extend(bytes);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if is_recording_clone.load(Ordering::SeqCst) {
                            let samples_i16: Vec<i16> = data
                                .iter()
                                .map(|&s| (s * i16::MAX as f32) as i16)
                                .collect();
                            let bytes: Vec<u8> = samples_i16
                                .iter()
                                .flat_map(|&sample| sample.to_le_bytes())
                                .collect();
                            recorded_data_clone.lock().unwrap().extend(bytes);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                return Err(OpenClawError::Config(
                    "不支持的音频格式".to_string(),
                ));
            }
        }
        .map_err(|e| OpenClawError::Config(format!("创建录音流失败: {}", e)))?;

        stream
            .play()
            .map_err(|e| OpenClawError::Config(format!("开始录音失败: {}", e)))?;

        Ok(AudioRecording {
            sample_rate,
            channels,
            stream: Some(stream),
            is_recording,
            recorded_data,
        })
    }
}

/// 音频录制会话
pub struct AudioRecording {
    sample_rate: u32,
    channels: u16,
    stream: Option<cpal::Stream>,
    is_recording: Arc<AtomicBool>,
    recorded_data: Arc<Mutex<Vec<u8>>>,
}

impl AudioRecording {
    /// 停止录音并获取数据
    pub fn stop(self) -> Result<Vec<u8>> {
        self.is_recording.store(false, Ordering::SeqCst);
        
        if let Some(stream) = self.stream {
            drop(stream);
        }
        
        let data = self.recorded_data.lock().unwrap().clone();
        Ok(data)
    }

    /// 获取当前录音时长（秒）
    pub fn duration(&self) -> f64 {
        let bytes = self.recorded_data.lock().unwrap().len() as f64;
        let bytes_per_sample = self.channels as f64 * 2.0;
        bytes / bytes_per_sample / self.sample_rate as f64
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
    pub fn play(&self, audio_data: &[u8]) -> Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| OpenClawError::Config("找不到音频输出设备".to_string()))?;

        let config = device
            .default_output_config()
            .map_err(|e| OpenClawError::Config(format!("获取音频配置失败: {}", e)))?;

        let err_fn = |err| eprintln!("音频播放错误: {}", err);

        let audio_buffer = audio_data.to_vec();
        let audio_len = audio_buffer.len();
        let stream_config: cpal::StreamConfig = config.clone().into();
        let channels = config.channels() as usize;

        let duration_secs = (audio_len as f64 / (self.channels as f64 * 2.0)) / self.sample_rate as f64;

        let stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [u8], _: &cpal::OutputCallbackInfo| {
                    let bytes_per_sample = 2;
                    let bytes_per_frame = channels * bytes_per_sample;
                    
                    for (i, chunk) in data.chunks_mut(bytes_per_frame).enumerate() {
                        let sample_idx = i * bytes_per_frame;
                        if sample_idx < audio_len {
                            let remaining = audio_len - sample_idx;
                            let copy_len = std::cmp::min(chunk.len(), remaining);
                            chunk[..copy_len].copy_from_slice(&audio_buffer[sample_idx..sample_idx + copy_len]);
                        } else {
                            for byte in chunk {
                                *byte = 0;
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| OpenClawError::Config(format!("创建播放流失败: {}", e)))?;

        stream
            .play()
            .map_err(|e| OpenClawError::Config(format!("开始播放失败: {}", e)))?;

        std::thread::sleep(std::time::Duration::from_secs_f64(duration_secs + 0.1));

        Ok(())
    }

    /// 播放音频文件
    pub fn play_file(&self, file_path: &std::path::Path) -> Result<()> {
        let data = std::fs::read(file_path)
            .map_err(|e| OpenClawError::Config(format!("读取音频文件失败: {}", e)))?;
        self.play(&data)
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
    /// 获取默认输入设备信息
    pub fn get_input_device_info() -> Result<(String, AudioInfo)> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| OpenClawError::Config("找不到音频输入设备".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let config = device
            .default_input_config()
            .map_err(|e| OpenClawError::Config(format!("获取音频配置失败: {}", e)))?;

        let info = AudioInfo {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
            bits_per_sample: config.sample_format().sample_size() as u16 * 8,
            duration_seconds: 0.0,
        };

        Ok((name, info))
    }

    /// 获取默认输出设备信息
    pub fn get_output_device_info() -> Result<(String, AudioInfo)> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| OpenClawError::Config("找不到音频输出设备".to_string()))?;

        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let config = device
            .default_output_config()
            .map_err(|e| OpenClawError::Config(format!("获取音频配置失败: {}", e)))?;

        let info = AudioInfo {
            sample_rate: config.sample_rate().0,
            channels: config.channels(),
            bits_per_sample: config.sample_format().sample_size() as u16 * 8,
            duration_seconds: 0.0,
        };

        Ok((name, info))
    }

    /// 列出所有输入设备
    pub fn list_input_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host
            .input_devices()
            .map_err(|e| OpenClawError::Config(format!("获取设备列表失败: {}", e)))?;

        let mut result = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                result.push(name);
            }
        }
        Ok(result)
    }

    /// 列出所有输出设备
    pub fn list_output_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| OpenClawError::Config(format!("获取设备列表失败: {}", e)))?;

        let mut result = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                result.push(name);
            }
        }
        Ok(result)
    }
}
