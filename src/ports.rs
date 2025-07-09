use crate::domain::{FileMetadata, HashAlgorithm, ScanConfig, ScanResult};
use anyhow::Result;
use std::path::Path;

pub trait FileSystemPort {
    fn scan_files(&self, config: &ScanConfig) -> Result<Vec<FileMetadata>>;
}

pub trait HashingPort {
    fn hash_file(&self, path: &Path, algorithm: HashAlgorithm) -> Result<String>;
    fn hash_partial(&self, path: &Path, bytes: u64, algorithm: HashAlgorithm) -> Result<String>;
}

pub trait OutputPort {
    fn write_results(&self, results: &ScanResult) -> Result<()>;
}

pub trait ProgressPort {
    fn start(&self, total: u64);
    fn update(&self, processed: u64);
    fn finish(&self);
}