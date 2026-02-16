use std::path::PathBuf;
use std::sync::Arc;

use openclaw_core::Result;
use openclaw_vector::{VectorStore, StoreBackend, create_store_async};

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
