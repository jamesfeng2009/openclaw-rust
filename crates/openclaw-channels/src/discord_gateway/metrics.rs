//! Discord Gateway 监控指标

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "discord")]
use super::gateway::ConnectionHealth;

#[cfg(feature = "discord")]
#[derive(Debug, Clone)]
pub struct GatewayMetrics {
    messages_received: Arc<AtomicU64>,
    messages_sent: Arc<AtomicU64>,
    reconnects: Arc<AtomicU64>,
    errors: Arc<AtomicU64>,
    last_message_time: Arc<AtomicU64>,
    connection_health: Arc<tokio::sync::RwLock<ConnectionHealth>>,
}

#[cfg(feature = "discord")]
impl GatewayMetrics {
    pub fn new() -> Self {
        Self {
            messages_received: Arc::new(AtomicU64::new(0)),
            messages_sent: Arc::new(AtomicU64::new(0)),
            reconnects: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            last_message_time: Arc::new(AtomicU64::new(0)),
            connection_health: Arc::new(tokio::sync::RwLock::new(ConnectionHealth::Disconnected)),
        }
    }

    pub fn increment_messages_received(&self) {
        self.messages_received.fetch_add(1, Ordering::SeqCst);
        self.update_last_message_time();
    }

    pub fn increment_messages_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_reconnects(&self) {
        self.reconnects.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_errors(&self) {
        self.errors.fetch_add(1, Ordering::SeqCst);
    }

    fn update_last_message_time(&self) {
        if let Ok(duration) = SystemTime::now().duration_since(UNIX_EPOCH) {
            self.last_message_time.store(duration.as_secs(), Ordering::SeqCst);
        }
    }

    pub fn get_messages_received(&self) -> u64 {
        self.messages_received.load(Ordering::SeqCst)
    }

    pub fn get_messages_sent(&self) -> u64 {
        self.messages_sent.load(Ordering::SeqCst)
    }

    pub fn get_reconnects(&self) -> u64 {
        self.reconnects.load(Ordering::SeqCst)
    }

    pub fn get_errors(&self) -> u64 {
        self.errors.load(Ordering::SeqCst)
    }

    pub fn get_last_message_time(&self) -> u64 {
        self.last_message_time.load(Ordering::SeqCst)
    }

    pub async fn set_connection_health(&self, health: ConnectionHealth) {
        let mut h = self.connection_health.write().await;
        *h = health;
    }

    pub async fn get_connection_health(&self) -> ConnectionHealth {
        let h = self.connection_health.read().await;
        h.clone()
    }

    pub fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            messages_received: self.get_messages_received(),
            messages_sent: self.get_messages_sent(),
            reconnects: self.get_reconnects(),
            errors: self.get_errors(),
            last_message_time: self.get_last_message_time(),
        }
    }
}

#[cfg(feature = "discord")]
impl Default for GatewayMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub messages_received: u64,
    pub messages_sent: u64,
    pub reconnects: u64,
    pub errors: u64,
    pub last_message_time: u64,
}

#[cfg(feature = "discord")]
impl std::fmt::Display for MetricsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Gateway Metrics: received={}, sent={}, reconnects={}, errors={}, last_msg={}",
            self.messages_received,
            self.messages_sent,
            self.reconnects,
            self.errors,
            self.last_message_time
        )
    }
}

#[cfg(test)]
#[cfg(feature = "discord")]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_summary_creation() {
        let summary = MetricsSummary {
            messages_received: 100,
            messages_sent: 50,
            reconnects: 5,
            errors: 2,
            last_message_time: 1234567890,
        };
        assert_eq!(summary.messages_received, 100);
        assert_eq!(summary.messages_sent, 50);
    }

    #[tokio::test]
    async fn test_gateway_metrics_increment() {
        let metrics = GatewayMetrics::new();
        
        metrics.increment_messages_received();
        metrics.increment_messages_sent();
        metrics.increment_reconnects();
        metrics.increment_errors();
        
        assert_eq!(metrics.get_messages_received(), 1);
        assert_eq!(metrics.get_messages_sent(), 1);
        assert_eq!(metrics.get_reconnects(), 1);
        assert_eq!(metrics.get_errors(), 1);
    }

    #[tokio::test]
    async fn test_gateway_metrics_summary() {
        let metrics = GatewayMetrics::new();
        
        metrics.increment_messages_received();
        metrics.increment_messages_received();
        metrics.increment_messages_sent();
        
        let summary = metrics.get_summary();
        assert_eq!(summary.messages_received, 2);
        assert_eq!(summary.messages_sent, 1);
    }

    #[tokio::test]
    async fn test_connection_health_tracking() {
        use crate::discord_gateway::ConnectionHealth;
        
        let metrics = GatewayMetrics::new();
        
        metrics.set_connection_health(ConnectionHealth::Healthy).await;
        assert_eq!(metrics.get_connection_health().await, ConnectionHealth::Healthy);
        
        metrics.set_connection_health(ConnectionHealth::Degraded).await;
        assert_eq!(metrics.get_connection_health().await, ConnectionHealth::Degraded);
    }
}
