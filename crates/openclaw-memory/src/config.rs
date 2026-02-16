use std::path::PathBuf;
use std::sync::Arc;

use openclaw_ai::{
    providers::{CustomProvider, ProviderFactory},
    AIProvider,
};
use openclaw_core::Result;
use openclaw_vector::{VectorStore, StoreBackend, create_store_async};

use crate::ai_adapter::AIProviderEmbeddingAdapter;
use crate::embedding::EmbeddingProvider;
use crate::types::MemoryConfig;

#[derive(Debug, Clone)]
pub enum MemoryBackend {
    Memory,
    SQLite { path: PathBuf },
    LanceDB { path: PathBuf },
    Qdrant { url: String, collection: String },
    PgVector { url: String },
}

impl From<&str> for MemoryBackend {
    fn from(backend: &str) -> Self {
        match backend {
            "memory" => MemoryBackend::Memory,
            "sqlite" => MemoryBackend::SQLite { 
                path: PathBuf::from("data/memory.db"),
            },
            "lancedb" => MemoryBackend::LanceDB { 
                path: PathBuf::from("data/lancedb"),
            },
            "qdrant" => MemoryBackend::Qdrant { 
                url: "http://localhost:6333".to_string(),
                collection: "openclaw_memories".to_string(),
            },
            "pgvector" => MemoryBackend::PgVector { 
                url: "postgresql://localhost/openclaw".to_string(),
            },
            _ => MemoryBackend::LanceDB { 
                path: PathBuf::from("data/lancedb"),
            },
        }
    }
}

pub async fn create_memory_store(
    backend: MemoryBackend,
    table_name: &str,
) -> Result<Arc<dyn VectorStore>> {
    let store_backend = match backend {
        MemoryBackend::Memory => StoreBackend::Memory,
        
        MemoryBackend::SQLite { path } => {
            StoreBackend::SQLite { path, table: table_name.to_string() }
        }
        
        MemoryBackend::LanceDB { path } => {
            StoreBackend::LanceDB { path }
        }
        
        MemoryBackend::Qdrant { url, collection } => {
            StoreBackend::Qdrant { 
                url, 
                collection, 
                api_key: None,
            }
        }
        
        MemoryBackend::PgVector { url } => {
            StoreBackend::PgVector { 
                url, 
                table: table_name.to_string(),
            }
        }
    };

    create_store_async(store_backend).await
}

pub async fn create_memory_store_from_config(
    config: &MemoryConfig,
) -> Result<Option<Arc<dyn VectorStore>>> {
    let long_term_config = &config.long_term;
    
    if !long_term_config.enabled {
        return Ok(None);
    }

    let backend = MemoryBackend::from(long_term_config.backend.as_str());
    let store = create_memory_store(backend, &long_term_config.collection).await?;
    
    Ok(Some(store))
}

pub async fn create_embedding_provider_from_config(
    provider_name: &str,
    model: &str,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<Arc<dyn EmbeddingProvider>> {
    let provider: Arc<dyn AIProvider> = match provider_name {
        "custom" => {
            let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let api_key = api_key.unwrap_or_else(|| "dummy".to_string());
            let custom = CustomProvider::new("custom", &base_url, &api_key);
            if !model.is_empty() {
                Arc::new(custom.with_default_model(model))
            } else {
                Arc::new(custom)
            }
        }
        "gemini" => {
            ProviderFactory::create_from_name("gemini", api_key, base_url)
                .map_err(|e| openclaw_core::OpenClawError::Config(format!("Failed to create Gemini provider: {}", e)))?
        }
        "anthropic" => {
            return Err(openclaw_core::OpenClawError::Config(
                "Anthropic (Claude) does not provide embedding API. Please use another provider (OpenAI, Ollama, DeepSeek, GLM, Qwen, Minimax, Kimi, or custom).".to_string()
            ));
        }
        _ => {
            ProviderFactory::create_from_name(provider_name, api_key, base_url)
                .map_err(|e| openclaw_core::OpenClawError::Config(format!("Failed to create provider: {}", e)))?
        }
    };

    let dimensions = match provider_name {
        "openai" => match model {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        },
        "gemini" => 768,
        "ollama" => 768,
        "deepseek" => 1536,
        "glm" => 1536,
        "qwen" => 1536,
        "minimax" => 1536,
        "kimi" => 1536,
        "custom" => 1536,
        _ => 1536,
    };

    let adapter = AIProviderEmbeddingAdapter::new(
        provider,
        model.to_string(),
        dimensions,
    );

    Ok(Arc::new(adapter))
}
