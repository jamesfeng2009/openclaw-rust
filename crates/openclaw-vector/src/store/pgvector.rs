use async_trait::async_trait;
use sqlx::Row;
use sqlx::postgres::{PgPool, PgRow};

use crate::VectorStore;
use crate::types::{Filter, SearchQuery, SearchResult, StoreStats, VectorItem};
use openclaw_core::{OpenClawError, Result};

pub struct PgVectorStore {
    pool: PgPool,
    table_name: String,
    dimension: usize,
}

impl PgVectorStore {
    pub async fn new(connection_string: &str, table_name: &str, dimension: usize) -> Result<Self> {
        let pool = PgPool::connect(connection_string).await.map_err(|e| {
            OpenClawError::Config(format!("Failed to connect to PostgreSQL: {}", e))
        })?;

        let store = Self {
            pool,
            table_name: table_name.to_string(),
            dimension,
        };

        store.initialize_table().await?;

        Ok(store)
    }

    async fn initialize_table(&self) -> Result<()> {
        let create_table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                vector VECTOR({}) NOT NULL,
                content TEXT,
                payload JSONB,
                created_at TIMESTAMP DEFAULT NOW()
            )
            "#,
            self.table_name, self.dimension
        );

        sqlx::query(&create_table_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create table: {}", e)))?;

        let create_index_sql = format!(
            r#"
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_class c 
                    JOIN pg_namespace n ON n.oid = c.relnamespace 
                    WHERE c.relname = '{}__vector_idx' AND n.nspname = 'public'
                ) THEN
                    CREATE INDEX {}__vector_idx 
                    ON {} USING ivfflat (vector vector_cosine_ops)
                    WITH (lists = 100);
                END IF;
            END
            $$
            "#,
            self.table_name, self.table_name, self.table_name
        );

        sqlx::query(&create_index_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create index: {}", e)))?;

        let create_gin_index_sql = format!(
            r#"
            DO $$
            BEGIN
                IF NOT EXISTS (
                    SELECT 1 FROM pg_class c 
                    JOIN pg_namespace n ON n.oid = c.relnamespace 
                    WHERE c.relname = '{}__payload_idx' AND n.nspname = 'public'
                ) THEN
                    CREATE INDEX {}__payload_idx 
                    ON {} USING GIN (payload);
                END IF;
            END
            $$
            "#,
            self.table_name, self.table_name, self.table_name
        );

        sqlx::query(&create_gin_index_sql)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create GIN index: {}", e)))?;

        Ok(())
    }

    fn vector_to_string(vector: &[f32]) -> String {
        format!(
            "[{}]",
            vector
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        )
    }

    fn string_to_vector(s: &str) -> Vec<f32> {
        let s = s.trim();
        if s.starts_with('[') && s.ends_with(']') {
            s[1..s.len() - 1]
                .split(',')
                .filter_map(|s| s.trim().parse::<f32>().ok())
                .collect()
        } else {
            Vec::new()
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorStore for PgVectorStore {
    async fn upsert(&self, item: VectorItem) -> Result<()> {
        let vector_str = Self::vector_to_string(&item.vector);
        let payload_json = serde_json::to_string(&item.payload)
            .map_err(|e| OpenClawError::Config(format!("Failed to serialize payload: {}", e)))?;

        let content = item
            .payload
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let sql = format!(
            "INSERT INTO {} (id, vector, content, payload) VALUES ($1, $2::vector, $3, $4::jsonb) 
             ON CONFLICT (id) DO UPDATE SET vector = EXCLUDED.vector::vector, content = EXCLUDED.content, payload = EXCLUDED.payload::jsonb",
            self.table_name
        );

        sqlx::query(&sql)
            .bind(&item.id)
            .bind(&vector_str)
            .bind(&content)
            .bind(&payload_json)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to upsert: {}", e)))?;

        Ok(())
    }

    async fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize> {
        let count = items.len();

        for item in items {
            self.upsert(item).await?;
        }

        Ok(count)
    }

    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        let vector_str = Self::vector_to_string(&query.vector);

        let sql = format!(
            "SELECT id, vector::text, content, payload FROM {} ORDER BY vector <=> $1::vector LIMIT $2",
            self.table_name
        );

        let rows: Vec<PgRow> = sqlx::query(&sql)
            .bind(&vector_str)
            .bind(query.limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to search: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let id: String = row.get(0);
            let vector_str: String = row.get(1);
            let _content: Option<String> = row.get(2);
            let payload_json: Option<String> = row.get(3);

            let stored_vector = Self::string_to_vector(&vector_str);
            let score = Self::cosine_similarity(&query.vector, &stored_vector);

            if let Some(min_score) = query.min_score
                && score < min_score
            {
                continue;
            }

            let payload: serde_json::Value = payload_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null);

            results.push(SearchResult { id, score, payload });
        }

        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<VectorItem>> {
        let sql = format!(
            "SELECT id, vector::text, content, payload FROM {} WHERE id = $1",
            self.table_name
        );

        let rows: Vec<PgRow> = sqlx::query(&sql)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to get: {}", e)))?;

        if let Some(row) = rows.into_iter().next() {
            let id: String = row.get(0);
            let vector_str: String = row.get(1);
            let _content: Option<String> = row.get(2);
            let payload_json: Option<String> = row.get(3);

            let vector = Self::string_to_vector(&vector_str);
            let payload: serde_json::Value = payload_json
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::Value::Null);

            Ok(Some(VectorItem {
                id,
                vector,
                payload,
                created_at: chrono::Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let sql = format!("DELETE FROM {} WHERE id = $1", self.table_name);

        sqlx::query(&sql)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to delete: {}", e)))?;

        Ok(())
    }

    async fn delete_by_filter(&self, filter: Filter) -> Result<usize> {
        let condition = filter.to_sql_condition();
        let sql = format!("DELETE FROM {} WHERE {}", self.table_name, condition);

        let result = sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to delete by filter: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }

    async fn stats(&self) -> Result<StoreStats> {
        let sql = format!("SELECT COUNT(*) FROM {}", self.table_name);

        let row: PgRow = sqlx::query(&sql)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to get stats: {}", e)))?;

        let count: i64 = row.get(0);

        Ok(StoreStats {
            total_vectors: count as usize,
            total_size_bytes: 0,
            last_updated: chrono::Utc::now(),
        })
    }

    async fn clear(&self) -> Result<()> {
        let sql = format!("DELETE FROM {}", self.table_name);

        sqlx::query(&sql)
            .execute(&self.pool)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to clear: {}", e)))?;

        Ok(())
    }
}
