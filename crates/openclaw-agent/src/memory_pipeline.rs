use anyhow::Result;
use openclaw_memory::{
    AgentWorkspace, Bm25Index,
    chunk::ChunkManager,
    file_tracker::{FileTracker, FileTrackerConfig},
    manager::MemoryManager as MemManager,
    recall::RecallResult,
    recall_strategy::{RecallConfig, RecallItem as RecallStrategyItem, RecallStrategy},
};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct MemoryPipelineConfig {
    pub workspace_path: PathBuf,
    pub index_path: PathBuf,
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub recall_config: RecallConfig,
    pub vector_weight: f32,
    pub bm25_weight: f32,
}

impl Default for MemoryPipelineConfig {
    fn default() -> Self {
        Self {
            workspace_path: PathBuf::from(".openclaw-rust/agents"),
            index_path: PathBuf::from(".openclaw-rust/indexes"),
            chunk_size: 400,
            chunk_overlap: 50,
            recall_config: RecallConfig::default(),
            vector_weight: 0.6,
            bm25_weight: 0.4,
        }
    }
}

pub struct MemoryPipeline {
    config: MemoryPipelineConfig,
    workspace: Option<Arc<AgentWorkspace>>,
    bm25_index: Option<Arc<Bm25Index>>,
    chunk_manager: ChunkManager,
    recall_strategy: RecallStrategy,
    file_tracker: Option<FileTracker>,
    memory_manager: Option<Arc<MemManager>>,
}

impl MemoryPipeline {
    pub fn new(config: MemoryPipelineConfig) -> Self {
        Self {
            config: config.clone(),
            workspace: None,
            bm25_index: None,
            chunk_manager: ChunkManager::new(
                config.chunk_size,
                config.chunk_overlap,
                "cl100k_base",
            ),
            recall_strategy: RecallStrategy::new(config.recall_config),
            file_tracker: None,
            memory_manager: None,
        }
    }

    pub fn initialize(&mut self, agent_id: &str) -> Result<()> {
        let agent_workspace = self.config.workspace_path.join(agent_id);

        let workspace = AgentWorkspace::new(agent_id.to_string(), agent_workspace.clone());
        workspace.initialize()?;

        self.workspace = Some(Arc::new(workspace));

        let index_path = self.config.index_path.join(agent_id).join("bm25");
        let bm25_index = Bm25Index::new(&index_path)?;
        self.bm25_index = Some(Arc::new(bm25_index));

        let tracker_config = FileTrackerConfig {
            data_dir: agent_workspace.join(".tracker"),
            index_file: agent_workspace.join(".tracker").join("index.json"),
        };
        let mut file_tracker = FileTracker::new(tracker_config);
        file_tracker.load()?;
        self.file_tracker = Some(file_tracker);

        Ok(())
    }

    pub fn with_memory_manager(mut self, memory: Arc<MemManager>) -> Self {
        self.memory_manager = Some(memory);
        self
    }

    pub fn workspace(&self) -> Option<&Arc<AgentWorkspace>> {
        self.workspace.as_ref()
    }

    pub fn recall(&self, query: &str, limit: usize) -> Result<Vec<RecallStrategyItem>> {
        let mut all_items = Vec::new();

        if let Some(bm25) = &self.bm25_index {
            let bm25_results = bm25.search(query, limit)?;
            for r in bm25_results {
                all_items.push(RecallStrategyItem {
                    id: r.id,
                    content: r.content,
                    score: r.score,
                    source: r.source,
                    timestamp: r.timestamp,
                    importance: 0.5,
                    access_count: 0,
                    last_access: None,
                });
            }
        }

        if let Some(memory) = &self.memory_manager {
            let retrieval: RecallResult = futures::executor::block_on(memory.recall(query))?;
            for item in retrieval.items {
                all_items.push(RecallStrategyItem {
                    id: item.id,
                    content: item.content,
                    score: item.similarity,
                    source: item.source,
                    timestamp: 0,
                    importance: item.similarity,
                    access_count: 0,
                    last_access: None,
                });
            }
        }

        let reranked = self.recall_strategy.rerank(all_items, query);

        Ok(reranked)
    }

    pub fn get_context_for_prompt(&self) -> Result<String> {
        match &self.workspace {
            Some(ws) => Ok(ws.get_context_for_prompt()?),
            None => Ok(String::new()),
        }
    }

    pub async fn learn(&self, content: &str, source: &str) -> Result<()> {
        let chunks = self.chunk_manager.chunk_text(content, source)?;

        if let Some(bm25) = &self.bm25_index {
            for chunk in chunks {
                let _ = bm25
                    .add_document(&chunk.id, &chunk.content, source, chunk.metadata.created_at)
                    .await;
            }
        }

        Ok(())
    }

    pub async fn sync_workspace(&mut self) -> Result<Vec<PathBuf>> {
        if self.workspace.is_none() || self.file_tracker.is_none() {
            return Ok(Vec::new());
        }

        let ws_path = self.workspace.as_ref().unwrap().workspace_path();
        let tracker = self.file_tracker.as_mut().unwrap();

        let changed = tracker.scan_directory(ws_path)?;

        let mut temp_chunks = Vec::new();
        for path in &changed {
            if let Ok(content) = std::fs::read_to_string(path) {
                let chunks = self
                    .chunk_manager
                    .chunk_text(&content, &path.to_string_lossy())?;
                temp_chunks.extend(chunks);
            }
        }

        if let Some(bm25) = &self.bm25_index {
            for chunk in temp_chunks {
                let _ = bm25
                    .add_document(
                        &chunk.id,
                        &chunk.content,
                        &chunk.source,
                        chunk.metadata.created_at,
                    )
                    .await;
            }
        }

        tracker.save()?;

        Ok(changed)
    }
}
