use crate::domain::{FileCache, FileMetadata, ScanConfig};
use anyhow::Result;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

pub struct FileCacheAdapter;

impl FileCacheAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn load_cache(&self, cache_path: &Path) -> Result<Option<FileCache>> {
        if !cache_path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(cache_path)?;
        let cache: FileCache = serde_json::from_str(&contents)?;
        Ok(Some(cache))
    }

    pub fn save_cache(&self, cache_path: &Path, cache: &FileCache) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(cache)?;
        fs::write(cache_path, contents)?;
        Ok(())
    }

    pub fn is_cache_valid(&self, cache: &FileCache, config: &ScanConfig) -> bool {
        if cache.scan_config_hash != config.config_hash() {
            return false;
        }

        if let Ok(elapsed) = SystemTime::now().duration_since(cache.last_scan) {
            if elapsed.as_secs() > 24 * 60 * 60 {
                return false;
            }
        }

        if cache.version != env!("CARGO_PKG_VERSION") {
            return false;
        }

        true
    }

    pub fn filter_changed_files(&self, cached_files: &[FileMetadata]) -> Vec<FileMetadata> {
        cached_files
            .iter()
            .filter(|file| {
                if let Ok(metadata) = fs::metadata(&file.path) {
                    if let Ok(modified) = metadata.modified() {
                        return metadata.len() == file.size && modified == file.modified;
                    }
                }
                false
            })
            .cloned()
            .collect()
    }

    pub fn create_cache(&self, files: Vec<FileMetadata>, config: &ScanConfig) -> FileCache {
        FileCache {
            files,
            scan_config_hash: config.config_hash(),
            last_scan: SystemTime::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}