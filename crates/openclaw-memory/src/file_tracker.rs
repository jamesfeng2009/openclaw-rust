use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub hash: String,
    pub modified_time: u64,
    pub size: u64,
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTrackerConfig {
    pub data_dir: PathBuf,
    pub index_file: PathBuf,
}

impl Default for FileTrackerConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".openclaw-rust/file_tracker"),
            index_file: PathBuf::from(".openclaw-rust/file_tracker/index.json"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileTracker {
    config: FileTrackerConfig,
    files: HashMap<String, FileEntry>,
    index_modified: bool,
}

impl FileTracker {
    pub fn new(config: FileTrackerConfig) -> Self {
        Self {
            config,
            files: HashMap::new(),
            index_modified: false,
        }
    }

    pub fn load(&mut self) -> anyhow::Result<()> {
        if self.config.index_file.exists() {
            let content = fs::read_to_string(&self.config.index_file)?;
            self.files = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if !self.index_modified {
            return Ok(());
        }

        if let Some(parent) = self.config.index_file.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.files)?;
        fs::write(&self.config.index_file, content)?;
        Ok(())
    }

    pub fn track_file(&mut self, path: &Path) -> anyhow::Result<Option<FileEntry>> {
        let metadata = fs::metadata(path)?;
        let modified = metadata
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        let size = metadata.len();
        let hash = self.compute_hash(path)?;

        let path_str = path.to_string_lossy().to_string();

        let existing = self.files.get(&path_str).cloned();

        if let Some(entry) = &existing
            && entry.hash == hash
            && entry.modified_time == modified
        {
            return Ok(None);
        }

        let entry = FileEntry {
            path: path_str.clone(),
            hash: hash.clone(),
            modified_time: modified,
            size,
            content_hash: None,
        };

        self.files.insert(path_str, entry.clone());
        self.index_modified = true;

        Ok(Some(entry))
    }

    pub fn untrack_file(&mut self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();
        let removed = self.files.remove(&path_str).is_some();
        if removed {
            self.index_modified = true;
        }
        removed
    }

    pub fn is_changed(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_string();

        if let Some(entry) = self.files.get(&path_str)
            && let Ok(metadata) = fs::metadata(path)
            && let Ok(modified) = metadata.modified()
        {
            let modified_secs = modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            return modified_secs != entry.modified_time;
        }

        true
    }

    pub fn get_entry(&self, path: &Path) -> Option<&FileEntry> {
        let path_str = path.to_string_lossy().to_string();
        self.files.get(&path_str)
    }

    pub fn list_tracked_files(&self) -> Vec<&FileEntry> {
        self.files.values().collect()
    }

    pub fn get_changed_files(&self, dir: &Path) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && self.is_changed(&path) {
                    changed.push(path);
                }
            }
        }

        changed
    }

    fn compute_hash(&self, path: &Path) -> anyhow::Result<String> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let content = fs::read(path)?;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);

        Ok(format!("{:x}", hasher.finish()))
    }

    pub fn scan_directory(&mut self, dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let mut updated = Vec::new();

        if !dir.exists() {
            return Ok(updated);
        }

        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file()
                && let Some(result) = self.track_file(path)?
            {
                updated.push(PathBuf::from(result.path));
            }
        }

        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_file_tracker() {
        let temp_dir = env::temp_dir().join("openclaw_test_tracker");
        fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("test.txt");
        fs::write(&test_file, "Hello World").unwrap();

        let config = FileTrackerConfig {
            data_dir: temp_dir.clone(),
            index_file: temp_dir.join("index.json"),
        };

        let mut tracker = FileTracker::new(config);
        let result = tracker.track_file(&test_file).unwrap();

        assert!(result.is_some());

        let tracked = tracker.list_tracked_files();
        assert!(!tracked.is_empty());

        let _ = fs::remove_dir_all(temp_dir);
    }
}
