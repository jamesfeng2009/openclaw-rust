//! Turn Limiter - Turn 计数限制器
//!
//! 防止无限循环：
//! - 最大 Turn 数限制
//! - 每个 Turn 的 token 限制
//! - 滑动窗口统计

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnLimitConfig {
    pub max_turns: u64,
    pub max_tokens_per_turn: u64,
    pub max_total_tokens: u64,
    pub window_size: usize,
}

impl Default for TurnLimitConfig {
    fn default() -> Self {
        Self {
            max_turns: 100,
            max_tokens_per_turn: 100000,
            max_total_tokens: 1000000,
            window_size: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnStats {
    pub turn_count: u64,
    pub total_tokens: u64,
    pub last_turn_at: DateTime<Utc>,
    pub window_tokens: Vec<u64>,
}

impl Default for TurnStats {
    fn default() -> Self {
        Self {
            turn_count: 0,
            total_tokens: 0,
            last_turn_at: Utc::now(),
            window_tokens: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitReason {
    MaxTurnsReached,
    MaxTokensPerTurnReached,
    MaxTotalTokensReached,
    TokenBurstDetected,
}

impl std::fmt::Display for LimitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LimitReason::MaxTurnsReached => write!(f, "max turns reached"),
            LimitReason::MaxTokensPerTurnReached => write!(f, "max tokens per turn reached"),
            LimitReason::MaxTotalTokensReached => write!(f, "max total tokens reached"),
            LimitReason::TokenBurstDetected => write!(f, "token burst detected"),
        }
    }
}

pub struct TurnLimiter {
    config: TurnLimitConfig,
    turn_count: AtomicU64,
    total_tokens: AtomicU64,
    stats: Arc<std::sync::RwLock<TurnStats>>,
}

impl TurnLimiter {
    pub fn new(config: TurnLimitConfig) -> Self {
        Self {
            config,
            turn_count: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            stats: Arc::new(std::sync::RwLock::new(TurnStats::default())),
        }
    }

    pub fn with_default() -> Self {
        Self::new(TurnLimitConfig::default())
    }

    #[inline]
    pub fn config(&self) -> &TurnLimitConfig {
        &self.config
    }

    #[inline]
    pub fn max_turns(&self) -> u64 {
        self.config.max_turns
    }

    #[inline]
    pub fn max_tokens_per_turn(&self) -> u64 {
        self.config.max_tokens_per_turn
    }

    #[inline]
    pub fn max_total_tokens(&self) -> u64 {
        self.config.max_total_tokens
    }

    pub fn current_turn(&self) -> u64 {
        self.turn_count.load(Ordering::SeqCst)
    }

    pub fn current_tokens(&self) -> u64 {
        self.total_tokens.load(Ordering::SeqCst)
    }

    pub fn increment_turn(&self, tokens: u64) -> Result<(), LimitReason> {
        let current_turn = self.turn_count.load(Ordering::SeqCst);
        let current_total = self.total_tokens.load(Ordering::SeqCst);
        
        if current_turn >= self.config.max_turns {
            return Err(LimitReason::MaxTurnsReached);
        }

        if tokens > self.config.max_tokens_per_turn {
            return Err(LimitReason::MaxTokensPerTurnReached);
        }

        let new_total = current_total + tokens;
        if new_total > self.config.max_total_tokens {
            return Err(LimitReason::MaxTotalTokensReached);
        }

        let window_avg = {
            let stats = self.stats.read().unwrap();
            if stats.window_tokens.len() >= 2 {
                let sum: u64 = stats.window_tokens.iter().sum();
                sum / stats.window_tokens.len() as u64
            } else {
                0
            }
        };
        
        if window_avg > 0 && tokens > window_avg * 3 {
            return Err(LimitReason::TokenBurstDetected);
        }

        let turn = self.turn_count.fetch_add(1, Ordering::SeqCst);
        let total = self.total_tokens.fetch_add(tokens, Ordering::SeqCst) + tokens;
        
        {
            let mut stats = self.stats.write().unwrap();
            stats.turn_count = turn + 1;
            stats.total_tokens = total;
            stats.last_turn_at = Utc::now();
            
            if stats.window_tokens.len() >= self.config.window_size {
                stats.window_tokens.remove(0);
            }
            stats.window_tokens.push(tokens);
        }

        Ok(())
    }

    pub fn check(&self) -> Result<(), LimitReason> {
        let turn = self.turn_count.load(Ordering::SeqCst);
        let total = self.total_tokens.load(Ordering::SeqCst);

        if turn >= self.config.max_turns {
            return Err(LimitReason::MaxTurnsReached);
        }

        if total > self.config.max_total_tokens {
            return Err(LimitReason::MaxTotalTokensReached);
        }

        Ok(())
    }

    pub fn is_limited(&self) -> bool {
        self.check().is_err()
    }

    pub fn remaining_turns(&self) -> u64 {
        let current = self.turn_count.load(Ordering::SeqCst);
        self.config.max_turns.saturating_sub(current)
    }

    pub fn remaining_tokens(&self) -> u64 {
        let current = self.total_tokens.load(Ordering::SeqCst);
        self.config.max_total_tokens.saturating_sub(current)
    }

    pub fn reset(&self) {
        self.turn_count.store(0, Ordering::SeqCst);
        self.total_tokens.store(0, Ordering::SeqCst);
        
        let mut stats = self.stats.write().unwrap();
        *stats = TurnStats::default();
    }

    pub fn stats(&self) -> TurnStats {
        let stats = self.stats.read().unwrap();
        stats.clone()
    }

    pub fn window_average_tokens(&self) -> u64 {
        let stats = self.stats.read().unwrap();
        if stats.window_tokens.is_empty() {
            return 0;
        }
        stats.window_tokens.iter().sum::<u64>() / stats.window_tokens.len() as u64
    }
}

impl Default for TurnLimiter {
    fn default() -> Self {
        Self::with_default()
    }
}

impl std::fmt::Debug for TurnLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TurnLimiter")
            .field("config", &self.config)
            .field("turn_count", &self.turn_count.load(Ordering::SeqCst))
            .field("total_tokens", &self.total_tokens.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_limiter() -> TurnLimiter {
        TurnLimiter::new(TurnLimitConfig {
            max_turns: 5,
            max_tokens_per_turn: 100,
            max_total_tokens: 500,
            window_size: 3,
        })
    }

    #[test]
    fn test_increment_turn_success() {
        let limiter = create_test_limiter();
        
        let result = limiter.increment_turn(50);
        assert!(result.is_ok());
        assert_eq!(limiter.current_turn(), 1);
    }

    #[test]
    fn test_max_turns_reached() {
        let limiter = TurnLimiter::new(TurnLimitConfig {
            max_turns: 4,
            max_tokens_per_turn: 1000,
            max_total_tokens: 10000,
            window_size: 3,
        });
        
        for _ in 0..4 {
            limiter.increment_turn(10).unwrap();
        }
        
        let result = limiter.increment_turn(10);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), LimitReason::MaxTurnsReached);
    }

    #[test]
    fn test_max_tokens_per_turn() {
        let limiter = create_test_limiter();
        
        let result = limiter.increment_turn(150);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), LimitReason::MaxTokensPerTurnReached);
    }

    #[test]
    fn test_max_total_tokens() {
        let limiter = TurnLimiter::new(TurnLimitConfig {
            max_turns: 100,
            max_tokens_per_turn: 1000,
            max_total_tokens: 500,
            window_size: 3,
        });
        
        for _ in 0..4 {
            limiter.increment_turn(100).unwrap();
        }
        
        let result = limiter.increment_turn(200);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), LimitReason::MaxTotalTokensReached);
    }

    #[test]
    fn test_token_burst_detection() {
        let limiter = TurnLimiter::new(TurnLimitConfig {
            max_turns: 10,
            max_tokens_per_turn: 1000,
            max_total_tokens: 10000,
            window_size: 3,
        });
        
        limiter.increment_turn(10).unwrap();
        limiter.increment_turn(10).unwrap();
        limiter.increment_turn(10).unwrap();
        
        let result = limiter.increment_turn(150);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), LimitReason::TokenBurstDetected);
    }

    #[test]
    fn test_remaining() {
        let limiter = create_test_limiter();
        
        assert_eq!(limiter.remaining_turns(), 5);
        assert_eq!(limiter.remaining_tokens(), 500);
        
        limiter.increment_turn(50).unwrap();
        
        assert_eq!(limiter.remaining_turns(), 4);
        assert_eq!(limiter.remaining_tokens(), 450);
    }

    #[test]
    fn test_reset() {
        let limiter = create_test_limiter();
        
        limiter.increment_turn(100).unwrap();
        assert_eq!(limiter.current_turn(), 1);
        
        limiter.reset();
        assert_eq!(limiter.current_turn(), 0);
        assert_eq!(limiter.current_tokens(), 0);
    }

    #[test]
    fn test_check() {
        let limiter = TurnLimiter::new(TurnLimitConfig {
            max_turns: 10,
            max_tokens_per_turn: 1000,
            max_total_tokens: 10000,
            window_size: 3,
        });
        
        assert!(limiter.check().is_ok());
        
        for _ in 0..10 {
            limiter.increment_turn(10).unwrap();
        }
        
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_is_limited() {
        let limiter = TurnLimiter::new(TurnLimitConfig {
            max_turns: 10,
            max_tokens_per_turn: 1000,
            max_total_tokens: 10000,
            window_size: 3,
        });
        
        assert!(!limiter.is_limited());
        
        for _ in 0..10 {
            limiter.increment_turn(10).unwrap();
        }
        
        assert!(limiter.is_limited());
    }

    #[test]
    fn test_stats() {
        let limiter = create_test_limiter();
        
        limiter.increment_turn(50).unwrap();
        
        let stats = limiter.stats();
        assert_eq!(stats.turn_count, 1);
        assert_eq!(stats.total_tokens, 50);
    }

    #[test]
    fn test_window_average() {
        let limiter = create_test_limiter();
        
        assert_eq!(limiter.window_average_tokens(), 0);
        
        limiter.increment_turn(10).unwrap();
        assert_eq!(limiter.window_average_tokens(), 10);
        
        limiter.increment_turn(20).unwrap();
        assert_eq!(limiter.window_average_tokens(), 15);
        
        limiter.increment_turn(30).unwrap();
        assert_eq!(limiter.window_average_tokens(), 20);
    }
}
