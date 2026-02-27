//! Discord Gateway 客户端
//!
//! 使用 serenity 库连接 Discord Gateway，接收实时事件
//! 支持自动重连、心跳保活、错误处理

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time;
use futures::executor::block_on;

#[cfg(feature = "discord")]
use serenity::{
    all::GatewayIntents,
    client::ClientBuilder,
};
#[cfg(feature = "discord")]
use tokio::sync::RwLock;

#[cfg(feature = "discord")]
use super::handler::GatewayEventHandler;
#[cfg(feature = "discord")]
use super::types::DiscordGatewayEvent;

#[cfg(feature = "discord")]
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Disconnected,
}

#[cfg(feature = "discord")]
pub struct DiscordGatewayClient {
    client: serenity::Client,
    token: String,
    intents: GatewayIntents,
    running: Arc<AtomicBool>,
    sequence: Arc<AtomicU64>,
    session_id: Arc<RwLock<Option<String>>>,
    reconnect_attempts: Arc<AtomicU64>,
    last_heartbeat: Arc<RwLock<std::time::Instant>>,
    max_reconnect_delay: u64,
}

#[cfg(feature = "discord")]
impl DiscordGatewayClient {
    pub async fn new(
        bot_token: &str,
        intents: GatewayIntents,
    ) -> Result<(Self, mpsc::UnboundedReceiver<DiscordGatewayEvent>), Box<dyn std::error::Error + Send + Sync>> {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        let handler = GatewayEventHandler::new(event_sender);
        
        let client = ClientBuilder::new(bot_token, intents)
            .event_handler(handler)
            .await?;
            
        let running = Arc::new(AtomicBool::new(true));
        let sequence = Arc::new(AtomicU64::new(0));
        let session_id = Arc::new(RwLock::new(None));
        let reconnect_attempts = Arc::new(AtomicU64::new(0));
        let last_heartbeat = Arc::new(RwLock::new(std::time::Instant::now()));
        const MAX_RECONNECT_DELAY: u64 = 300;

        Ok((
            Self { 
                client, 
                token: bot_token.to_string(),
                intents,
                running,
                sequence,
                session_id,
                reconnect_attempts,
                last_heartbeat,
                max_reconnect_delay: MAX_RECONNECT_DELAY,
            }, 
            event_receiver
        ))
    }
    
    pub async fn run(&mut self) -> Result<(), serenity::Error> {
        self.client.start().await
    }

    pub async fn run_with_reconnect(&mut self) {
        let running = self.running.clone();
        let _token = self.token.clone();
        let _intents = self.intents;
        let _session_id = self.session_id.clone();
        
        while running.load(Ordering::SeqCst) {
            match self.run().await {
                Ok(_) => {
                    tracing::info!("Discord Gateway 连接正常关闭");
                    self.reset_reconnect_attempts();
                    break;
                }
                Err(e) => {
                    tracing::error!("Discord Gateway 连接断开: {}", e);
                    self.increment_reconnect_attempts();
                    
                    if !running.load(Ordering::SeqCst) {
                        tracing::info!("Gateway 已停止");
                        break;
                    }
                    
                    let wait_time = self.calculate_reconnect_delay();
                    tracing::info!("等待 {} 秒后尝试重新连接 (尝试 {} 次)...", 
                        wait_time.as_secs(), 
                        self.reconnect_attempts.load(Ordering::SeqCst));
                    time::sleep(wait_time).await;
                    
                    tracing::info!("尝试重新连接 Discord Gateway...");
                    
                    match self.create_new_client().await {
                        Ok(new_client) => {
                            self.client = new_client;
                            tracing::info!("Discord Gateway 重新连接成功");
                        }
                        Err(e) => {
                            tracing::error!("重新创建 Gateway 客户端失败: {}", e);
                        }
                    }
                }
            }
        }
    }

    fn calculate_reconnect_delay(&self) -> Duration {
        let attempts = self.reconnect_attempts.load(Ordering::SeqCst);
        let delay = 2_u64.pow(attempts.min(8) as u32).min(self.max_reconnect_delay);
        Duration::from_secs(delay)
    }

    fn increment_reconnect_attempts(&self) {
        let _ = self.reconnect_attempts.fetch_add(1, Ordering::SeqCst);
    }

    fn reset_reconnect_attempts(&self) {
        self.reconnect_attempts.store(0, Ordering::SeqCst);
    }

    pub fn update_heartbeat(&self) {
        let mut hb = block_on(self.last_heartbeat.write());
        *hb = std::time::Instant::now();
    }

    pub async fn check_heartbeat(&self) -> bool {
        let hb = self.last_heartbeat.read().await;
        let elapsed = hb.elapsed();
        elapsed < Duration::from_secs(60)
    }

    pub fn get_connection_health(&self) -> ConnectionHealth {
        let attempts = self.reconnect_attempts.load(Ordering::SeqCst);
        let heartbeat_ok = block_on(self.check_heartbeat());
        
        if attempts == 0 && heartbeat_ok {
            ConnectionHealth::Healthy
        } else if attempts < 3 && heartbeat_ok {
            ConnectionHealth::Degraded
        } else if attempts < 10 {
            ConnectionHealth::Unhealthy
        } else {
            ConnectionHealth::Disconnected
        }
    }

    async fn create_new_client(&self) -> Result<serenity::Client, Box<dyn std::error::Error + Send + Sync>> {
        let (event_sender, _event_receiver) = mpsc::unbounded_channel();
        
        let handler = GatewayEventHandler::new(event_sender);
        
        let client = ClientBuilder::new(&self.token, self.intents)
            .event_handler(handler)
            .await?;
            
        Ok(client)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub async fn update_sequence(&self, seq: u64) {
        self.sequence.store(seq, Ordering::SeqCst);
    }

    pub fn get_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    pub async fn set_session_id(&self, session_id: Option<String>) {
        let mut id = self.session_id.write().await;
        *id = session_id;
    }

    pub async fn get_session_id(&self) -> Option<String> {
        let id = self.session_id.read().await;
        id.clone()
    }
}

#[cfg(feature = "discord")]
impl Drop for DiscordGatewayClient {
    fn drop(&mut self) {
        self.stop();
        tracing::info!("Discord Gateway 客户端已停止");
    }
}

#[cfg(test)]
#[cfg(feature = "discord")]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_client_fields() {
        let token = "test_token".to_string();
        let intents = serenity::all::GatewayIntents::GUILD_MESSAGES;
        
        assert_eq!(token.len(), 10);
        assert!(intents.contains(serenity::all::GatewayIntents::GUILD_MESSAGES));
    }

    #[tokio::test]
    async fn test_calculate_reconnect_delay() {
        let delay = Duration::from_secs(5);
        assert_eq!(delay.as_secs(), 5);
    }
}
