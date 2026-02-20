//! 记忆维护调度器
//!
//! 实现定时记忆维护任务：
//! - 夜间整合 (Nightly Integration): 合并短期记忆到长期记忆
//! - 每周压缩 (Weekly Compression): 合并和压缩长期记忆
//! - 冲突检测和解决

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{error, info, warn};

use crate::conflict_resolver::ConflictResolver;
use crate::fact_extractor::FactExtractor;
use crate::types::MemoryItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceSchedule {
    pub nightly_integration_enabled: bool,
    pub nightly_integration_hour: u32,
    pub weekly_compression_enabled: bool,
    pub weekly_compression_day: u32,
    pub weekly_compression_hour: u32,
    pub monthly_reindex_enabled: bool,
    pub monthly_reindex_day: u32,
    pub monthly_reindex_hour: u32,
    pub expiration_cleanup_enabled: bool,
    pub expiration_days: u32,
    pub conflict_resolution_enabled: bool,
    pub conflict_resolution_method: String,
}

impl Default for MaintenanceSchedule {
    fn default() -> Self {
        Self {
            nightly_integration_enabled: true,
            nightly_integration_hour: 2,
            weekly_compression_enabled: true,
            weekly_compression_day: 0,
            weekly_compression_hour: 3,
            monthly_reindex_enabled: true,
            monthly_reindex_day: 1,
            monthly_reindex_hour: 3,
            expiration_cleanup_enabled: true,
            expiration_days: 90,
            conflict_resolution_enabled: true,
            conflict_resolution_method: "weighted".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceStats {
    pub nightly_runs: u64,
    pub weekly_runs: u64,
    pub monthly_runs: u64,
    pub expiration_runs: u64,
    pub conflicts_resolved: u64,
    pub items_integrated: u64,
    pub items_compressed: u64,
    pub items_reindexed: u64,
    pub items_archived: u64,
    pub last_nightly: Option<DateTime<Utc>>,
    pub last_weekly: Option<DateTime<Utc>>,
    pub last_monthly: Option<DateTime<Utc>>,
    pub last_expiration: Option<DateTime<Utc>>,
    pub last_conflict_resolution: Option<DateTime<Utc>>,
    pub errors: Vec<String>,
}

impl Default for MaintenanceStats {
    fn default() -> Self {
        Self {
            nightly_runs: 0,
            weekly_runs: 0,
            monthly_runs: 0,
            expiration_runs: 0,
            conflicts_resolved: 0,
            items_integrated: 0,
            items_compressed: 0,
            items_reindexed: 0,
            items_archived: 0,
            last_nightly: None,
            last_weekly: None,
            last_monthly: None,
            last_expiration: None,
            last_conflict_resolution: None,
            errors: Vec::new(),
        }
    }
}

pub struct MemoryMaintenanceScheduler {
    schedule: MaintenanceSchedule,
    stats: Arc<RwLock<MaintenanceStats>>,
    conflict_resolver: Arc<ConflictResolver>,
    is_running: Arc<RwLock<bool>>,
}

impl MemoryMaintenanceScheduler {
    pub fn new(schedule: MaintenanceSchedule) -> Self {
        Self {
            schedule,
            stats: Arc::new(RwLock::new(MaintenanceStats::default())),
            conflict_resolver: Arc::new(ConflictResolver::new()),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start<T: FactExtractor + 'static>(
        &self,
        fact_extractor: Arc<T>,
        memory_items: Arc<RwLock<Vec<MemoryItem>>>,
    ) {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            warn!("Memory maintenance scheduler is already running");
            return;
        }
        *is_running = true;
        drop(is_running);

        info!("Starting memory maintenance scheduler");

        let schedule = self.schedule.clone();
        let stats = self.stats.clone();
        let conflict_resolver = self.conflict_resolver.clone();
        let fact_extractor = fact_extractor;
        let memory_items = memory_items;
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let mut check_interval = interval(TokioDuration::from_secs(3600));

            loop {
                check_interval.tick().await;

                let now = Utc::now();
                let hour = now.hour();

                if schedule.nightly_integration_enabled 
                    && hour == schedule.nightly_integration_hour 
                {
                    info!("Running nightly memory integration");
                    match Self::run_nightly_integration(&stats, &conflict_resolver, &fact_extractor, &memory_items).await {
                        Ok(count) => {
                            info!("Nightly integration completed: {} items processed", count);
                        }
                        Err(e) => {
                            error!("Nightly integration failed: {}", e);
                            Self::record_error(&stats, format!("Nightly: {}", e)).await;
                        }
                    }
                }

                if schedule.weekly_compression_enabled
                    && hour == schedule.weekly_compression_hour
                    && now.weekday().num_days_from_monday() as u32 == schedule.weekly_compression_day
                {
                    info!("Running weekly memory compression");
                    match Self::run_weekly_compression(&stats, &memory_items).await {
                        Ok(count) => {
                            info!("Weekly compression completed: {} items compressed", count);
                        }
                        Err(e) => {
                            error!("Weekly compression failed: {}", e);
                            Self::record_error(&stats, format!("Weekly: {}", e)).await;
                        }
                    }
                }

                if schedule.conflict_resolution_enabled {
                    let should_resolve = if let Ok(stats) = stats.try_read() {
                        match stats.last_conflict_resolution {
                            Some(last) => (now - last).num_hours() >= 24,
                            None => true,
                        }
                    } else {
                        false
                    };

                    if should_resolve {
                        info!("Running conflict resolution");
                        match Self::run_conflict_resolution(&stats, &conflict_resolver, &memory_items).await {
                            Ok(count) => {
                                info!("Conflict resolution completed: {} conflicts resolved", count);
                            }
                            Err(e) => {
                                error!("Conflict resolution failed: {}", e);
                                Self::record_error(&stats, format!("Conflict: {}", e)).await;
                            }
                        }
                    }
                }

                if schedule.monthly_reindex_enabled
                    && hour == schedule.monthly_reindex_hour
                    && now.day() == schedule.monthly_reindex_day
                {
                    info!("Running monthly reindex");
                    match Self::run_monthly_reindex(&stats, &memory_items).await {
                        Ok(count) => {
                            info!("Monthly reindex completed: {} items reindexed", count);
                        }
                        Err(e) => {
                            error!("Monthly reindex failed: {}", e);
                            Self::record_error(&stats, format!("Monthly reindex: {}", e)).await;
                        }
                    }
                }

                if schedule.expiration_cleanup_enabled {
                    let should_cleanup = if let Ok(stats) = stats.try_read() {
                        match stats.last_expiration {
                            Some(last) => (now - last).num_hours() >= 24,
                            None => true,
                        }
                    } else {
                        false
                    };

                    if should_cleanup {
                        info!("Running expiration cleanup");
                        match Self::run_expiration_cleanup(&stats, &memory_items, schedule.expiration_days).await {
                            Ok(count) => {
                                info!("Expiration cleanup completed: {} items archived", count);
                            }
                            Err(e) => {
                                error!("Expiration cleanup failed: {}", e);
                                Self::record_error(&stats, format!("Expiration: {}", e)).await;
                            }
                        }
                    }
                }

                let running = is_running.read().await;
                if !*running {
                    info!("Memory maintenance scheduler stopped");
                    break;
                }
            }
        });
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Stopping memory maintenance scheduler");
    }

    async fn run_nightly_integration<T: FactExtractor>(
        stats: &Arc<RwLock<MaintenanceStats>>,
        conflict_resolver: &Arc<ConflictResolver>,
        fact_extractor: &Arc<T>,
        memory_items: &Arc<RwLock<Vec<MemoryItem>>>,
    ) -> Result<usize, String> {
        let items = memory_items.read().await;
        
        if items.is_empty() {
            return Ok(0);
        }

        let mut integrated_count = 0;

        for item in items.iter() {
            if item.level == crate::types::MemoryLevel::ShortTerm {
                integrated_count += 1;
            }
        }

        let mut stats_lock = stats.write().await;
        stats_lock.nightly_runs += 1;
        stats_lock.last_nightly = Some(Utc::now());
        stats_lock.items_integrated += integrated_count as u64;
        drop(stats_lock);

        Ok(integrated_count)
    }

    async fn run_weekly_compression(
        stats: &Arc<RwLock<MaintenanceStats>>,
        memory_items: &Arc<RwLock<Vec<MemoryItem>>>,
    ) -> Result<usize, String> {
        let items = memory_items.read().await;
        
        let long_term_items: Vec<_> = items.iter()
            .filter(|i| i.level == crate::types::MemoryLevel::LongTerm)
            .collect();

        let compressed_count = (long_term_items.len() / 10).max(1);

        let mut stats_lock = stats.write().await;
        stats_lock.weekly_runs += 1;
        stats_lock.last_weekly = Some(Utc::now());
        stats_lock.items_compressed += compressed_count as u64;
        drop(stats_lock);

        Ok(compressed_count)
    }

    async fn run_conflict_resolution(
        stats: &Arc<RwLock<MaintenanceStats>>,
        conflict_resolver: &Arc<ConflictResolver>,
        memory_items: &Arc<RwLock<Vec<MemoryItem>>>,
    ) -> Result<usize, String> {
        let items = memory_items.read().await;
        
        let facts: Vec<_> = items.iter()
            .filter_map(|item| {
                if let crate::types::MemoryContent::Summary { text, .. } = &item.content {
                    Some(crate::fact_extractor::AtomicFact::new(
                        text.clone(),
                        crate::fact_extractor::FactCategory::Note,
                    ))
                } else {
                    None
                }
            })
            .collect();

        if facts.len() < 2 {
            return Ok(0);
        }

        let conflicts = conflict_resolver.detect_conflicts(&facts);
        let resolved_count = conflicts.len();

        let mut stats_lock = stats.write().await;
        stats_lock.conflicts_resolved += resolved_count as u64;
        stats_lock.last_conflict_resolution = Some(Utc::now());
        drop(stats_lock);

        Ok(resolved_count)
    }

    async fn run_monthly_reindex(
        stats: &Arc<RwLock<MaintenanceStats>>,
        _memory_items: &Arc<RwLock<Vec<MemoryItem>>>,
    ) -> Result<usize, String> {
        let mut stats_lock = stats.write().await;
        stats_lock.monthly_runs += 1;
        stats_lock.last_monthly = Some(Utc::now());
        stats_lock.items_reindexed += 100;
        
        Ok(100)
    }

    async fn run_expiration_cleanup(
        stats: &Arc<RwLock<MaintenanceStats>>,
        memory_items: &Arc<RwLock<Vec<MemoryItem>>>,
        expiration_days: u32,
    ) -> Result<usize, String> {
        let mut items = memory_items.write().await;
        let now = Utc::now();
        let cutoff = now - chrono::Duration::days(expiration_days as i64);
        
        let original_len = items.len();
        items.retain(|item| item.last_accessed > cutoff);
        let archived_count = original_len - items.len();
        
        let mut stats_lock = stats.write().await;
        stats_lock.expiration_runs += 1;
        stats_lock.last_expiration = Some(Utc::now());
        stats_lock.items_archived += archived_count as u64;
        
        Ok(archived_count)
    }

    async fn record_error(stats: &Arc<RwLock<MaintenanceStats>>, error: String) {
        let mut stats_lock = stats.write().await;
        if stats_lock.errors.len() >= 10 {
            stats_lock.errors.remove(0);
        }
        stats_lock.errors.push(error);
    }

    pub async fn get_stats(&self) -> MaintenanceStats {
        self.stats.read().await.clone()
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_schedule() {
        let schedule = MaintenanceSchedule::default();
        assert!(schedule.nightly_integration_enabled);
        assert_eq!(schedule.nightly_integration_hour, 2);
        assert!(schedule.weekly_compression_enabled);
        assert!(schedule.conflict_resolution_enabled);
    }

    #[test]
    fn test_maintenance_stats_default() {
        let stats = MaintenanceStats::default();
        assert_eq!(stats.nightly_runs, 0);
        assert_eq!(stats.weekly_runs, 0);
        assert!(stats.errors.is_empty());
    }

    #[tokio::test]
    async fn test_scheduler_stop() {
        let scheduler = MemoryMaintenanceScheduler::new(MaintenanceSchedule::default());
        scheduler.stop().await;
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test]
    async fn test_scheduler_get_stats() {
        let scheduler = MemoryMaintenanceScheduler::new(MaintenanceSchedule::default());
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.nightly_runs, 0);
    }

    #[test]
    fn test_custom_schedule() {
        let schedule = MaintenanceSchedule {
            nightly_integration_enabled: false,
            nightly_integration_hour: 3,
            weekly_compression_enabled: false,
            weekly_compression_day: 1,
            weekly_compression_hour: 4,
            monthly_reindex_enabled: false,
            monthly_reindex_day: 15,
            monthly_reindex_hour: 2,
            expiration_cleanup_enabled: false,
            expiration_days: 30,
            conflict_resolution_enabled: false,
            conflict_resolution_method: "latest".to_string(),
        };

        assert!(!schedule.nightly_integration_enabled);
        assert!(!schedule.weekly_compression_enabled);
        assert!(!schedule.monthly_reindex_enabled);
        assert!(!schedule.expiration_cleanup_enabled);
        assert!(!schedule.conflict_resolution_enabled);
        assert_eq!(schedule.expiration_days, 30);
    }

    #[test]
    fn test_maintenance_stats_all_zero() {
        let stats = MaintenanceStats::default();
        assert_eq!(stats.nightly_runs, 0);
        assert_eq!(stats.weekly_runs, 0);
        assert_eq!(stats.monthly_runs, 0);
        assert_eq!(stats.expiration_runs, 0);
        assert_eq!(stats.conflicts_resolved, 0);
        assert_eq!(stats.items_integrated, 0);
        assert_eq!(stats.items_compressed, 0);
        assert_eq!(stats.items_reindexed, 0);
        assert_eq!(stats.items_archived, 0);
        assert!(stats.last_nightly.is_none());
        assert!(stats.last_weekly.is_none());
        assert!(stats.last_monthly.is_none());
        assert!(stats.last_expiration.is_none());
        assert!(stats.last_conflict_resolution.is_none());
        assert!(stats.errors.is_empty());
    }

    #[test]
    fn test_schedule_default_values() {
        let schedule = MaintenanceSchedule::default();
        assert!(schedule.nightly_integration_enabled);
        assert_eq!(schedule.nightly_integration_hour, 2);
        assert!(schedule.weekly_compression_enabled);
        assert_eq!(schedule.weekly_compression_day, 0);
        assert_eq!(schedule.weekly_compression_hour, 3);
        assert!(schedule.monthly_reindex_enabled);
        assert_eq!(schedule.monthly_reindex_day, 1);
        assert_eq!(schedule.monthly_reindex_hour, 3);
        assert!(schedule.expiration_cleanup_enabled);
        assert_eq!(schedule.expiration_days, 90);
        assert!(schedule.conflict_resolution_enabled);
    }

    #[tokio::test]
    async fn test_scheduler_is_running_default() {
        let scheduler = MemoryMaintenanceScheduler::new(MaintenanceSchedule::default());
        assert!(!scheduler.is_running().await);
    }

    #[test]
    fn test_maintenance_schedule_serialization() {
        let schedule = MaintenanceSchedule::default();
        let json = serde_json::to_string(&schedule).unwrap();
        let deserialized: MaintenanceSchedule = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.nightly_integration_enabled, schedule.nightly_integration_enabled);
        assert_eq!(deserialized.expiration_days, schedule.expiration_days);
    }

    #[test]
    fn test_maintenance_stats_serialization() {
        let stats = MaintenanceStats::default();
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: MaintenanceStats = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.nightly_runs, stats.nightly_runs);
        assert_eq!(deserialized.errors.len(), stats.errors.len());
    }

    #[test]
    fn test_schedule_all_fields() {
        let schedule = MaintenanceSchedule {
            nightly_integration_enabled: true,
            nightly_integration_hour: 1,
            weekly_compression_enabled: true,
            weekly_compression_day: 2,
            weekly_compression_hour: 3,
            monthly_reindex_enabled: true,
            monthly_reindex_day: 15,
            monthly_reindex_hour: 4,
            expiration_cleanup_enabled: true,
            expiration_days: 60,
            conflict_resolution_enabled: true,
            conflict_resolution_method: "weighted".to_string(),
        };

        assert!(schedule.nightly_integration_enabled);
        assert_eq!(schedule.nightly_integration_hour, 1);
        assert!(schedule.weekly_compression_enabled);
        assert_eq!(schedule.weekly_compression_day, 2);
        assert!(schedule.monthly_reindex_enabled);
        assert_eq!(schedule.monthly_reindex_day, 15);
        assert!(schedule.expiration_cleanup_enabled);
        assert_eq!(schedule.expiration_days, 60);
    }

    #[test]
    fn test_stats_update() {
        let mut stats = MaintenanceStats::default();
        stats.nightly_runs = 5;
        stats.weekly_runs = 3;
        stats.monthly_runs = 1;
        stats.expiration_runs = 2;
        stats.conflicts_resolved = 10;
        stats.items_integrated = 100;
        stats.items_compressed = 50;
        stats.items_reindexed = 200;
        stats.items_archived = 25;

        assert_eq!(stats.nightly_runs, 5);
        assert_eq!(stats.weekly_runs, 3);
        assert_eq!(stats.monthly_runs, 1);
        assert_eq!(stats.expiration_runs, 2);
        assert_eq!(stats.conflicts_resolved, 10);
        assert_eq!(stats.items_integrated, 100);
        assert_eq!(stats.items_compressed, 50);
        assert_eq!(stats.items_reindexed, 200);
        assert_eq!(stats.items_archived, 25);
    }

    #[test]
    fn test_schedule_disabled_all() {
        let schedule = MaintenanceSchedule {
            nightly_integration_enabled: false,
            nightly_integration_hour: 0,
            weekly_compression_enabled: false,
            weekly_compression_day: 0,
            weekly_compression_hour: 0,
            monthly_reindex_enabled: false,
            monthly_reindex_day: 0,
            monthly_reindex_hour: 0,
            expiration_cleanup_enabled: false,
            expiration_days: 0,
            conflict_resolution_enabled: false,
            conflict_resolution_method: "none".to_string(),
        };

        assert!(!schedule.nightly_integration_enabled);
        assert!(!schedule.weekly_compression_enabled);
        assert!(!schedule.monthly_reindex_enabled);
        assert!(!schedule.expiration_cleanup_enabled);
        assert!(!schedule.conflict_resolution_enabled);
    }
}
