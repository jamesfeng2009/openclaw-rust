pub mod protocol {
    use crate::types::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "type")]
    pub enum CollabMessage {
        Join { user: crate::collaboration::UserInfo },
        Leave,
        CursorMove { user_id: UserId, cursor: UserCursor },
        ElementAdd { element: Element },
        ElementUpdate { id: String, updates: ElementUpdate },
        ElementDelete { id: String },
        SyncRequest,
        StateSync { state: CanvasState },
        UserJoined { user: crate::collaboration::UserInfo },
        UserLeft { user_id: UserId },
        ElementAdded { element: Element },
        ElementUpdated { id: String, updates: ElementUpdate },
        ElementDeleted { id: String },
        Error { code: String, message: String },
    }

    impl CollabMessage {
        pub fn to_bytes(&self) -> Vec<u8> {
            serde_json::to_vec(self).unwrap_or_default()
        }

        pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
            serde_json::from_slice(bytes).ok()
        }
    }
}

pub mod client {
    use crate::types::*;
    use tokio::sync::broadcast;

    pub struct CollabClient {
        pub canvas_id: CanvasId,
        pub user_id: UserId,
    }

    impl CollabClient {
        pub async fn connect(_url: &str, canvas_id: CanvasId, user_id: UserId) -> Result<Self, ClientError> {
            Ok(Self { canvas_id, user_id })
        }

        pub async fn send_cursor(&self, _cursor: UserCursor) -> Result<(), ClientError> {
            Ok(())
        }

        pub async fn send_element_added(&self, _element: Element) -> Result<(), ClientError> {
            Ok(())
        }

        pub async fn send_element_updated(&self, _id: &str, _updates: ElementUpdate) -> Result<(), ClientError> {
            Ok(())
        }

        pub async fn send_element_deleted(&self, _id: &str) -> Result<(), ClientError> {
            Ok(())
        }

        pub fn subscribe(&self) -> broadcast::Receiver<crate::collaboration::CollabEvent> {
            let (tx, _) = broadcast::channel(1024);
            tx.subscribe()
        }

        pub async fn disconnect(&self) -> Result<(), ClientError> {
            Ok(())
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ClientError {
        #[error("Connection failed: {0}")]
        ConnectionFailed(String),
        #[error("Send failed: {0}")]
        SendFailed(String),
    }
}

pub mod server {
    use crate::types::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    pub struct CollabServer {
        rooms: Arc<RwLock<HashMap<CanvasId, ()>>>,
        port: u16,
    }

    impl CollabServer {
        pub fn new(port: u16) -> Self {
            Self {
                rooms: Arc::new(RwLock::new(HashMap::new())),
                port,
            }
        }

        pub async fn start(&self) -> Result<(), ServerError> {
            tracing::info!("CollabServer starting on port {}", self.port);
            Ok(())
        }

        pub async fn get_room_clients(&self, _canvas_id: &CanvasId) -> usize {
            0
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ServerError {
        #[error("Bind failed: {0}")]
        BindFailed(String),
    }
}
