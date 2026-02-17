//! SQLite 向量存储实现 - 支持 FTS5 全文搜索和向量相似度搜索

use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::VectorStore;
use crate::types::{Filter, SearchQuery, SearchResult, StoreStats, VectorItem};
use openclaw_core::{OpenClawError, Result};

/// SQLite 向量存储
pub struct SqliteStore {
    conn: Mutex<Connection>,
    table_name: String,
}

impl SqliteStore {
    pub fn new(path: PathBuf, table_name: &str) -> Result<Self> {
        let conn = Connection::open(&path).map_err(|e| OpenClawError::Config(e.to_string()))?;

        conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    id TEXT PRIMARY KEY,
                    vector BLOB NOT NULL,
                    content TEXT,
                    payload TEXT,
                    created_at TEXT NOT NULL
                )",
                table_name
            ),
            [],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;

        conn.execute(
            &format!(
                "CREATE VIRTUAL TABLE IF NOT EXISTS {}_fts USING fts5(
                    id,
                    content,
                    tokenize='porter unicode61'
                )",
                table_name
            ),
            [],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;

        Ok(Self {
            conn: Mutex::new(conn),
            table_name: table_name.to_string(),
        })
    }

    pub fn upsert(&self, item: VectorItem) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        let vector_blob = serialize_vector(&item.vector);

        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {} (id, vector, content, payload, created_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                self.table_name
            ),
            params![
                item.id,
                vector_blob,
                item.payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                item.payload.to_string(),
                item.created_at.to_rfc3339()
            ],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let content = item
            .payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        conn.execute(
            &format!(
                "INSERT OR REPLACE INTO {}_fts (id, content) VALUES (?1, ?2)",
                self.table_name
            ),
            params![item.id, content],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;

        Ok(())
    }

    pub fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        for item in &items {
            let vector_blob = serialize_vector(&item.vector);

            conn.execute(
                &format!(
                    "INSERT OR REPLACE INTO {} (id, vector, content, payload, created_at) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    self.table_name
                ),
                params![
                    item.id,
                    vector_blob,
                    item.payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                    item.payload.to_string(),
                    item.created_at.to_rfc3339()
                ],
            )
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

            let content = item
                .payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            conn.execute(
                &format!(
                    "INSERT OR REPLACE INTO {}_fts (id, content) VALUES (?1, ?2)",
                    self.table_name
                ),
                params![item.id, content],
            )
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        }

        Ok(items.len())
    }

    pub fn vector_search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, vector, payload FROM {}",
                self.table_name
            ))
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let query_vector = &query.vector;
        let mut results: Vec<SearchResult> = Vec::new();

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let vector_blob: Vec<u8> = row.get(1)?;
                let payload_str: String = row.get(2)?;
                Ok((id, vector_blob, payload_str))
            })
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        for row in rows {
            let (id, vector_blob, payload_str) =
                row.map_err(|e| OpenClawError::Config(e.to_string()))?;
            let stored_vector = deserialize_vector(&vector_blob);
            let score = cosine_similarity(query_vector, &stored_vector);

            if let Some(min_score) = query.min_score {
                if score < min_score {
                    continue;
                }
            }

            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            results.push(SearchResult { id, score, payload });
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(query.limit);
        Ok(results)
    }

    pub fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let mut stmt = conn.prepare(&format!(
            "SELECT f.id, m.payload FROM {}_fts f JOIN {} m ON f.id = m.id WHERE {}fts MATCH ?1 LIMIT ?2",
            self.table_name, self.table_name, self.table_name
        )).map_err(|e| OpenClawError::Config(e.to_string()))?;

        let mut results = Vec::new();
        let rows = stmt
            .query_map(params![query, limit as i64], |row| {
                let id: String = row.get(0)?;
                let payload_str: String = row.get(1)?;
                Ok((id, payload_str))
            })
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        for row in rows {
            let (id, payload_str) = row.map_err(|e| OpenClawError::Config(e.to_string()))?;
            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            results.push(SearchResult {
                id,
                score: 1.0,
                payload,
            });
        }

        Ok(results)
    }

    pub fn hybrid_search(
        &self,
        query_vector: &[f32],
        query_text: &str,
        vector_weight: f32,
        keyword_weight: f32,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let vector_results = if vector_weight > 0.0 {
            let q = SearchQuery::new(query_vector.to_vec());
            self.vector_search(&q)?
        } else {
            vec![]
        };

        let fts_results = if keyword_weight > 0.0 && !query_text.is_empty() {
            self.fts_search(query_text, limit)?
        } else {
            vec![]
        };

        Ok(self.merge_results(
            vector_results,
            fts_results,
            vector_weight,
            keyword_weight,
            limit,
        ))
    }

    fn merge_results(
        &self,
        vector_results: Vec<SearchResult>,
        fts_results: Vec<SearchResult>,
        vector_weight: f32,
        keyword_weight: f32,
        limit: usize,
    ) -> Vec<SearchResult> {
        use std::collections::HashMap;

        let mut combined: HashMap<String, SearchResult> = HashMap::new();

        let max_vector_score = vector_results.first().map(|r| r.score).unwrap_or(1.0);
        for result in vector_results {
            let normalized_score = if max_vector_score > 0.0 {
                result.score / max_vector_score
            } else {
                0.0
            } * vector_weight;
            combined.insert(
                result.id.clone(),
                SearchResult {
                    id: result.id,
                    score: normalized_score,
                    payload: result.payload,
                },
            );
        }

        for result in fts_results {
            let normalized_score = keyword_weight;
            if let Some(existing) = combined.get_mut(&result.id) {
                existing.score += normalized_score;
            } else {
                combined.insert(
                    result.id.clone(),
                    SearchResult {
                        id: result.id,
                        score: normalized_score,
                        payload: result.payload,
                    },
                );
            }
        }

        let mut results: Vec<SearchResult> = combined.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        results
    }

    pub fn get(&self, id: &str) -> Result<Option<VectorItem>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT id, vector, payload, created_at FROM {} WHERE id = ?1",
                self.table_name
            ))
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        let mut rows = stmt
            .query(params![id])
            .map_err(|e| OpenClawError::Config(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| OpenClawError::Config(e.to_string()))?
        {
            let vector_blob: Vec<u8> = row
                .get(1)
                .map_err(|e| OpenClawError::Config(e.to_string()))?;
            let payload_str: String = row
                .get(2)
                .map_err(|e| OpenClawError::Config(e.to_string()))?;
            let created_at_str: String = row
                .get(3)
                .map_err(|e| OpenClawError::Config(e.to_string()))?;

            let payload: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            Ok(Some(VectorItem {
                id: id.to_string(),
                vector: deserialize_vector(&vector_blob),
                payload,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        conn.execute(
            &format!("DELETE FROM {} WHERE id = ?1", self.table_name),
            params![id],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;
        conn.execute(
            &format!("DELETE FROM {}_fts WHERE id = ?1", self.table_name),
            params![id],
        )
        .map_err(|e| OpenClawError::Config(e.to_string()))?;
        Ok(())
    }

    pub fn stats(&self) -> Result<StoreStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        let count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", self.table_name),
                [],
                |row| row.get(0),
            )
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        Ok(StoreStats {
            total_vectors: count as usize,
            total_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    pub fn clear(&self) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        conn.execute(&format!("DELETE FROM {}", self.table_name), [])
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        conn.execute(&format!("DELETE FROM {}_fts", self.table_name), [])
            .map_err(|e| OpenClawError::Config(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl VectorStore for SqliteStore {
    async fn upsert(&self, item: VectorItem) -> Result<()> {
        self.upsert(item)
    }
    async fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize> {
        self.upsert_batch(items)
    }
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        self.vector_search(&query)
    }
    async fn get(&self, id: &str) -> Result<Option<VectorItem>> {
        self.get(id)
    }
    async fn delete(&self, id: &str) -> Result<()> {
        self.delete(id)
    }
    async fn delete_by_filter(&self, _filter: Filter) -> Result<usize> {
        Err(OpenClawError::Config("not implemented".to_string()))
    }
    async fn stats(&self) -> Result<StoreStats> {
        self.stats()
    }
    async fn clear(&self) -> Result<()> {
        self.clear()
    }
}

fn serialize_vector(vector: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vector.len() * 4);
    for &f in vector {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

fn deserialize_vector(blob: &[u8]) -> Vec<f32> {
    let f32_count = blob.len() / 4;
    let mut vector = vec![0.0f32; f32_count];
    for i in 0..f32_count {
        let bytes: [u8; 4] = [
            blob[i * 4],
            blob[i * 4 + 1],
            blob[i * 4 + 2],
            blob[i * 4 + 3],
        ];
        vector[i] = f32::from_le_bytes(bytes);
    }
    vector
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_serialize_deserialize() {
        let vector = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let blob = serialize_vector(&vector);
        let restored = deserialize_vector(&blob);
        assert_eq!(vector, restored);
    }
}
