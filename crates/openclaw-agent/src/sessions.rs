//! Sessions 工具 - 完整会话管理
//!
//! 提供会话的完整生命周期管理：
//! - 会话创建、恢复、关闭
//! - 会话历史
//! - 会话元数据
//! - 会话状态持久化

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use openclaw_core::session::SessionScope;

use crate::types::{AgentId, PersonaId};

/// Session 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话 ID
    pub id: Uuid,
    /// 会话名称
    pub name: String,
    /// 会话作用域
    pub scope: SessionScope,
    /// 关联的 Agent ID
    pub agent_id: AgentId,
    /// Persona ID - 用于记忆隔离
    pub persona_id: Option<PersonaId>,
    /// 通道类型
    pub channel_type: Option<String>,
    /// 账户 ID
    pub account_id: Option<String>,
    /// 对端 ID
    pub peer_id: Option<String>,
    /// 会话状态
    pub state: SessionState,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 消息数量
    pub message_count: usize,
    /// Token 数量
    pub token_count: u64,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 历史消息 (简化版，仅保留摘要)
    pub history_summary: Option<String>,
}

/// 会话状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SessionState {
    #[default]
    Active,
    Idle,
    Paused,
    Closed,
}

impl Session {
    pub fn new(name: impl Into<String>, agent_id: AgentId, scope: SessionScope) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            scope,
            agent_id,
            persona_id: None,
            channel_type: None,
            account_id: None,
            peer_id: None,
            state: SessionState::Active,
            created_at: now,
            updated_at: now,
            last_active_at: now,
            message_count: 0,
            token_count: 0,
            metadata: HashMap::new(),
            system_prompt: None,
            history_summary: None,
        }
    }

    pub fn with_persona(mut self, persona_id: PersonaId) -> Self {
        self.persona_id = Some(persona_id);
        self
    }

    pub fn with_channel(mut self, channel_type: impl Into<String>) -> Self {
        self.channel_type = Some(channel_type.into());
        self
    }

    pub fn with_peer(mut self, peer_id: impl Into<String>) -> Self {
        self.peer_id = Some(peer_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn update_activity(&mut self) {
        self.last_active_at = Utc::now();
        self.updated_at = Utc::now();
        if self.state == SessionState::Idle {
            self.state = SessionState::Active;
        }
    }

    pub fn add_message(&mut self, tokens: u64) {
        self.message_count += 1;
        self.token_count += tokens;
        self.update_activity();
    }

    pub fn pause(&mut self) {
        self.state = SessionState::Paused;
        self.updated_at = Utc::now();
    }

    pub fn resume(&mut self) {
        self.state = SessionState::Active;
        self.update_activity();
    }

    pub fn close(&mut self) {
        self.state = SessionState::Closed;
        self.updated_at = Utc::now();
    }

    pub fn set_idle(&mut self) {
        self.state = SessionState::Idle;
        self.updated_at = Utc::now();
    }

    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    pub fn is_closed(&self) -> bool {
        self.state == SessionState::Closed
    }

    pub fn key(&self) -> String {
        match self.scope {
            SessionScope::Main => "main".to_string(),
            SessionScope::PerPeer => {
                format!(
                    "{}:{}",
                    self.channel_type.as_deref().unwrap_or("unknown"),
                    self.peer_id.as_deref().unwrap_or("unknown")
                )
            }
            SessionScope::PerChannelPeer => {
                format!(
                    "{}:{}:{}",
                    self.channel_type.as_deref().unwrap_or("unknown"),
                    self.account_id.as_deref().unwrap_or("unknown"),
                    self.peer_id.as_deref().unwrap_or("unknown")
                )
            }
            SessionScope::PerAccountChannelPeer => {
                format!(
                    "{}:{}:{}",
                    self.channel_type.as_deref().unwrap_or("unknown"),
                    self.account_id.as_deref().unwrap_or("unknown"),
                    self.peer_id.as_deref().unwrap_or("unknown")
                )
            }
        }
    }
}

/// 会话存储后端
#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn save(&self, session: &Session) -> crate::Result<()>;
    async fn load(&self, id: &Uuid) -> crate::Result<Option<Session>>;
    async fn delete(&self, id: &Uuid) -> crate::Result<()>;
    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        state: Option<SessionState>,
    ) -> crate::Result<Vec<Session>>;
    async fn find_by_key(&self, key: &str, agent_id: &AgentId) -> crate::Result<Option<Session>>;
    async fn save_messages(&self, session_id: &Uuid, messages: &[openclaw_core::Message]) -> crate::Result<()>;
    async fn load_messages(&self, session_id: &Uuid) -> crate::Result<Vec<openclaw_core::Message>>;
    async fn delete_messages(&self, session_id: &Uuid) -> crate::Result<()>;
}

/// 内存会话存储
pub struct MemorySessionStorage {
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    key_index: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl MemorySessionStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            key_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemorySessionStorage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SqliteSessionStorage {
    db_path: PathBuf,
}

impl SqliteSessionStorage {
    pub fn new(db_path: impl Into<PathBuf>) -> Self {
        Self {
            db_path: db_path.into(),
        }
    }

    fn get_connection(&self) -> crate::Result<rusqlite::Connection> {
        rusqlite::Connection::open(&self.db_path)
            .map_err(|e| crate::OpenClawError::Session(format!("SQLite error: {}", e)))
    }

    fn init_tables(conn: &rusqlite::Connection) -> crate::Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                scope TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                persona_id TEXT,
                channel_type TEXT,
                account_id TEXT,
                peer_id TEXT,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_active_at TEXT NOT NULL,
                message_count INTEGER NOT NULL,
                token_count INTEGER NOT NULL,
                metadata TEXT,
                system_prompt TEXT,
                history_summary TEXT
            )",
            [],
        ).map_err(|e| crate::OpenClawError::Session(format!("Failed to create table: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_agent_id ON sessions(agent_id)",
            [],
        ).map_err(|e| crate::OpenClawError::Session(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS session_messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                token_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            )",
            [],
        ).map_err(|e| crate::OpenClawError::Session(format!("Failed to create messages table: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_messages_session_id ON session_messages(session_id)",
            [],
        ).map_err(|e| crate::OpenClawError::Session(format!("Failed to create messages index: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl SessionStorage for SqliteSessionStorage {
    async fn save(&self, session: &Session) -> crate::Result<()> {
        let conn = self.get_connection()?;
        Self::init_tables(&conn)?;

        let id = session.id.to_string();
        let scope = session_scope_to_string(&session.scope);
        let agent_id = session.agent_id.clone();
        let state = session_state_to_string(&session.state);
        let created_at = session.created_at.to_rfc3339();
        let updated_at = session.updated_at.to_rfc3339();
        let last_active_at = session.last_active_at.to_rfc3339();
        let metadata = serde_json::to_string(&session.metadata).unwrap_or_default();
        let persona_id = session.persona_id.as_ref().map(|p| p.to_string());
        let channel_type = session.channel_type.clone();
        let account_id = session.account_id.clone();
        let peer_id = session.peer_id.clone();
        let system_prompt = session.system_prompt.clone();
        let history_summary = session.history_summary.clone();

        conn.execute(
            "INSERT OR REPLACE INTO sessions 
             (id, name, scope, agent_id, persona_id, channel_type, account_id, peer_id, state,
              created_at, updated_at, last_active_at, message_count, token_count, metadata,
              system_prompt, history_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![
                id, session.name, scope, agent_id, persona_id, channel_type, account_id, peer_id,
                state, created_at, updated_at, last_active_at, session.message_count as i64,
                session.token_count as i64, metadata, system_prompt, history_summary
            ],
        ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;

        Ok(())
    }

    async fn load(&self, id: &Uuid) -> crate::Result<Option<Session>> {
        let conn = self.get_connection()?;
        let id_str = id.to_string();

        let mut stmt = conn.prepare(
            "SELECT * FROM sessions WHERE id = ?1"
        ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;

        let result = stmt.query_row([&id_str], |row| {
            Ok(Session {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                name: row.get(1)?,
                scope: session_scope_from_string(&row.get::<_, String>(2)?),
                agent_id: row.get(3)?,
                persona_id: row.get::<_, Option<String>>(4)?.map(|s| s.parse().unwrap_or_default()),
                channel_type: row.get(5)?,
                account_id: row.get(6)?,
                peer_id: row.get(7)?,
                state: session_state_from_string(&row.get::<_, String>(8)?),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_active_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                message_count: row.get::<_, i64>(12)? as usize,
                token_count: row.get::<_, i64>(13)? as u64,
                metadata: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                system_prompt: row.get(15)?,
                history_summary: row.get(16)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::OpenClawError::Session(e.to_string())),
        }
    }

    async fn delete(&self, id: &Uuid) -> crate::Result<()> {
        let conn = self.get_connection()?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", [id.to_string()])
            .map_err(|e| crate::OpenClawError::Session(e.to_string()))?;
        Ok(())
    }

    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        state: Option<SessionState>,
    ) -> crate::Result<Vec<Session>> {
        let conn = self.get_connection()?;
        let mut sql = String::from("SELECT * FROM sessions WHERE 1=1");
        let mut params: Vec<String> = Vec::new();

        if let Some(aid) = agent_id {
            sql.push_str(" AND agent_id = ?");
            params.push(aid.to_string());
        }
        if let Some(ref s) = state {
            sql.push_str(" AND state = ?");
            params.push(session_state_to_string(s).to_string());
        }

        let mut stmt = conn.prepare(&sql)
            .map_err(|e| crate::OpenClawError::Session(e.to_string()))?;

        let sessions = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(Session {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                name: row.get(1)?,
                scope: session_scope_from_string(&row.get::<_, String>(2)?),
                agent_id: row.get(3)?,
                persona_id: row.get::<_, Option<String>>(4)?.map(|s| s.parse().unwrap_or_default()),
                channel_type: row.get(5)?,
                account_id: row.get(6)?,
                peer_id: row.get(7)?,
                state: session_state_from_string(&row.get::<_, String>(8)?),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_active_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                message_count: row.get::<_, i64>(12)? as usize,
                token_count: row.get::<_, i64>(13)? as u64,
                metadata: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                system_prompt: row.get(15)?,
                history_summary: row.get(16)?,
            })
        }).map_err(|e| crate::OpenClawError::Session(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(sessions)
    }

    async fn find_by_key(&self, key: &str, agent_id: &AgentId) -> crate::Result<Option<Session>> {
        let conn = self.get_connection()?;
        
        let mut stmt = conn.prepare(
            "SELECT * FROM sessions WHERE agent_id = ?1 AND (name = ?2 OR peer_id = ?2)"
        ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![agent_id.to_string(), key], |row| {
            Ok(Session {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                name: row.get(1)?,
                scope: session_scope_from_string(&row.get::<_, String>(2)?),
                agent_id: row.get(3)?,
                persona_id: row.get::<_, Option<String>>(4)?.map(|s| s.parse().unwrap_or_default()),
                channel_type: row.get(5)?,
                account_id: row.get(6)?,
                peer_id: row.get(7)?,
                state: session_state_from_string(&row.get::<_, String>(8)?),
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_active_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                message_count: row.get::<_, i64>(12)? as usize,
                token_count: row.get::<_, i64>(13)? as u64,
                metadata: serde_json::from_str(&row.get::<_, String>(14)?).unwrap_or_default(),
                system_prompt: row.get(15)?,
                history_summary: row.get(16)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(crate::OpenClawError::Session(e.to_string())),
        }
    }

    async fn save_messages(&self, session_id: &Uuid, messages: &[openclaw_core::Message]) -> crate::Result<()> {
        let conn = self.get_connection()?;
        Self::init_tables(&conn)?;

        for msg in messages {
            let msg_id = msg.id.to_string();
            let role = format!("{:?}", msg.role);
            let content = serde_json::to_string(&msg.content).unwrap_or_default();
            let created_at = msg.created_at.to_rfc3339();
            let token_count = msg.metadata.token_count.unwrap_or(0) as i64;
            let metadata = serde_json::to_string(&msg.metadata).unwrap_or_default();

            conn.execute(
                "INSERT OR REPLACE INTO session_messages 
                 (id, session_id, role, content, created_at, token_count, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![msg_id, session_id.to_string(), role, content, created_at, token_count, metadata],
            ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;
        }

        Ok(())
    }

    async fn load_messages(&self, session_id: &Uuid) -> crate::Result<Vec<openclaw_core::Message>> {
        let conn = self.get_connection()?;

        let mut stmt = conn.prepare(
            "SELECT id, role, content, created_at, token_count, metadata 
             FROM session_messages WHERE session_id = ?1 ORDER BY created_at ASC"
        ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;

        let messages = stmt.query_map([session_id.to_string()], |row| {
            use openclaw_core::message::Role as MsgRole;
            use openclaw_core::Content;
            use openclaw_core::message::MessageMetadata;
            
            let role_str: String = row.get(1)?;
            let content_str: String = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            
            let role = match role_str.as_str() {
                "User" => MsgRole::User,
                "Assistant" => MsgRole::Assistant,
                "System" => MsgRole::System,
                "Tool" => MsgRole::Tool,
                _ => MsgRole::User,
            };
            
            let content: Vec<Content> = serde_json::from_str(&content_str).unwrap_or_default();
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());
            let metadata_str: String = row.get(5)?;
            let metadata: MessageMetadata = serde_json::from_str(&metadata_str).unwrap_or_default();

            Ok(openclaw_core::Message {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_default(),
                role,
                content,
                created_at,
                metadata,
            })
        }).map_err(|e| crate::OpenClawError::Session(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

        Ok(messages)
    }

    async fn delete_messages(&self, session_id: &Uuid) -> crate::Result<()> {
        let conn = self.get_connection()?;
        conn.execute(
            "DELETE FROM session_messages WHERE session_id = ?1",
            [session_id.to_string()],
        ).map_err(|e| crate::OpenClawError::Session(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl SessionStorage for MemorySessionStorage {
    async fn save(&self, session: &Session) -> crate::Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id, session.clone());

        let key = session.key();
        let mut key_index = self.key_index.write().await;
        key_index.insert(format!("{}:{}", session.agent_id, key), session.id);

        Ok(())
    }

    async fn load(&self, id: &Uuid) -> crate::Result<Option<Session>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(id).cloned())
    }

    async fn delete(&self, id: &Uuid) -> crate::Result<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.remove(id) {
            let key = session.key();
            let mut key_index = self.key_index.write().await;
            key_index.remove(&format!("{}:{}", session.agent_id, key));
        }

        Ok(())
    }

    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        state: Option<SessionState>,
    ) -> crate::Result<Vec<Session>> {
        let sessions = self.sessions.read().await;
        let mut result: Vec<Session> = sessions.values().cloned().collect();

        if let Some(aid) = agent_id {
            result.retain(|s| &s.agent_id == aid);
        }

        if let Some(st) = state {
            result.retain(|s| s.state == st);
        }

        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(result)
    }

    async fn find_by_key(&self, key: &str, agent_id: &AgentId) -> crate::Result<Option<Session>> {
        let key_index = self.key_index.read().await;
        let lookup_key = format!("{}:{}", agent_id, key);

        if let Some(id) = key_index.get(&lookup_key) {
            let id = *id;
            drop(key_index);
            return self.load(&id).await;
        }

        Ok(None)
    }

    async fn save_messages(&self, _session_id: &Uuid, _messages: &[openclaw_core::Message]) -> crate::Result<()> {
        Ok(())
    }

    async fn load_messages(&self, _session_id: &Uuid) -> crate::Result<Vec<openclaw_core::Message>> {
        Ok(Vec::new())
    }

    async fn delete_messages(&self, _session_id: &Uuid) -> crate::Result<()> {
        Ok(())
    }
}

/// 会话管理器
pub struct SessionManager {
    storage: Arc<dyn SessionStorage>,
    active_sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    config: SessionConfig,
}

/// 会话配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// 最大活跃会话数
    pub max_active_sessions: usize,
    /// 会话空闲超时 (秒)
    pub idle_timeout: u64,
    /// 会话最大消息数
    pub max_messages: usize,
    /// 会话最大 token 数
    pub max_tokens: u64,
    /// 是否自动保存
    pub auto_save: bool,
    /// 会话历史保留天数
    pub history_retention_days: u32,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_active_sessions: 100,
            idle_timeout: 3600,
            max_messages: 10000,
            max_tokens: 10_000_000,
            auto_save: true,
            history_retention_days: 30,
        }
    }
}

impl SessionManager {
    pub fn new(storage: Arc<dyn SessionStorage>) -> Self {
        Self {
            storage,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config: SessionConfig::default(),
        }
    }

    pub fn with_config(storage: Arc<dyn SessionStorage>, config: SessionConfig) -> Self {
        Self {
            storage,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// 创建新会话
    pub async fn create_session(
        &self,
        name: impl Into<String>,
        agent_id: AgentId,
        scope: SessionScope,
        channel_type: Option<String>,
        peer_id: Option<String>,
    ) -> crate::Result<Session> {
        let agent_id_clone = agent_id.clone();
        let mut session = Session::new(name, agent_id_clone, scope);
        session.channel_type = channel_type;
        session.peer_id = peer_id;

        let key = session.key();

        if let Some(existing) = self.storage.find_by_key(&key, &agent_id).await?
            && existing.is_active()
        {
            return Ok(existing);
        }

        if self.config.auto_save {
            self.storage.save(&session).await?;
        }

        let mut active = self.active_sessions.write().await;
        active.insert(session.id, session.clone());

        Ok(session)
    }

    /// 获取会话
    pub async fn get_session(&self, id: &Uuid) -> crate::Result<Option<Session>> {
        {
            let active = self.active_sessions.read().await;
            if let Some(session) = active.get(id) {
                return Ok(Some(session.clone()));
            }
        }

        self.storage.load(id).await
    }

    /// 获取或创建会话
    pub async fn get_or_create_session(
        &self,
        name: impl Into<String>,
        agent_id: AgentId,
        scope: SessionScope,
        channel_type: Option<String>,
        peer_id: Option<String>,
    ) -> crate::Result<Session> {
        let key = format!(
            "{}:{}:{}",
            session_scope_to_string(&scope),
            channel_type.as_deref().unwrap_or("unknown"),
            peer_id.as_deref().unwrap_or("unknown")
        );

        if let Some(session) = self.storage.find_by_key(&key, &agent_id).await?
            && session.is_active()
        {
            let mut active = self.active_sessions.write().await;
            active.insert(session.id, session.clone());
            return Ok(session);
        }

        self.create_session(name, agent_id, scope, channel_type, peer_id)
            .await
    }

    /// 更新会话
    pub async fn update_session(&self, session: &Session) -> crate::Result<()> {
        if self.config.auto_save {
            self.storage.save(session).await?;
        }

        let mut active = self.active_sessions.write().await;
        active.insert(session.id, session.clone());
        Ok(())
    }

    /// 关闭会话
    pub async fn close_session(&self, id: &Uuid) -> crate::Result<()> {
        let mut session = self
            .get_session(id)
            .await?
            .ok_or_else(|| crate::OpenClawError::Session(format!("Session {} not found", id)))?;

        session.close();
        self.update_session(&session).await?;

        let mut active = self.active_sessions.write().await;
        active.remove(id);

        Ok(())
    }

    /// 列出所有会话
    pub async fn list_sessions(
        &self,
        agent_id: Option<AgentId>,
        state: Option<SessionState>,
    ) -> crate::Result<Vec<Session>> {
        self.storage.list(agent_id.as_ref(), state).await
    }

    /// 获取活跃会话
    pub async fn get_active_sessions(
        &self,
        agent_id: Option<&AgentId>,
    ) -> crate::Result<Vec<Session>> {
        self.storage
            .list(agent_id, Some(SessionState::Active))
            .await
    }

    /// 清理空闲会话
    pub async fn cleanup_idle_sessions(&self) -> crate::Result<usize> {
        let idle_timeout = chrono::Duration::seconds(self.config.idle_timeout as i64);
        let now = Utc::now();
        let mut cleaned = 0;

        let sessions = self.list_sessions(None, Some(SessionState::Idle)).await?;

        for mut session in sessions {
            if now.signed_duration_since(session.last_active_at) > idle_timeout {
                session.close();
                self.storage.save(&session).await?;
                cleaned += 1;
            }
        }

        Ok(cleaned)
    }

    /// 统计会话
    pub async fn get_stats(&self) -> SessionStats {
        let sessions = self.storage.list(None, None).await.unwrap_or_default();

        let total = sessions.len();
        let active = sessions
            .iter()
            .filter(|s| s.state == SessionState::Active)
            .count();
        let idle = sessions
            .iter()
            .filter(|s| s.state == SessionState::Idle)
            .count();
        let closed = sessions
            .iter()
            .filter(|s| s.state == SessionState::Closed)
            .count();

        let total_messages: usize = sessions.iter().map(|s| s.message_count).sum();
        let total_tokens: u64 = sessions.iter().map(|s| s.token_count).sum();

        SessionStats {
            total,
            active,
            idle,
            closed,
            total_messages,
            total_tokens,
        }
    }
}

/// 会话统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub total: usize,
    pub active: usize,
    pub idle: usize,
    pub closed: usize,
    pub total_messages: usize,
    pub total_tokens: u64,
}

/// Session Scope 辅助函数
pub fn session_scope_to_string(scope: &SessionScope) -> &'static str {
    match scope {
        SessionScope::Main => "main",
        SessionScope::PerPeer => "per_peer",
        SessionScope::PerChannelPeer => "per_channel_peer",
        SessionScope::PerAccountChannelPeer => "per_account_channel_peer",
    }
}

pub fn session_scope_from_string(s: &str) -> SessionScope {
    match s {
        "per_peer" => SessionScope::PerPeer,
        "per_channel_peer" => SessionScope::PerChannelPeer,
        "per_account_channel_peer" => SessionScope::PerAccountChannelPeer,
        _ => SessionScope::Main,
    }
}

pub fn session_state_to_string(state: &SessionState) -> &'static str {
    match state {
        SessionState::Active => "active",
        SessionState::Idle => "idle",
        SessionState::Paused => "paused",
        SessionState::Closed => "closed",
    }
}

pub fn session_state_from_string(s: &str) -> SessionState {
    match s {
        "idle" => SessionState::Idle,
        "paused" => SessionState::Paused,
        "closed" => SessionState::Closed,
        _ => SessionState::Active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(storage);

        let session = manager
            .create_session(
                "test-session",
                "agent-1".to_string(),
                SessionScope::PerPeer,
                Some("telegram".to_string()),
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(session.name, "test-session");
        assert_eq!(session.agent_id, "agent-1");
    }

    #[tokio::test]
    async fn test_get_or_create() {
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(storage);

        let session1 = manager
            .get_or_create_session(
                "test",
                "agent-1".to_string(),
                SessionScope::PerPeer,
                Some("telegram".to_string()),
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        let session2 = manager
            .get_or_create_session(
                "test",
                "agent-1".to_string(),
                SessionScope::PerPeer,
                Some("telegram".to_string()),
                Some("user-1".to_string()),
            )
            .await
            .unwrap();

        assert_eq!(session1.id, session2.id);
    }

    #[tokio::test]
    async fn test_close_session() {
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(storage);

        let session = manager
            .create_session(
                "test",
                "agent-1".to_string(),
                SessionScope::Main,
                None,
                None,
            )
            .await
            .unwrap();

        manager.close_session(&session.id).await.unwrap();

        let loaded = manager.get_session(&session.id).await.unwrap().unwrap();
        assert!(loaded.is_closed());
    }
}
