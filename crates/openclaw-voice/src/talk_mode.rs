//! Talk Mode - 持续对话模式

use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use crate::types::TalkModeState;
use openclaw_core::Result;

/// Talk Mode 事件
#[derive(Debug, Clone)]
pub enum TalkModeEvent {
    /// 开始监听
    ListeningStarted,
    /// 检测到语音
    VoiceDetected,
    /// 语音结束
    VoiceEnded,
    /// 转录结果
    Transcription(String),
    /// AI 回复
    AiResponse(String),
    /// 开始播放语音
    SpeakingStarted,
    /// 播放结束
    SpeakingEnded,
    /// 错误
    Error(String),
    /// 状态改变
    StateChanged(TalkModeState),
}

/// Talk Mode 配置
#[derive(Debug, Clone)]
pub struct TalkModeConfig {
    /// 静音检测阈值
    pub silence_threshold: f32,
    /// 静音超时（毫秒）
    pub silence_timeout_ms: u64,
    /// 最大录音时长（秒）
    pub max_recording_seconds: u64,
    /// 是否自动开始下一次监听
    pub auto_continue: bool,
    /// 回复后等待时间（毫秒）
    pub response_delay_ms: u64,
}

impl Default for TalkModeConfig {
    fn default() -> Self {
        Self {
            silence_threshold: 0.02,
            silence_timeout_ms: 1500,
            max_recording_seconds: 60,
            auto_continue: true,
            response_delay_ms: 500,
        }
    }
}

/// Talk Mode 管理器
pub struct TalkMode {
    /// 当前状态
    state: Arc<RwLock<TalkModeState>>,
    /// 配置
    config: TalkModeConfig,
    /// 事件发送器
    event_tx: broadcast::Sender<TalkModeEvent>,
    /// 是否运行中
    running: Arc<RwLock<bool>>,
}

impl TalkMode {
    /// 创建新的 Talk Mode
    pub fn new(config: TalkModeConfig) -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            state: Arc::new(RwLock::new(TalkModeState::Idle)),
            config,
            event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 获取当前状态
    pub async fn state(&self) -> TalkModeState {
        self.state.read().await.clone()
    }

    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<TalkModeEvent> {
        self.event_tx.subscribe()
    }

    /// 启动 Talk Mode
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        self.set_state(TalkModeState::Listening).await?;
        self.emit_event(TalkModeEvent::ListeningStarted).await;

        tracing::info!("Talk Mode 已启动");
        Ok(())
    }

    /// 停止 Talk Mode
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        drop(running);

        self.set_state(TalkModeState::Idle).await?;

        tracing::info!("Talk Mode 已停止");
        Ok(())
    }

    /// 是否运行中
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// 处理转录结果
    pub async fn on_transcription(&self, text: String) -> Result<()> {
        self.set_state(TalkModeState::Processing).await;
        self.emit_event(TalkModeEvent::Transcription(text.clone()))
            .await;
        Ok(())
    }

    /// 处理 AI 响应
    pub async fn on_ai_response(&self, text: String) -> Result<()> {
        self.set_state(TalkModeState::Speaking).await;
        self.emit_event(TalkModeEvent::AiResponse(text)).await;
        Ok(())
    }

    /// 播放完成
    pub async fn on_speaking_ended(&self) -> Result<()> {
        self.emit_event(TalkModeEvent::SpeakingEnded).await;

        // 自动继续监听
        if self.config.auto_continue && self.is_running().await {
            self.set_state(TalkModeState::Listening).await;
            self.emit_event(TalkModeEvent::ListeningStarted).await;
        } else {
            self.set_state(TalkModeState::Idle).await;
        }

        Ok(())
    }

    /// 设置状态
    async fn set_state(&self, new_state: TalkModeState) -> Result<()> {
        let mut state = self.state.write().await;
        *state = new_state.clone();
        drop(state);

        self.emit_event(TalkModeEvent::StateChanged(new_state))
            .await;
        Ok(())
    }

    /// 发送事件
    async fn emit_event(&self, event: TalkModeEvent) {
        let _ = self.event_tx.send(event);
    }
}

/// Talk Mode 构建器
pub struct TalkModeBuilder {
    config: TalkModeConfig,
}

impl TalkModeBuilder {
    pub fn new() -> Self {
        Self {
            config: TalkModeConfig::default(),
        }
    }

    pub fn silence_threshold(mut self, threshold: f32) -> Self {
        self.config.silence_threshold = threshold;
        self
    }

    pub fn silence_timeout(mut self, timeout_ms: u64) -> Self {
        self.config.silence_timeout_ms = timeout_ms;
        self
    }

    pub fn auto_continue(mut self, enabled: bool) -> Self {
        self.config.auto_continue = enabled;
        self
    }

    pub fn build(self) -> TalkMode {
        TalkMode::new(self.config)
    }
}

impl Default for TalkModeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_talk_mode_lifecycle() {
        let talk_mode = TalkMode::new(TalkModeConfig::default());

        assert!(!talk_mode.is_running().await);

        talk_mode.start().await.unwrap();
        assert!(talk_mode.is_running().await);
        assert_eq!(talk_mode.state().await, TalkModeState::Listening);

        talk_mode.stop().await.unwrap();
        assert!(!talk_mode.is_running().await);
        assert_eq!(talk_mode.state().await, TalkModeState::Idle);
    }

    #[test]
    fn test_talk_mode_builder() {
        let talk_mode = TalkModeBuilder::new()
            .silence_threshold(0.05)
            .silence_timeout(2000)
            .auto_continue(false)
            .build();

        assert_eq!(talk_mode.config.silence_threshold, 0.05);
        assert_eq!(talk_mode.config.silence_timeout_ms, 2000);
        assert!(!talk_mode.config.auto_continue);
    }
}
