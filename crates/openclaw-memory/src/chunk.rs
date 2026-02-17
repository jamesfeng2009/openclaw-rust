use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tiktoken_rs::cl100k_base;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub content: String,
    pub token_count: usize,
    pub start_index: usize,
    pub end_index: usize,
    pub source: String,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkMetadata {
    pub created_at: i64,
    pub chunk_index: usize,
    pub total_chunks: usize,
    pub document_id: Option<String>,
}

pub struct ChunkManager {
    chunk_size: usize,
    overlap: usize,
    #[allow(dead_code)]
    model: String,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self::new(400, 50, "cl100k_base")
    }
}

impl ChunkManager {
    pub fn new(chunk_size: usize, overlap: usize, model: &str) -> Self {
        Self {
            chunk_size,
            overlap,
            model: model.to_string(),
        }
    }

    pub fn chunk_text(&self, text: &str, source: &str) -> Result<Vec<Chunk>> {
        let bpe = cl100k_base()?;

        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();

        if total_chars == 0 {
            return Ok(vec![]);
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < total_chars {
            let end = (start + self.chunk_size * 4).min(total_chars);

            let mut chunk_chars = &chars[start..end];

            if start > 0 && chunk_chars.len() > self.overlap {
                let overlap_chars = chunk_chars.len() - self.overlap;
                chunk_chars = &chunk_chars[chunk_chars.len() - overlap_chars..];
            }

            let chunk_text: String = chunk_chars.iter().collect();
            let tokens = bpe.encode_with_special_tokens(&chunk_text);
            let token_count = tokens.len();

            if token_count == 0 {
                break;
            }

            let chunk = Chunk {
                id: format!("{}_{}_{}", source, chunk_index, uuid::Uuid::new_v4()),
                content: chunk_text.clone(),
                token_count,
                start_index: start,
                end_index: start + chunk_chars.len(),
                source: source.to_string(),
                metadata: ChunkMetadata {
                    created_at: chrono::Utc::now().timestamp(),
                    chunk_index,
                    total_chunks: 0,
                    document_id: None,
                },
            };

            chunks.push(chunk);

            if end >= total_chars {
                break;
            }

            start += chunk_chars.len() - self.overlap;
            chunk_index += 1;
        }

        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total_chunks;
        }

        Ok(chunks)
    }

    pub fn chunk_text_by_paragraph(&self, text: &str, source: &str) -> Result<Vec<Chunk>> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;
        let mut chunk_index = 0;

        let bpe = cl100k_base()?;

        for para in paragraphs {
            let para_tokens = bpe.encode_with_special_tokens(para).len();

            if current_tokens + para_tokens > self.chunk_size && !current_chunk.is_empty() {
                chunks.push(Chunk {
                    id: format!("{}_{}_{}", source, chunk_index, uuid::Uuid::new_v4()),
                    content: current_chunk.trim().to_string(),
                    token_count: current_tokens,
                    start_index: 0,
                    end_index: current_chunk.len(),
                    source: source.to_string(),
                    metadata: ChunkMetadata {
                        created_at: chrono::Utc::now().timestamp(),
                        chunk_index,
                        total_chunks: 0,
                        document_id: None,
                    },
                });

                chunk_index += 1;
                current_chunk = String::new();
                current_tokens = 0;
            }

            current_chunk.push_str(para);
            current_chunk.push_str("\n\n");
            current_tokens += para_tokens;
        }

        if !current_chunk.is_empty() {
            chunks.push(Chunk {
                id: format!("{}_{}_{}", source, chunk_index, uuid::Uuid::new_v4()),
                content: current_chunk.trim().to_string(),
                token_count: current_tokens,
                start_index: 0,
                end_index: current_chunk.len(),
                source: source.to_string(),
                metadata: ChunkMetadata {
                    created_at: chrono::Utc::now().timestamp(),
                    chunk_index,
                    total_chunks: chunks.len(),
                    document_id: None,
                },
            });
        }

        let total_chunks = chunks.len();
        for chunk in &mut chunks {
            chunk.metadata.total_chunks = total_chunks;
        }

        Ok(chunks)
    }

    pub fn chunk_file(&self, file_path: &PathBuf, source: &str) -> Result<Vec<Chunk>> {
        let content = std::fs::read_to_string(file_path)?;
        self.chunk_text(&content, source)
    }

    pub fn get_chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size;
    }

    pub fn get_overlap(&self) -> usize {
        self.overlap
    }

    pub fn set_overlap(&mut self, overlap: usize) {
        self.overlap = overlap;
    }
}

pub struct ChunkConfig {
    pub chunk_size: usize,
    pub overlap: usize,
    pub model: String,
    pub strategy: ChunkStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChunkStrategy {
    Fixed,
    Paragraph,
    Sentence,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 400,
            overlap: 50,
            model: "cl100k_base".to_string(),
            strategy: ChunkStrategy::Paragraph,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text() {
        let manager = ChunkManager::default();

        let text = "This is a test. ".repeat(100);
        let chunks = manager.chunk_text(&text, "test").unwrap();

        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_by_paragraph() {
        let manager = ChunkManager::default();

        let text = "Paragraph 1\n\nParagraph 2\n\nParagraph 3";
        let chunks = manager.chunk_text_by_paragraph(text, "test").unwrap();

        assert!(!chunks.is_empty());
    }
}
