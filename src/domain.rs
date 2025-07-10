use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HashAlgorithm {
    Blake3,
    Md5,
    Sha1,
    Sha256,
}

impl HashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashAlgorithm::Blake3 => "blake3",
            HashAlgorithm::Md5 => "md5",
            HashAlgorithm::Sha1 => "sha1",
            HashAlgorithm::Sha256 => "sha256",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub partial_hash: Option<String>,
    pub full_hash: Option<String>,
    pub modified: SystemTime,
}

impl FileMetadata {
    pub fn new(path: PathBuf, size: u64, modified: SystemTime) -> Self {
        Self {
            path,
            size,
            partial_hash: None,
            full_hash: None,
            modified,
        }
    }

    pub fn with_partial_hash(mut self, hash: String) -> Self {
        self.partial_hash = Some(hash);
        self
    }

    pub fn with_full_hash(mut self, hash: String) -> Self {
        self.full_hash = Some(hash);
        self
    }

    pub fn get_best_hash(&self) -> Option<&String> {
        self.full_hash.as_ref().or(self.partial_hash.as_ref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateSet {
    pub hash: String,
    pub files: Vec<FileMetadata>,
    pub total_size: u64,
}

impl DuplicateSet {
    pub fn new(hash: String, files: Vec<FileMetadata>) -> Self {
        let total_size = files.iter().map(|f| f.size).sum();
        Self {
            hash,
            files,
            total_size,
        }
    }

    pub fn wasted_space(&self) -> u64 {
        if self.files.len() <= 1 {
            0
        } else {
            self.total_size - self.files[0].size
        }
    }

    pub fn duplicate_count(&self) -> usize {
        self.files.len().saturating_sub(1)
    }
}

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub paths: Vec<PathBuf>,
    pub follow_symlinks: bool,
    pub min_size: u64,
    pub max_depth: Option<usize>,
    pub ignore_patterns: HashSet<String>,
    pub partial_hash_size: u64,
    pub use_mmap_threshold: u64,
    pub thread_count: Option<usize>,
    pub hash_algorithm: HashAlgorithm,
    pub cross_filesystem: bool,
    pub cache_file: Option<PathBuf>,
    pub incremental: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub files: Vec<FileMetadata>,
    pub scan_config_hash: String,
    pub last_scan: SystemTime,
    pub version: String,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            paths: vec![PathBuf::from(".")],
            follow_symlinks: false,
            min_size: 0,
            max_depth: None,
            ignore_patterns: HashSet::new(),
            partial_hash_size: 8192,
            use_mmap_threshold: 64 * 1024 * 1024,
            thread_count: None,
            hash_algorithm: HashAlgorithm::Blake3,
            cross_filesystem: true,
            cache_file: None,
            incremental: false,
        }
    }
}

impl ScanConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.paths = paths;
        self
    }

    pub fn with_follow_symlinks(mut self, follow: bool) -> Self {
        self.follow_symlinks = follow;
        self
    }

    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_size = size;
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn with_cache_file(mut self, cache_file: PathBuf) -> Self {
        self.cache_file = Some(cache_file);
        self
    }

    pub fn with_incremental(mut self, incremental: bool) -> Self {
        self.incremental = incremental;
        self
    }

    pub fn config_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.paths.hash(&mut hasher);
        self.follow_symlinks.hash(&mut hasher);
        self.min_size.hash(&mut hasher);
        self.max_depth.hash(&mut hasher);
        let mut sorted_patterns: Vec<_> = self.ignore_patterns.iter().collect();
        sorted_patterns.sort();
        sorted_patterns.hash(&mut hasher);
        self.partial_hash_size.hash(&mut hasher);
        self.use_mmap_threshold.hash(&mut hasher);
        self.thread_count.hash(&mut hasher);
        self.hash_algorithm.hash(&mut hasher);
        self.cross_filesystem.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub duplicates: Vec<DuplicateSet>,
    pub total_files_scanned: usize,
    pub total_size_scanned: u64,
    pub total_wasted_space: u64,
}

impl ScanResult {
    pub fn new(duplicates: Vec<DuplicateSet>, total_files_scanned: usize, total_size_scanned: u64) -> Self {
        let total_wasted_space = duplicates.iter().map(|d| d.wasted_space()).sum();
        Self {
            duplicates,
            total_files_scanned,
            total_size_scanned,
            total_wasted_space,
        }
    }

    pub fn total_duplicate_files(&self) -> usize {
        self.duplicates.iter().map(|d| d.duplicate_count()).sum()
    }

    pub fn duplicate_groups(&self) -> usize {
        self.duplicates.len()
    }
}