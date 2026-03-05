use crate::types::*;
use crate::websocket::protocol::CollabMessage;
use futures::stream::StreamExt;
use futures::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub struct CollabServer {
    rooms: Arc<RwLock<HashMap<CanvasId, Room>>>,
    port: u16,
}

struct Room {
    canvas_id: CanvasId,
    clients: Arc<RwLock<HashMap<UserId, broadcast::Sender<CollabMessage>>>>,
}

impl CollabServer {
    pub fn new(port: u16) -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            port,
        }
    }

    pub async fn start(&self) -> Result<(), ServerError> {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| ServerError::BindFailed(e.to_string()))?;

        tracing::info!("CollabServer listening on {}", addr);

        loop {
            if let Ok((stream, _peer_addr)) = listener.accept().await {
                let rooms = self.rooms.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(stream, rooms).await {
                        tracing::error!("Connection error: {}", e);
                    }
                });
            }
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        rooms: Arc<RwLock<HashMap<CanvasId, Room>>>,
    ) -> Result<(), ServerError> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| ServerError::HandshakeFailed(e.to_string()))?;

        let (mut ws_write, mut ws_read) = ws_stream.split();

        let (event_tx, _) = broadcast::channel::<CollabMessage>(1024);

        let mut current_canvas_id: Option<CanvasId> = None;
        let mut current_user_id: Option<UserId> = None;

        loop {
            tokio::select! {
                msg = ws_read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Some(collab_msg) = CollabMessage::from_bytes(text.as_bytes()) {
                                match collab_msg {
                                    CollabMessage::Join { user } => {
                                        current_user_id = Some(user.id.clone());
                                        let canvas_id: CanvasId = "default".to_string();
                                        current_canvas_id = Some(canvas_id.clone());
                                        
                                        let mut rooms_guard = rooms.write().await;
                                        let room = rooms_guard.entry(canvas_id.clone())
                                            .or_insert_with(|| Room {
                                                canvas_id: canvas_id.clone(),
                                                clients: Arc::new(RwLock::new(HashMap::new())),
                                            });
                                        
                                        let mut clients = room.clients.write().await;
                                        clients.insert(user.id.clone(), event_tx.clone());
                                        
                                        let join_msg = CollabMessage::UserJoined { user };
                                        let _ = event_tx.send(join_msg);
                                    }
                                    CollabMessage::Leave => {
                                        break;
                                    }
                                    CollabMessage::CursorMove { cursor } => {
                                        if let Some(user_id) = &current_user_id {
                                            let msg = CollabMessage::CursorMoved {
                                                user_id: user_id.clone(),
                                                cursor,
                                            };
                                            let _ = event_tx.send(msg);
                                        }
                                    }
                                    CollabMessage::ElementAdd { element } => {
                                        let msg = CollabMessage::ElementAdded { element };
                                        let _ = event_tx.send(msg);
                                    }
                                    CollabMessage::ElementUpdate { id, updates } => {
                                        let msg = CollabMessage::ElementUpdated { id, updates };
                                        let _ = event_tx.send(msg);
                                    }
                                    CollabMessage::ElementDelete { id } => {
                                        let msg = CollabMessage::ElementDeleted { id };
                                        let _ = event_tx.send(msg);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        if let (Some(canvas_id), Some(user_id)) = (current_canvas_id, current_user_id) {
            let mut rooms_guard = rooms.write().await;
            if let Some(room) = rooms_guard.get_mut(&canvas_id) {
                let mut clients = room.clients.write().await;
                clients.remove(&user_id);
                
                let leave_msg = CollabMessage::UserLeft { user_id };
                let _ = event_tx.send(leave_msg);
            }
        }

        Ok(())
    }

    pub async fn get_room_clients(&self, canvas_id: &CanvasId) -> usize {
        let rooms = self.rooms.read().await;
        rooms
            .get(canvas_id)
            .map(|r| r.clients.blocking_read().len())
            .unwrap_or(0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Bind failed: {0}")]
    BindFailed(String),
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("Connection lost: {0}")]
    ConnectionLost(String),
}
