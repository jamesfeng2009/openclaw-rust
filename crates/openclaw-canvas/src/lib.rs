//! OpenClaw Canvas - 实时协作画布模块
//!
//! 提供 A2UI 可视化工作空间和实时协作画布功能

pub mod canvas;
pub mod collaboration;
pub mod draw;
pub mod types;
pub mod websocket;

pub use canvas::{CanvasInfo, CanvasManager, CanvasOps};
pub use collaboration::{CollabEvent, CollabManager, CollabSession, UserInfo, UserColorGenerator, WsMessage};
pub use draw::DrawAction;
pub use types::{CanvasState, Color, Element, ElementUpdate, UserCursor, CanvasId, UserId};
pub use websocket::protocol::CollabMessage;
