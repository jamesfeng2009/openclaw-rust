//! 语音配置 CLI 工具
//!
//! 提供命令行接口来管理语音功能和配置

use clap::Subcommand;
use openclaw_core::OpenClawError;
use openclaw_voice::{
    SttProvider, SynthesisOptions,
    TalkModeBuilder, TalkModeEvent, TtsProvider, VoiceConfigManager,
};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum VoiceCommand {
    /// 设置语音 API Key
    SetKey {
        /// 提供商 (openai, azure, google)
        #[arg(default_value = "openai")]
        provider: String,
        /// API Key
        api_key: String,
        /// Base URL (可选)
        #[arg(short, long)]
        url: Option<String>,
    },

    /// 语音识别 (STT)
    Transcribe {
        /// 音频文件路径
        audio_file: String,
        /// 语言 (可选，自动检测)
        #[arg(short, long)]
        language: Option<String>,
        /// 提供商 (openai, local)
        #[arg(short, long, default_value = "openai")]
        provider: String,
    },

    /// 语音合成 (TTS)
    Synthesize {
        /// 要转换的文本
        text: String,
        /// 输出文件路径
        #[arg(short, long, default_value = "output.mp3")]
        output: String,
        /// 语音 (alloy, echo, fable, onyx, nova, shimmer)
        #[arg(short, long, default_value = "alloy")]
        voice: String,
        /// 语速 (0.25 - 4.0)
        #[arg(short, long, default_value = "1.0")]
        speed: f32,
        /// 提供商 (openai, edge)
        #[arg(short, long, default_value = "openai")]
        provider: String,
    },

    /// 启动持续对话模式
    Talk {
        /// 静音检测阈值
        #[arg(long, default_value = "0.02")]
        silence_threshold: f32,
        /// 静音超时 (毫秒)
        #[arg(long, default_value = "1500")]
        silence_timeout: u64,
        /// 是否自动继续
        #[arg(long, default_value = "true")]
        auto_continue: bool,
    },

    /// 启用/禁用语音功能
    Enable {
        /// 是否启用
        #[arg(default_value = "true")]
        enabled: bool,
    },

    /// 显示语音配置
    Config,

    /// 列出可用语音
    Voices {
        /// 提供商 (openai, edge)
        #[arg(default_value = "openai")]
        provider: String,
    },

    /// 检查麦克风
    CheckMic,

    /// 播放音频文件
    Play {
        /// 音频文件路径
        audio_file: String,
    },
}

impl VoiceCommand {
    /// 执行命令
    pub async fn execute(&self) -> Result<(), OpenClawError> {
        let mut manager = VoiceConfigManager::load();

        match self {
            VoiceCommand::SetKey {
                provider,
                api_key,
                url,
            } => {
                let provider_lower = provider.to_lowercase();

                match provider_lower.as_str() {
                    "openai" => {
                        manager.set_stt_api_key(SttProvider::OpenAI, api_key.clone());
                        manager.set_tts_api_key(TtsProvider::OpenAI, api_key.clone());
                        if let Some(base_url) = url {
                            manager.set_openai_base_url(base_url.clone());
                        }
                        manager.save()?;
                        println!("✅ 已设置 OpenAI API Key");
                    }
                    "azure" => {
                        println!("⚠️  Azure Speech 尚未实现");
                    }
                    "google" => {
                        println!("⚠️  Google Speech 尚未实现");
                    }
                    _ => {
                        println!("❌ 不支持的提供商: {}", provider);
                        println!("\n支持的提供商: openai, azure, google");
                    }
                }
            }

            VoiceCommand::Transcribe {
                audio_file,
                language,
                provider,
            } => {
                let path = PathBuf::from(audio_file);
                if !path.exists() {
                    println!("❌ 文件不存在: {}", audio_file);
                    return Ok(());
                }

                println!("🔍 正在识别语音...");

                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => SttProvider::OpenAI,
                    "local" => SttProvider::LocalWhisper,
                    _ => SttProvider::OpenAI,
                };

                let config = manager.voice.stt_config.clone();
                let stt = openclaw_voice::create_stt(provider_type, config);

                match stt.transcribe_file(&path, language.as_deref()).await {
                    Ok(result) => {
                        println!("\n📝 转录结果:");
                        println!("{}", result.text);
                        if let Some(lang) = result.language {
                            println!("\n🌐 检测语言: {}", lang);
                        }
                        if let Some(duration) = result.duration {
                            println!("⏱️  时长: {:.2} 秒", duration);
                        }
                    }
                    Err(e) => {
                        println!("❌ 转录失败: {}", e);
                        println!("\n请确保已设置 API Key:");
                        println!("  openclaw-rust voice set-key openai sk-xxx");
                    }
                }
            }

            VoiceCommand::Synthesize {
                text,
                output,
                voice,
                speed,
                provider,
            } => {
                println!("🔊 正在合成语音...");

                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => TtsProvider::OpenAI,
                    "edge" => TtsProvider::Edge,
                    _ => TtsProvider::OpenAI,
                };

                let config = manager.voice.tts_config.clone();
                let tts = openclaw_voice::create_tts(provider_type, config);

                let options = SynthesisOptions {
                    voice: Some(voice.clone()),
                    speed: Some(*speed),
                    ..Default::default()
                };

                let output_path = PathBuf::from(output);

                match tts.synthesize_to_file(text, &output_path, Some(options)).await {
                    Ok(_) => {
                        println!("✅ 语音已保存到: {}", output);
                    }
                    Err(e) => {
                        println!("❌ 合成失败: {}", e);
                        println!("\n请确保已设置 API Key:");
                        println!("  openclaw-rust voice set-key openai sk-xxx");
                    }
                }
            }

            VoiceCommand::Talk {
                silence_threshold,
                silence_timeout,
                auto_continue,
            } => {
                println!("🎤 启动持续对话模式...");
                println!("   静音阈值: {}", silence_threshold);
                println!("   静音超时: {}ms", silence_timeout);
                println!("   自动继续: {}", auto_continue);
                println!();
                println!("按 Ctrl+C 退出");

                let talk_mode = TalkModeBuilder::new()
                    .silence_threshold(*silence_threshold)
                    .silence_timeout(*silence_timeout)
                    .auto_continue(*auto_continue)
                    .build();

                // 订阅事件
                let mut rx = talk_mode.subscribe();

                // 启动
                talk_mode.start().await?;

                // 监听事件
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            match event {
                                TalkModeEvent::ListeningStarted => {
                                    println!("👂 正在监听...");
                                }
                                TalkModeEvent::Transcription(text) => {
                                    println!("👤 你: {}", text);
                                }
                                TalkModeEvent::AiResponse(text) => {
                                    println!("🤖 AI: {}", text);
                                }
                                TalkModeEvent::StateChanged(state) => {
                                    tracing::debug!("状态: {:?}", state);
                                }
                                TalkModeEvent::Error(e) => {
                                    println!("❌ 错误: {}", e);
                                }
                                _ => {}
                            }
                        }
                        Err(_) => break,
                    }

                    if !talk_mode.is_running().await {
                        break;
                    }
                }
            }

            VoiceCommand::Enable { enabled } => {
                manager.set_enabled(*enabled);
                manager.save()?;
                println!(
                    "✅ 语音功能已{}",
                    if *enabled { "启用" } else { "禁用" }
                );
            }

            VoiceCommand::Config => {
                println!("📋 语音配置:");
                println!();
                println!("  状态: {}", if manager.voice.enabled { "已启用" } else { "已禁用" });
                println!("  STT 提供商: {:?}", manager.voice.stt_provider);
                println!("  TTS 提供商: {:?}", manager.voice.tts_provider);
                println!();

                // STT 配置
                println!("  STT 配置:");
                if let Some(key) = &manager.voice.stt_config.openai_api_key {
                    let masked = mask_api_key(key);
                    println!("    OpenAI Key: {}", masked);
                } else {
                    println!("    OpenAI Key: 未设置");
                }
                if let Some(url) = &manager.voice.stt_config.openai_base_url {
                    println!("    Base URL: {}", url);
                }
                println!();

                // TTS 配置
                println!("  TTS 配置:");
                if let Some(key) = &manager.voice.tts_config.openai_api_key {
                    let masked = mask_api_key(key);
                    println!("    OpenAI Key: {}", masked);
                } else {
                    println!("    OpenAI Key: 未设置");
                }
                println!("    默认语音: {:?}", manager.voice.tts_config.default_voice);
                println!("    默认语速: {}", manager.voice.tts_config.default_speed);
            }

            VoiceCommand::Voices { provider } => {
                let provider_type = match provider.to_lowercase().as_str() {
                    "openai" => TtsProvider::OpenAI,
                    "edge" => TtsProvider::Edge,
                    _ => TtsProvider::OpenAI,
                };

                let config = manager.voice.tts_config.clone();
                let tts = openclaw_voice::create_tts(provider_type, config);
                let voices = tts.available_voices();

                println!("🎙️  可用语音 ({}) :", provider);
                println!();
                for voice in voices {
                    println!("  - {}", voice);
                }
            }

            VoiceCommand::CheckMic => {
                println!("🎤 检查麦克风...");
                // TODO: 实现麦克风检测
                println!("⚠️  麦克风检测功能开发中");
                println!();
                println!("手动测试方法:");
                println!("  1. 确保系统已授权麦克风权限");
                println!("  2. 使用 'openclaw-rust voice talk' 测试录音");
            }

            VoiceCommand::Play { audio_file } => {
                let path = PathBuf::from(audio_file);
                if !path.exists() {
                    println!("❌ 文件不存在: {}", audio_file);
                    return Ok(());
                }

                println!("▶️  播放音频: {}", audio_file);
                // TODO: 实现音频播放
                println!("⚠️  音频播放功能开发中");
                println!();
                println!("临时方案: 使用系统播放器");
                println!("  macOS: open {}", audio_file);
                println!("  Linux: xdg-open {}", audio_file);
                println!("  Windows: start {}", audio_file);
            }
        }

        Ok(())
    }
}

/// 隐藏 API Key 中间部分
fn mask_api_key(key: &str) -> String {
    if key.len() <= 12 {
        return "*".repeat(key.len());
    }

    let start = &key[..8];
    let end = &key[key.len() - 4..];
    format!("{}****{}", start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key() {
        assert_eq!(mask_api_key("sk-short"), "********");
        assert_eq!(
            mask_api_key("sk-1234567890abcdef"),
            "sk-12345****cdef"
        );
    }
}
