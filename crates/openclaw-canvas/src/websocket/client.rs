use crate::types::*;
use crate::websocket::protocol::CollabMessage;
use futures::stream::StreamExt;
use futures::SinkExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct CollabClient {
    canvas_id: CanvasId,
    user_id: UserId,
    ws: Arc<RwLock<Option<tokio_tungstenite::WebSocketStream<TcpStream>>>>,
    event_tx: broadcast::Sender<CollabEvent>,
}

impl CollabClient {
    pub async fn connect(url: &str, canvas_id: CanvasId, user_id: UserId) -> Result<Self, ClientError> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| ClientError::ConnectionFailed(e.to_string()))?;

        let (mut write, mut read) = ws_stream.split();

        let join_msg = CollabMessage::Join {
            user: UserInfo {
                id: user_id.clone(),
                name: "User".to_string(),
                color: Color::default(),
                avatar_url: None,
            },
        };
        
        write
            .send(Message::Text(serde_json::to_string(&join_msg).unwrap()))
            .await
            .map_err(|e| ClientError::SendFailed(e.to_string()))?;

        let (event_tx, _) = broadcast::channel(1024);

        let client = Self {
            canvas_id,
            user_id,
            ws: Arc::new(RwLock::new(None)),
            event_tx,
        };

        let ws_clone = client.ws.clone();
        let event_tx_clone = client.event_tx.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Some(collab_msg) = CollabMessage::from_bytes(text.as_bytes()) {
                        let event = match collab_msg {
                            CollabMessage::UserJoined { user } => {
                                Some(CollabEvent::UserJoined { canvas_id: "".into(), user })
                            }
                            CollabMessage::UserLeft { user_id } => {
                                Some(CollabEvent::UserLeft { canvas_id: "".into(), user_id })
                            }
                            CollabMessage::CursorMove { user_id: _, cursor } => {
                                Some(CollabEvent::CursorMove { canvas_id: "".into(), cursor })
                            }
                            CollabMessage::ElementAdded { element } => {
                                Some(CollabEvent::ElementAdded { canvas_id: "".into(), element })
                            }
                            CollabMessage::ElementUpdated { element_id, updates } => {
                                Some(CollabEvent::ElementUpdated { canvas_id: "".into(), element_id, updates })
                            }
                            CollabMessage::ElementDeleted { element_id } => {
                                Some(CollabEvent::ElementDeleted { canvas_id: "".into(), element_id })
                            }
                            _ => None,
                        };
                        
                        if let Some(evt) = event {
                            let _ = event_tx_clone.send(evt);
                        }
                    }
                }
            }
        });

        *ws_clone.write().await = Some(ws_stream);

        Ok(client)
    }

    pub async fn send_cursor(&self, cursor: UserCursor) -> Result<(), ClientError> {
        let msg = CollabMessage::CursorMove { cursor };
        self.send_message(msg).await
    }

    pub async fn send_element_added(&self, element: Element) -> Result<(), ClientError> {
        let msg = CollabMessage::ElementAdd { element };
        self.send_message(msg).await
    }

    pub async fn send_element_updated(&self, id: &str, updates: ElementUpdate) -> Result<(), ClientError> {
        let msg = CollabMessage::ElementUpdate {
            id: id.to_string(),
            updates,
        };
        self.send_message(msg).await
    }

    pub async fn send_element_deleted(&self, id: &str) -> Result<(), ClientError> {
        let msg = CollabMessage::ElementDelete {
            id: id.to_string(),
        };
        self.send_message(msg).await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CollabEvent> {
        self.event_tx.subscribe()
    }

    pub async fn disconnect(&self) -> Result<(), ClientError> {
        let msg = CollabMessage::Leave;
        let _ = self.send_message(msg).await;
        *self.ws.write().await = None;
        Ok(())
    }

    async fn send_message(&self, msg: CollabMessage) -> Result<(), ClientError> {
        let ws_guard = self.ws.read().await;
        if let Some(ws) = ws_guard.as_ref() {
            let mut ws_write = ws.clone();
            ws_write
                .send(Message::Text(serde_json::to_string(&msg).unwrap()))
                .await
                .map_err(|e| ClientError::SendFailed(e.to_string()))?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
}
