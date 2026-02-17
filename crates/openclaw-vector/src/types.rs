//! 向量存储类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 向量项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorItem {
    pub id: String,
    pub vector: Vec<f32>,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl VectorItem {
    pub fn new(vector: Vec<f32>, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            vector,
            payload,
            created_at: Utc::now(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
}

/// 搜索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 查询向量
    pub vector: Vec<f32>,
    /// 返回数量
    pub limit: usize,
    /// 过滤条件
    pub filter: Option<Filter>,
    /// 最小相似度
    pub min_score: Option<f32>,
}

impl SearchQuery {
    pub fn new(vector: Vec<f32>) -> Self {
        Self {
            vector,
            limit: 10,
            filter: None,
            min_score: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = Some(score);
        self
    }
}

/// 过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub conditions: Vec<FilterCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub field: String,
    pub operator: FilterOperator,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    Contains,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
        }
    }

    pub fn eq(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            conditions: vec![FilterCondition {
                field: field.into(),
                operator: FilterOperator::Eq,
                value,
            }],
        }
    }

    pub fn and(mut self, other: Filter) -> Self {
        self.conditions.extend(other.conditions);
        self
    }

    pub fn to_sql_condition(&self) -> String {
        if self.conditions.is_empty() {
            return "TRUE".to_string();
        }

        let conditions: Vec<String> = self
            .conditions
            .iter()
            .map(|c| {
                let value_str = match &c.value {
                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => format!("'{}'", c.value.to_string().replace('\'', "''")),
                };

                match c.operator {
                    FilterOperator::Eq => format!("{} = {}", c.field, value_str),
                    FilterOperator::Ne => format!("{} != {}", c.field, value_str),
                    FilterOperator::Gt => format!("{} > {}", c.field, value_str),
                    FilterOperator::Gte => format!("{} >= {}", c.field, value_str),
                    FilterOperator::Lt => format!("{} < {}", c.field, value_str),
                    FilterOperator::Lte => format!("{} <= {}", c.field, value_str),
                    FilterOperator::In => {
                        format!("{} = ANY(string_to_array({}, ','))", c.field, value_str)
                    }
                    FilterOperator::Contains => format!("{} LIKE '%{}%'", c.field, c.value),
                }
            })
            .collect();

        conditions.join(" AND ")
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub total_vectors: usize,
    pub total_size_bytes: usize,
    pub last_updated: DateTime<Utc>,
}
