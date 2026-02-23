//! Voice Wake - 语音唤醒功能
//!
//! 检测唤醒词，触发语音交互

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

fn zip_err(e: zip::result::ZipError) -> OpenClawError {
    OpenClawError::Io(std::io::Error::other(e.to_string()))
}

/// 唤醒词配置
#[derive(Debug, Clone)]
pub struct WakeWordConfig {
    /// 唤醒词列表
    pub wake_words: Vec<String>,
    /// 检测阈值 (0.0 - 1.0)
    pub threshold: f32,
    /// 最小音频长度 (毫秒)
    pub min_audio_length_ms: u64,
    /// 灵敏度 (0.0 - 1.0)
    pub sensitivity: f32,
}

impl Default for WakeWordConfig {
    fn default() -> Self {
        Self {
            wake_words: vec![
                "hey openclaw".to_string(),
                "你好小爪".to_string(),
                "小爪小爪".to_string(),
            ],
            threshold: 0.5,
            min_audio_length_ms: 500,
            sensitivity: 0.5,
        }
    }
}

impl WakeWordConfig {
    /// 创建自定义配置
    pub fn new(wake_words: Vec<String>) -> Self {
        Self {
            wake_words,
            ..Default::default()
        }
    }

    /// 设置阈值
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 设置灵敏度
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = sensitivity.clamp(0.0, 1.0);
        self
    }
}

/// 唤醒事件
#[derive(Debug, Clone)]
pub struct WakeEvent {
    /// 检测到的唤醒词
    pub wake_word: String,
    /// 置信度
    pub confidence: f32,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Voice Wake Trait
#[async_trait]
pub trait VoiceWake: Send + Sync {
    /// 开始监听
    async fn start(&mut self) -> Result<()>;

    /// 停止监听
    async fn stop(&mut self) -> Result<()>;

    /// 订阅唤醒事件
    fn subscribe(&self) -> broadcast::Receiver<WakeEvent>;

    /// 检查是否运行中
    async fn is_running(&self) -> bool;
}

/// 基于关键词匹配的唤醒检测
pub struct KeywordWakeDetector {
    /// 配置
    config: WakeWordConfig,
    /// 运行状态
    running: Arc<RwLock<bool>>,
    /// 事件发送器
    event_tx: broadcast::Sender<WakeEvent>,
}

impl KeywordWakeDetector {
    pub fn new(config: WakeWordConfig) -> Self {
        let (event_tx, _) = broadcast::channel(16);
        Self {
            config,
            running: Arc::new(RwLock::new(false)),
            event_tx,
        }
    }

    /// 处理音频数据，检测唤醒词
    pub async fn process_audio(&self, _audio_data: &[u8]) -> Result<Option<WakeEvent>> {
        // 注意: 实际实现需要:
        // 1. 将音频转换为文本 (使用本地或云端 STT)
        // 2. 检查是否包含唤醒词
        // 3. 返回匹配结果

        // 这里是框架实现
        Ok(None)
    }

    /// 检查文本是否包含唤醒词
    pub fn check_wake_word(&self, text: &str) -> Option<(String, f32)> {
        let text_lower = text.to_lowercase();

        for wake_word in &self.config.wake_words {
            let wake_word_lower = wake_word.to_lowercase();

            if text_lower.contains(&wake_word_lower) {
                // 计算置信度 (简化实现)
                let confidence = if text_lower.trim() == wake_word_lower {
                    1.0
                } else {
                    0.8
                };

                if confidence >= self.config.threshold {
                    return Some((wake_word.clone(), confidence));
                }
            }
        }

        None
    }
}

#[async_trait]
impl VoiceWake for KeywordWakeDetector {
    async fn start(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        tracing::info!(
            "Voice Wake 已启动，监听唤醒词: {:?}",
            self.config.wake_words
        );

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;

        tracing::info!("Voice Wake 已停止");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<WakeEvent> {
        self.event_tx.subscribe()
    }

    async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Porcupine 唤醒词检测 (需要付费授权)
/// 注意: Porcupine 是 Picovoice 的产品，需要获取 Access Key
pub struct PorcupineWakeDetector {
    access_key: Option<String>,
    config: WakeWordConfig,
    running: Arc<RwLock<bool>>,
    event_tx: broadcast::Sender<WakeEvent>,
}

impl PorcupineWakeDetector {
    pub fn new(config: WakeWordConfig, access_key: Option<String>) -> Self {
        let (event_tx, _) = broadcast::channel(16);
        Self {
            access_key,
            config,
            running: Arc::new(RwLock::new(false)),
            event_tx,
        }
    }

    /// 检查是否可用
    pub fn is_available(&self) -> bool {
        self.access_key.is_some()
    }
}

#[async_trait]
impl VoiceWake for PorcupineWakeDetector {
    async fn start(&mut self) -> Result<()> {
        if self.access_key.is_none() {
            return Err(OpenClawError::Config(
                "Porcupine 需要 Access Key".to_string(),
            ));
        }

        let mut running = self.running.write().await;
        *running = true;

        tracing::info!("Porcupine Wake 已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;

        tracing::info!("Porcupine Wake 已停止");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<WakeEvent> {
        self.event_tx.subscribe()
    }

    async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Vosk 本地唤醒词检测
pub struct VoskWakeDetector {
    model_path: Option<String>,
    config: WakeWordConfig,
    running: Arc<RwLock<bool>>,
    event_tx: broadcast::Sender<WakeEvent>,
}

impl VoskWakeDetector {
    pub fn new(config: WakeWordConfig, model_path: Option<String>) -> Self {
        let (event_tx, _) = broadcast::channel(16);
        Self {
            model_path,
            config,
            running: Arc::new(RwLock::new(false)),
            event_tx,
        }
    }

    /// 下载 Vosk 模型
    pub async fn download_model(model_type: VoskModelType) -> Result<String> {
        let models_dir = Self::get_models_dir()?;

        let (filename, url) = match model_type {
            VoskModelType::SmallEn => (
                "vosk-model-small-en-us-0.15",
                "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip",
            ),
            VoskModelType::SmallZh => (
                "vosk-model-small-cn-0.22",
                "https://alphacephei.com/vosk/models/vosk-model-small-cn-0.22.zip",
            ),
            VoskModelType::Small => (
                "vosk-model-small-en-us-0.15",
                "https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip",
            ),
        };

        let model_path = models_dir.join(filename);

        if model_path.exists() {
            return Ok(model_path.to_string_lossy().to_string());
        }

        println!("📥 下载 Vosk 模型: {}", filename);

        std::fs::create_dir_all(&models_dir)
            .map_err(|e| OpenClawError::Config(format!("创建模型目录失败: {}", e)))?;

        // 下载并解压
        let response = reqwest::get(url)
            .await
            .map_err(|e| OpenClawError::Http(format!("下载模型失败: {}", e)))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| OpenClawError::Http(format!("读取模型数据失败: {}", e)))?;

        let zip_path = model_path.with_extension("zip");

        std::fs::write(&zip_path, &bytes)?;

        println!("📦 解压 Vosk 模型...");

        let file = std::fs::File::open(&zip_path)?;

        let mut archive = zip::ZipArchive::new(file).map_err(zip_err)?;

        let extract_dir = models_dir.join(filename.trim_end_matches(".zip"));

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(zip_err)?;

            let outpath = match file.enclosed_name() {
                Some(path) => extract_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        std::fs::remove_file(&zip_path)?;

        println!("✅ 模型已解压到: {}", extract_dir.display());

        Ok(model_path.to_string_lossy().to_string())
    }

    fn get_models_dir() -> Result<std::path::PathBuf> {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        Ok(std::path::PathBuf::from(home)
            .join(".openclaw-rust")
            .join("vosk-models"))
    }
}

#[async_trait]
impl VoiceWake for VoskWakeDetector {
    async fn start(&mut self) -> Result<()> {
        if self.model_path.is_none() {
            return Err(OpenClawError::Config("Vosk 需要指定模型路径".to_string()));
        }

        let mut running = self.running.write().await;
        *running = true;

        tracing::info!("Vosk Wake 已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;

        tracing::info!("Vosk Wake 已停止");
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<WakeEvent> {
        self.event_tx.subscribe()
    }

    async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Vosk 模型类型
#[derive(Debug, Clone, Copy)]
pub enum VoskModelType {
    /// 英语小模型
    SmallEn,
    /// 中文小模型
    SmallZh,
    /// 自动选择
    Small,
}

/// 创建唤醒检测器
pub fn create_wake_detector(
    detector_type: WakeDetectorType,
    config: WakeWordConfig,
) -> Box<dyn VoiceWake> {
    match detector_type {
        WakeDetectorType::Keyword => Box::new(KeywordWakeDetector::new(config)),
        WakeDetectorType::Porcupine(access_key) => {
            Box::new(PorcupineWakeDetector::new(config, access_key))
        }
        WakeDetectorType::Vosk(model_path) => Box::new(VoskWakeDetector::new(config, model_path)),
    }
}

/// 唤醒检测器类型
#[derive(Debug, Clone)]
pub enum WakeDetectorType {
    /// 关键词匹配 (简单，需要配合 STT)
    Keyword,
    /// Porcupine (高质量，需要授权)
    Porcupine(Option<String>),
    /// Vosk (本地，免费)
    Vosk(Option<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_config() {
        let config = WakeWordConfig::default();
        assert!(!config.wake_words.is_empty());
        assert!(config.threshold > 0.0);
    }

    #[test]
    fn test_check_wake_word() {
        let detector = KeywordWakeDetector::new(WakeWordConfig::default());

        let result = detector.check_wake_word("hey openclaw");
        assert!(result.is_some(), "Should detect wake word 'hey openclaw'");
        if let Some((word, confidence)) = result {
            assert_eq!(word, "hey openclaw");
            assert_eq!(confidence, 1.0);
        }

        let result = detector.check_wake_word("Hey OpenClaw, what time is it?");
        assert!(result.is_some(), "Should detect wake word in sentence");
        if let Some((word, confidence)) = result {
            assert_eq!(word, "hey openclaw");
            assert!(confidence >= 0.8);
        }

        let result = detector.check_wake_word("hello world");
        assert!(
            result.is_none(),
            "Should not detect wake word in 'hello world'"
        );
    }

    #[test]
    fn test_check_wake_word_with_empty_config() {
        let config = WakeWordConfig {
            wake_words: vec![],
            threshold: 0.5,
            min_audio_length_ms: 500,
            sensitivity: 0.5,
        };
        let detector = KeywordWakeDetector::new(config);

        let result = detector.check_wake_word("hey openclaw");
        assert!(result.is_none(), "Should not detect with empty wake words");
    }

    #[test]
    fn test_check_wake_word_threshold() {
        let config = WakeWordConfig {
            wake_words: vec!["hey openclaw".to_string()],
            threshold: 0.9,
            min_audio_length_ms: 500,
            sensitivity: 0.5,
        };
        let detector = KeywordWakeDetector::new(config);

        let result = detector.check_wake_word("hey openclaw");
        assert!(result.is_some(), "Should detect exact match");

        let result = detector.check_wake_word("Hey OpenClaw, what time is it?");
        assert!(
            result.is_none(),
            "Should not detect partial match due to high threshold"
        );
    }
}
