//! 实时协作模块

use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{debug, info};

/// 协作事件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum CollabEvent {
    /// 用户加入画布
    UserJoined { canvas_id: CanvasId, user: UserInfo },
    /// 用户离开画布
    UserLeft {
        canvas_id: CanvasId,
        user_id: UserId,
    },
    /// 光标移动
    CursorMove {
        canvas_id: CanvasId,
        cursor: UserCursor,
    },
    /// 元素添加
    ElementAdded {
        canvas_id: CanvasId,
        element: Element,
    },
    /// 元素更新
    ElementUpdated {
        canvas_id: CanvasId,
        element_id: String,
        updates: ElementUpdate,
    },
    /// 元素删除
    ElementDeleted {
        canvas_id: CanvasId,
        element_id: String,
    },
    /// 视口变化
    ViewportChanged {
        canvas_id: CanvasId,
        user_id: UserId,
        viewport: Viewport,
    },
    /// 图层操作
    LayerAdded { canvas_id: CanvasId, layer: Layer },
    LayerDeleted {
        canvas_id: CanvasId,
        layer_id: String,
    },
}

/// 用户信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    pub color: Color,
    pub avatar_url: Option<String>,
}

/// 协作会话
pub struct CollabSession {
    pub canvas_id: CanvasId,
    pub users: Arc<RwLock<HashMap<UserId, UserInfo>>>,
    pub cursors: Arc<RwLock<HashMap<UserId, UserCursor>>>,
    event_sender: broadcast::Sender<CollabEvent>,
}

impl CollabSession {
    /// 创建新的协作会话
    pub fn new(canvas_id: CanvasId) -> Self {
        let (event_sender, _) = broadcast::channel(1024);
        Self {
            canvas_id,
            users: Arc::new(RwLock::new(HashMap::new())),
            cursors: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
        }
    }

    /// 用户加入
    pub async fn join(&self, user: UserInfo) -> broadcast::Receiver<CollabEvent> {
        let user_id = user.id.clone();

        // 添加用户
        {
            let mut users = self.users.write().await;
            users.insert(user_id.clone(), user.clone());
        }

        // 初始化光标
        {
            let mut cursors = self.cursors.write().await;
            cursors.insert(
                user_id.clone(),
                UserCursor {
                    user_id: user_id.clone(),
                    position: Point::new(0.0, 0.0),
                    color: user.color,
                    name: user.name.clone(),
                    tool: Tool::default(),
                },
            );
        }

        // 广播加入事件
        let event = CollabEvent::UserJoined {
            canvas_id: self.canvas_id.clone(),
            user,
        };
        let _ = self.event_sender.send(event);

        info!("用户 {} 加入画布 {}", user_id, self.canvas_id);

        self.event_sender.subscribe()
    }

    /// 用户离开
    pub async fn leave(&self, user_id: &UserId) {
        {
            let mut users = self.users.write().await;
            users.remove(user_id);
        }

        {
            let mut cursors = self.cursors.write().await;
            cursors.remove(user_id);
        }

        let event = CollabEvent::UserLeft {
            canvas_id: self.canvas_id.clone(),
            user_id: user_id.clone(),
        };
        let _ = self.event_sender.send(event);

        info!("用户 {} 离开画布 {}", user_id, self.canvas_id);
    }

    /// 更新光标位置
    pub async fn update_cursor(&self, cursor: UserCursor) {
        let user_id = cursor.user_id.clone();

        {
            let mut cursors = self.cursors.write().await;
            cursors.insert(user_id.clone(), cursor.clone());
        }

        let event = CollabEvent::CursorMove {
            canvas_id: self.canvas_id.clone(),
            cursor,
        };
        let _ = self.event_sender.send(event);
    }

    /// 广播元素添加
    pub fn broadcast_element_added(&self, element: Element) {
        let event = CollabEvent::ElementAdded {
            canvas_id: self.canvas_id.clone(),
            element,
        };
        let _ = self.event_sender.send(event);
    }

    /// 广播元素更新
    pub fn broadcast_element_updated(&self, element_id: String, updates: ElementUpdate) {
        let event = CollabEvent::ElementUpdated {
            canvas_id: self.canvas_id.clone(),
            element_id,
            updates,
        };
        let _ = self.event_sender.send(event);
    }

    /// 广播元素删除
    pub fn broadcast_element_deleted(&self, element_id: String) {
        let event = CollabEvent::ElementDeleted {
            canvas_id: self.canvas_id.clone(),
            element_id,
        };
        let _ = self.event_sender.send(event);
    }

    /// 获取所有用户
    pub async fn get_users(&self) -> Vec<UserInfo> {
        let users = self.users.read().await;
        users.values().cloned().collect()
    }

    /// 获取所有光标
    pub async fn get_cursors(&self) -> Vec<UserCursor> {
        let cursors = self.cursors.read().await;
        cursors.values().cloned().collect()
    }

    /// 获取用户数量
    pub async fn user_count(&self) -> usize {
        let users = self.users.read().await;
        users.len()
    }
}

/// 协作管理器
pub struct CollabManager {
    sessions: Arc<RwLock<HashMap<CanvasId, Arc<CollabSession>>>>,
}

impl CollabManager {
    /// 创建新的协作管理器
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取或创建会话
    pub async fn get_or_create_session(&self, canvas_id: CanvasId) -> Arc<CollabSession> {
        let mut sessions = self.sessions.write().await;

        sessions
            .entry(canvas_id.clone())
            .or_insert_with(|| {
                debug!("创建协作会话: {}", canvas_id);
                Arc::new(CollabSession::new(canvas_id))
            })
            .clone()
    }

    /// 获取会话
    pub async fn get_session(&self, canvas_id: &CanvasId) -> Option<Arc<CollabSession>> {
        let sessions = self.sessions.read().await;
        sessions.get(canvas_id).cloned()
    }

    /// 移除会话（当没有用户时）
    pub async fn cleanup_session(&self, canvas_id: &CanvasId) {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.get(canvas_id)
            && session.user_count().await == 0 {
                sessions.remove(canvas_id);
                info!("清理协作会话: {}", canvas_id);
            }
    }

    /// 列出所有活跃会话
    pub async fn list_active_sessions(&self) -> Vec<(CanvasId, usize)> {
        let sessions = self.sessions.read().await;
        let mut result = Vec::new();

        for (canvas_id, session) in sessions.iter() {
            result.push((canvas_id.clone(), session.user_count().await));
        }

        result
    }
}

impl Default for CollabManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket 消息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// 加入画布
    JoinCanvas { canvas_id: CanvasId, user: UserInfo },
    /// 离开画布
    LeaveCanvas { canvas_id: CanvasId },
    /// 光标移动
    CursorMove { position: Point, tool: Tool },
    /// 绘图操作
    DrawAction { action: super::draw::DrawAction },
    /// 视口变化
    ViewportChange { viewport: Viewport },
    /// 同步请求
    SyncRequest,
    /// 同步响应
    SyncResponse {
        canvas_state: super::types::CanvasState,
        users: Vec<UserInfo>,
        cursors: Vec<UserCursor>,
    },
}

/// 用户颜色生成器
pub struct UserColorGenerator {
    colors: Vec<Color>,
    index: std::sync::atomic::AtomicUsize,
}

impl UserColorGenerator {
    pub fn new() -> Self {
        Self {
            colors: vec![
                Color::new(255, 107, 107, 255), // Red
                Color::new(78, 205, 196, 255),  // Teal
                Color::new(255, 230, 109, 255), // Yellow
                Color::new(170, 111, 255, 255), // Purple
                Color::new(107, 185, 255, 255), // Blue
                Color::new(255, 146, 76, 255),  // Orange
                Color::new(129, 199, 132, 255), // Green
                Color::new(255, 128, 171, 255), // Pink
            ],
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn next(&self) -> Color {
        let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.colors[idx % self.colors.len()]
    }
}

impl Default for UserColorGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_info_creation() {
        let info = UserInfo {
            id: "user1".to_string(),
            name: "Test User".to_string(),
            color: Color::new(255, 0, 0, 255),
            avatar_url: None,
        };
        assert_eq!(info.name, "Test User");
    }

    #[test]
    fn test_collab_session_creation() {
        let session = CollabSession::new("canvas1".to_string());
        assert_eq!(session.canvas_id, "canvas1".to_string());
    }

    #[test]
    fn test_user_color_generator() {
        let generator = UserColorGenerator::new();
        let color1 = generator.next();
        let color2 = generator.next();
        assert_ne!(color1, color2);
    }
}
