use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWatcherConfig {
    pub watch_paths: Vec<PathBuf>,
    pub poll_interval_ms: u64,
    pub ignored_patterns: Vec<String>,
    pub auto_reindex: bool,
}

impl Default for FileWatcherConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![],
            poll_interval_ms: 5000,
            ignored_patterns: vec![
                "*.tmp".to_string(),
                "*.swp".to_string(),
                ".git".to_string(),
            ],
            auto_reindex: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChangeType {
    Created,
    Modified,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
    pub timestamp: std::time::SystemTime,
}

pub type ChangeCallback = Arc<dyn Fn(FileChange) + Send + Sync>;

pub struct FileWatcher {
    config: FileWatcherConfig,
    callback: Option<ChangeCallback>,
    last_known_files: Arc<RwLock<HashMap<PathBuf, u64>>>,
    is_running: Arc<RwLock<bool>>,
}

impl FileWatcher {
    pub fn new(config: FileWatcherConfig) -> Self {
        Self {
            config,
            callback: None,
            last_known_files: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_callback(mut self, callback: ChangeCallback) -> Self {
        self.callback = Some(callback);
        self
    }

    pub async fn start(&self) -> Result<(), String> {
        if *self.is_running.read().await {
            return Err("FileWatcher is already running".to_string());
        }

        *self.is_running.write().await = true;

        let is_running = self.is_running.clone();
        let config = self.config.clone();
        let last_known_files = self.last_known_files.clone();
        let callback = self.callback.clone();

        tokio::spawn(async move {
            loop {
                if !*is_running.read().await {
                    break;
                }

                for watch_path in &config.watch_paths {
                    if !watch_path.exists() {
                        continue;
                    }

                    if let Err(e) = Self::scan_directory(
                        watch_path,
                        &config.ignored_patterns,
                        &last_known_files,
                        &callback,
                    )
                    .await
                    {
                        tracing::warn!("Error scanning directory {:?}: {}", watch_path, e);
                    }
                }

                tokio::time::sleep(Duration::from_millis(config.poll_interval_ms)).await;
            }

            *is_running.write().await = false;
        });

        Ok(())
    }

    async fn scan_directory(
        dir: &Path,
        ignored_patterns: &[String],
        last_known: &Arc<RwLock<HashMap<PathBuf, u64>>>,
        callback: &Option<ChangeCallback>,
    ) -> Result<(), String> {
        let mut dirs_to_scan = vec![dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let mut entries = tokio::fs::read_dir(&current_dir)
                .await
                .map_err(|e| format!("Failed to read directory: {}", e))?;

            let mut current_files: HashMap<PathBuf, u64> = HashMap::new();
            let mut last = last_known.write().await;

            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();

                if Self::should_ignore(&path, ignored_patterns) {
                    continue;
                }

                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.is_file() {
                    if let Ok(metadata) = entry.metadata().await {
                        let modified = metadata
                            .modified()
                            .map_err(|e| e.to_string())?
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_err(|e| e.to_string())?
                            .as_secs();

                        current_files.insert(path.clone(), modified);

                        if let Some(existing_modified) = last.get(&path) {
                            if *existing_modified != modified {
                                if let Some(cb) = callback {
                                    cb(FileChange {
                                        path: path.clone(),
                                        change_type: FileChangeType::Modified,
                                        timestamp: std::time::SystemTime::now(),
                                    });
                                }
                            }
                        } else {
                            if let Some(cb) = callback {
                                cb(FileChange {
                                    path: path.clone(),
                                    change_type: FileChangeType::Created,
                                    timestamp: std::time::SystemTime::now(),
                                });
                            }
                        }
                    }
                }
            }

            for (path, _) in last.iter() {
                if !current_files.contains_key(path) {
                    if let Some(cb) = callback {
                        cb(FileChange {
                            path: path.clone(),
                            change_type: FileChangeType::Removed,
                            timestamp: std::time::SystemTime::now(),
                        });
                    }
                }
            }

            *last = current_files;
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        *self.is_running.write().await = false;
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    fn should_ignore(path: &Path, patterns: &[String]) -> bool {
        let path_str = path.to_string_lossy();
        for pattern in patterns {
            if pattern.starts_with('*') {
                let suffix = &pattern[1..];
                if path_str.ends_with(suffix) {
                    return true;
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }
        false
    }
}

pub struct MemoryFileIndexer {
    index: Arc<RwLock<HashMap<PathBuf, MemoryFileInfo>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFileInfo {
    pub path: PathBuf,
    pub content: String,
    pub hash: u64,
    pub last_indexed: std::time::SystemTime,
}

impl MemoryFileIndexer {
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn index_file(&self, path: &Path) -> Result<MemoryFileInfo, String> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let hash = Self::simple_hash(&content);

        let info = MemoryFileInfo {
            path: path.to_path_buf(),
            content: content.clone(),
            hash,
            last_indexed: std::time::SystemTime::now(),
        };

        self.index.write().await.insert(path.to_path_buf(), info.clone());

        Ok(info)
    }

    pub async fn remove_file(&self, path: &Path) {
        self.index.write().await.remove(path);
    }

    pub async fn get_indexed_file(&self, path: &Path) -> Option<MemoryFileInfo> {
        self.index.read().await.get(path).cloned()
    }

    pub async fn has_changed(&self, path: &Path) -> Result<bool, String> {
        if let Some(indexed) = self.index.read().await.get(path) {
            let current_content = tokio::fs::read_to_string(path)
                .await
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let current_hash = Self::simple_hash(&current_content);
            Ok(current_hash != indexed.hash)
        } else {
            Ok(true)
        }
    }

    pub async fn rebuild_index(&self, watch_paths: &[PathBuf]) -> Result<(), String> {
        for watch_path in watch_paths {
            if !watch_path.exists() {
                continue;
            }

            Self::index_directory(watch_path, &self.index).await?;
        }

        Ok(())
    }

    async fn index_directory(dir: &Path, index: &Arc<RwLock<HashMap<PathBuf, MemoryFileInfo>>>) -> Result<(), String> {
        let mut dirs_to_scan = vec![dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let mut entries = tokio::fs::read_dir(&current_dir)
                .await
                .map_err(|e| format!("Failed to read directory: {}", e))?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
                let path = entry.path();

                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.is_file() {
                    if path.extension().map_or(false, |ext| ext == "md") {
                        let content = tokio::fs::read_to_string(&path)
                            .await
                            .map_err(|e| format!("Failed to read file: {}", e))?;
                        let hash = Self::simple_hash(&content);

                        let info = MemoryFileInfo {
                            path: path.clone(),
                            content,
                            hash,
                            last_indexed: std::time::SystemTime::now(),
                        };

                        index.write().await.insert(path, info);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn get_all_indexed(&self) -> Vec<MemoryFileInfo> {
        self.index.read().await.values().cloned().collect()
    }

    fn simple_hash(content: &str) -> u64 {
        let mut hash: u64 = 0;
        for (i, byte) in content.bytes().enumerate() {
            hash = hash.wrapping_add((byte as u64).wrapping_mul(i as u64 + 1));
        }
        hash
    }
}

impl Default for MemoryFileIndexer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_watcher_config_default() {
        let config = FileWatcherConfig::default();
        assert_eq!(config.poll_interval_ms, 5000);
        assert!(config.auto_reindex);
    }

    #[test]
    fn test_should_ignore() {
        assert!(FileWatcher::should_ignore(
            &PathBuf::from("/tmp/test.tmp"),
            &["*.tmp".to_string(), "*.swp".to_string()]
        ));
        assert!(FileWatcher::should_ignore(
            &PathBuf::from("/repo/.git/config"),
            &[".git".to_string()]
        ));
        assert!(!FileWatcher::should_ignore(
            &PathBuf::from("/repo/README.md"),
            &["*.tmp".to_string(), ".git".to_string()]
        ));
    }

    #[test]
    fn test_simple_hash() {
        let hash1 = MemoryFileIndexer::simple_hash("hello");
        let hash2 = MemoryFileIndexer::simple_hash("hello");
        let hash3 = MemoryFileIndexer::simple_hash("world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
