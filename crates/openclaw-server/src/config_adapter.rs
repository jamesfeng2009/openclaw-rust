//! 统一配置适配器
//!
//! 将 openclaw_core::Config 转换为各模块的配置类型

use openclaw_core::Config as CoreConfig;
use openclaw_device::config::DevicesConfig;
use openclaw_memory::types::MemoryConfig;
use openclaw_security::pipeline::PipelineConfig;
use std::sync::Arc;

pub struct ConfigAdapter {
    core: Arc<CoreConfig>,
    devices: Arc<DevicesConfig>,
}

impl ConfigAdapter {
    pub fn new(core: CoreConfig, devices: DevicesConfig) -> Self {
        Self {
            core: Arc::new(core),
            devices: Arc::new(devices),
        }
    }

    pub fn from_ref(core: &CoreConfig) -> Self {
        Self {
            core: Arc::new(core.clone()),
            devices: Arc::new(DevicesConfig::default()),
        }
    }

    /// 获取 Memory 配置
    pub fn memory(&self) -> MemoryConfig {
        use openclaw_memory::types::*;

        let core = &self.core.memory;

        let custom_emb = core.long_term.custom_embedding.as_ref().map(|c| {
            openclaw_memory::types::CustomEmbeddingConfig {
                base_url: c.base_url.clone(),
                api_key: c.api_key.clone(),
                model: c.model.clone(),
            }
        });

        MemoryConfig {
            backend_type: core.backend_type.clone(),
            short_term: ShortTermMemoryConfig {
                compress_after: core.short_term.compress_after,
                max_summaries: core.short_term.max_summaries,
                compression_mode: core.short_term.compression_mode.clone(),
                summary_model: core.short_term.summary_model.clone(),
            },
            long_term: LongTermMemoryConfig {
                enabled: core.long_term.enabled,
                backend: core.long_term.backend.clone(),
                collection: core.long_term.collection.clone(),
                embedding_provider: core.long_term.embedding_provider.clone(),
                embedding_model: core.long_term.embedding_model.clone(),
                embedding_dimensions: core.long_term.embedding_dimensions,
                chunk_size: core.long_term.chunk_size,
                overlap: core.long_term.overlap,
                enable_bm25: core.long_term.enable_bm25,
                enable_knowledge_graph: core.long_term.enable_knowledge_graph,
                custom_embedding: custom_emb,
                ..Default::default()
            },
            working: WorkingMemoryConfig {
                max_messages: core.working.max_messages,
                max_tokens: core.working.max_tokens,
            },
            ..Default::default()
        }
    }

    /// 获取 Security 配置
    pub fn security(&self) -> PipelineConfig {
        let core = &self.core.security;

        PipelineConfig {
            enable_input_filter: core.enable_input_filter,
            enable_classifier: core.enable_classifier,
            enable_output_validation: core.enable_output_validation,
            enable_audit: core.enable_audit,
            enable_self_healer: core.enable_self_healer,
            classifier_strict_mode: core.classifier_strict_mode,
            stuck_timeout: core.stuck_timeout,
        }
    }

    /// 获取 AI Provider 配置
    pub fn ai_provider(&self) -> openclaw_core::config::ProviderConfig {
        let core = &self.core.ai;

        core.providers
            .iter()
            .find(|p| p.name == core.default_provider)
            .cloned()
            .unwrap_or_else(|| openclaw_core::config::ProviderConfig {
                name: core.default_provider.clone(),
                provider_type: openclaw_core::config::ProviderType::OpenAI,
                api_key: None,
                base_url: None,
                default_model: "gpt-4o".to_string(),
                models: vec![],
                auth: Default::default(),
            })
    }

    /// 获取语音配置 (STT/TTS)
    pub fn voice(&self) -> openclaw_core::config::VoiceServerConfig {
        self.core
            .voice
            .clone()
            .unwrap_or_else(|| openclaw_core::config::VoiceServerConfig {
                stt_provider: "openai".to_string(),
                tts_provider: "openai".to_string(),
                api_key: None,
            })
    }

    /// 获取原始 Core Config
    pub fn core(&self) -> &CoreConfig {
        &self.core
    }

    /// 获取设备配置
    pub fn device(&self) -> DevicesConfig {
        (*self.devices).clone()
    }

    /// 获取沙箱配置
    pub fn sandbox(&self) -> &openclaw_core::config::SandboxSettings {
        &self.core.sandbox
    }
}
