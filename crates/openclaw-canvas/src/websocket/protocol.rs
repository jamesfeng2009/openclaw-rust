use crate::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollabMessage {
    Join { user: UserInfo },
    Leave,
    CursorMove { user_id: UserId, cursor: UserCursor },
    ElementAdd { element: Element },
    ElementUpdate { id: String, updates: ElementUpdate },
    ElementDelete { id: String },
    SyncRequest,
    StateSync { state: CanvasState },
    UserJoined { user: UserInfo },
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
