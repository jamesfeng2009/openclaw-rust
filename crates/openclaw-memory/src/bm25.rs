use anyhow::Result;
use std::path::{Path, PathBuf};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy, TantivyDocument, doc};

pub struct Bm25Index {
    index: Index,
    id_field: Field,
    content_field: Field,
    source_field: Field,
    timestamp_field: Field,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub source: String,
    pub score: f32,
    pub timestamp: i64,
}

impl Bm25Index {
    pub fn new(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let source_field = schema_builder.add_text_field("source", STRING | STORED);
        let timestamp_field = schema_builder.add_i64_field("timestamp", INDEXED | STORED | FAST);

        let schema = schema_builder.build();

        std::fs::create_dir_all(index_path)?;

        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
        };

        Ok(Self {
            index,
            id_field,
            content_field,
            source_field,
            timestamp_field,
        })
    }

    pub async fn add_document(
        &self,
        id: &str,
        content: &str,
        source: &str,
        timestamp: i64,
    ) -> Result<()> {
        let mut writer = self.index.writer(50_000_000)?;

        writer.add_document(doc!(
            self.id_field => id,
            self.content_field => content,
            self.source_field => source,
            self.timestamp_field => timestamp,
        ))?;

        writer.commit()?;

        Ok(())
    }

    pub async fn add_documents_batch(
        &self,
        docs: Vec<(String, String, String, i64)>,
    ) -> Result<()> {
        let mut writer = self.index.writer(50_000_000)?;

        for (id, content, source, timestamp) in docs {
            writer.add_document(doc!(
                self.id_field => id,
                self.content_field => content,
                self.source_field => source,
                self.timestamp_field => timestamp,
            ))?;
        }

        writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);

        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source = retrieved_doc
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let timestamp = retrieved_doc
                .get_first(self.timestamp_field)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            results.push(SearchResult {
                id,
                content,
                source,
                score,
                timestamp,
            });
        }

        Ok(results)
    }

    pub fn search_with_filters(
        &self,
        query_str: &str,
        source_filter: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);

        let query = query_parser.parse_query(query_str)?;

        let mut results = Vec::new();

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit * 2))?;

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let source = retrieved_doc
                .get_first(self.source_field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(filter) = source_filter {
                if source != filter {
                    continue;
                }
            }

            let timestamp = retrieved_doc
                .get_first(self.timestamp_field)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            if let Some(start) = start_time {
                if timestamp < start {
                    continue;
                }
            }

            if let Some(end) = end_time {
                if timestamp > end {
                    continue;
                }
            }

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult {
                id,
                content,
                source: source.to_string(),
                score,
                timestamp,
            });

            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    pub async fn delete_document(&self, id: &str) -> Result<()> {
        let mut writer: tantivy::IndexWriter = self.index.writer(50_000_000)?;

        let term = tantivy::Term::from_field_text(self.id_field, id);
        writer.delete_term(term);

        writer.commit()?;

        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        let mut writer: tantivy::IndexWriter = self.index.writer(50_000_000)?;

        writer.delete_all_documents()?;
        writer.commit()?;

        Ok(())
    }
}

pub struct Bm25Config {
    pub index_path: PathBuf,
    pub default_limit: usize,
    pub min_score: f32,
}

impl Default for Bm25Config {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from(".openclaw-rust/indexes/bm25"),
            default_limit: 10,
            min_score: 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_bm25_search() {
        let temp_dir = env::temp_dir().join("openclaw_test_bm25");

        let index = Bm25Index::new(&temp_dir).unwrap();

        let docs = vec![
            (
                "doc1".to_string(),
                "Rust is a systems programming language".to_string(),
                "memory".to_string(),
                1000,
            ),
            (
                "doc2".to_string(),
                "Python is great for data science".to_string(),
                "memory".to_string(),
                2000,
            ),
            (
                "doc3".to_string(),
                "Rust has excellent memory safety".to_string(),
                "memory".to_string(),
                3000,
            ),
        ];

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(index.add_documents_batch(docs)).unwrap();

        let results = index.search("Rust memory", 10).unwrap();

        assert!(!results.is_empty());

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
