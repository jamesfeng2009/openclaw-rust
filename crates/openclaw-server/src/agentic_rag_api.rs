//! Agentic RAG HTTP API

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agentic_rag::{AgenticRAGEngine, RAGRequest, RAGResponse};

static RAG_ENGINE: std::sync::OnceLock<Arc<RwLock<Option<Arc<AgenticRAGEngine>>>>> = std::sync::OnceLock::new();

pub fn init_agentic_rag_engine(engine: Arc<AgenticRAGEngine>) {
    let _ = RAG_ENGINE.set(Arc::new(RwLock::new(Some(engine))));
}

pub fn get_agentic_rag_engine() -> Option<Arc<AgenticRAGEngine>> {
    RAG_ENGINE.get().map(|e| e.blocking_read().clone()).flatten()
}

pub fn create_agentic_rag_router() -> Router {
    Router::new()
        .route("/api/agentic-rag/query", post(query_handler))
        .route("/api/agentic-rag/status", get(status_handler))
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub history: Option<Vec<openclaw_core::Message>>,
    pub options: Option<crate::agentic_rag::RAGOptions>,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub success: bool,
    pub data: Option<RAGResponse>,
    pub error: Option<String>,
}

async fn query_handler(Json(req): Json<QueryRequest>) -> Json<QueryResponse> {
    let engine = match get_agentic_rag_engine() {
        Some(e) => e,
        None => {
            return Json(QueryResponse {
                success: false,
                data: None,
                error: Some("Agentic RAG engine not initialized".to_string()),
            });
        }
    };

    let mut request = RAGRequest::new(req.query);
    if let Some(history) = req.history {
        request = request.with_history(history);
    }
    if let Some(options) = req.options {
        request = request.with_options(options);
    }

    match engine.process(&request).await {
        Ok(response) => Json(QueryResponse {
            success: true,
            data: Some(response),
            error: None,
        }),
        Err(e) => Json(QueryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        }),
    }
}

async fn status_handler() -> Json<serde_json::Value> {
    let initialized = get_agentic_rag_engine().is_some();

    Json(serde_json::json!({
        "enabled": true,
        "initialized": initialized,
    }))
}
