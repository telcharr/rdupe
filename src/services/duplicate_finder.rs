use crate::adapters::FileCacheAdapter;
use crate::domain::{DuplicateSet, FileMetadata, ScanConfig, ScanResult};
use crate::ports::{FileSystemPort, HashingPort, ProgressPort};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct DuplicateFinderService<F, H, P> {
    filesystem: F,
    hasher: H,
    progress: P,
    cache: FileCacheAdapter,
}

impl<F, H, P> DuplicateFinderService<F, H, P>
where
    F: FileSystemPort,
    H: HashingPort + Send + Sync,
    P: ProgressPort + Send + Sync,
{
    pub fn new(filesystem: F, hasher: H, progress: P) -> Self {
        Self {
            filesystem,
            hasher,
            progress,
            cache: FileCacheAdapter::new(),
        }
    }

    pub fn find_duplicates(&self, config: &ScanConfig) -> Result<ScanResult> {
        if let Some(thread_count) = config.thread_count {
            rayon::ThreadPoolBuilder::new()
                .num_threads(thread_count)
                .build_global()
                .map_err(|e| anyhow::anyhow!("Failed to configure thread pool: {}", e))?;
        }

        let mut cached_files = Vec::new();
        if let Some(cache_path) = &config.cache_file {
            if let Ok(Some(cache)) = self.cache.load_cache(cache_path) {
                if self.cache.is_cache_valid(&cache, config) {
                    cached_files = if config.incremental {
                        self.cache.filter_changed_files(&cache.files)
                    } else {
                        cache.files
                    };
                }
            }
        }

        let files = if config.incremental && !cached_files.is_empty() {
            let new_files = self.filesystem.scan_files(config)?;
            let new_paths: std::collections::HashSet<_> = new_files.iter().map(|f| &f.path).collect();
            let valid_cached: Vec<_> = cached_files.into_iter()
                .filter(|f| new_paths.contains(&f.path))
                .collect();

            let mut merged = valid_cached;
            for file in new_files {
                if !merged.iter().any(|f| f.path == file.path) {
                    merged.push(file);
                }
            }
            merged
        } else {
            self.filesystem.scan_files(config)?
        };

        let total_files = files.len();
        let total_size: u64 = files.iter().map(|f| f.size).sum();

        if files.is_empty() {
            return Ok(ScanResult::new(vec![], 0, 0));
        }

        let mut size_groups: HashMap<u64, Vec<FileMetadata>> = HashMap::new();
        for file in files.clone() {
            size_groups.entry(file.size).or_default().push(file);
        }

        let potential_duplicates: Vec<Vec<FileMetadata>> = size_groups
            .into_values()
            .filter(|group| group.len() > 1)
            .collect();

        if potential_duplicates.is_empty() {
            if let Some(cache_path) = &config.cache_file {
                let cache = self.cache.create_cache(files, config);
                let _ = self.cache.save_cache(cache_path, &cache);
            }
            return Ok(ScanResult::new(vec![], total_files, total_size));
        }

        let result = self.progressive_hash_with_channels(potential_duplicates, config)?;
        if let Some(cache_path) = &config.cache_file {
            let cache = self.cache.create_cache(files, config);
            let _ = self.cache.save_cache(cache_path, &cache);
        }

        Ok(ScanResult::new(result, total_files, total_size))
    }

    fn progressive_hash_with_channels(
        &self,
        size_groups: Vec<Vec<FileMetadata>>,
        config: &ScanConfig,
    ) -> Result<Vec<DuplicateSet>> {
        let total_files_to_hash: usize = size_groups.iter().map(|group| group.len()).sum();
        self.progress.start(total_files_to_hash as u64 * 2); // Partial + full hash
        let partial_hash_groups = self.hash_files_parallel(size_groups, config, true)?;
        let full_hash_groups = self.hash_files_parallel(partial_hash_groups, config, false)?;
        self.progress.finish();

        let mut hash_groups: HashMap<String, Vec<FileMetadata>> = HashMap::new();
        for group in full_hash_groups {
            for file in group {
                if let Some(hash) = file.get_best_hash() {
                    hash_groups.entry(hash.clone()).or_default().push(file);
                }
            }
        }

        let duplicates: Vec<DuplicateSet> = hash_groups
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|(hash, files)| DuplicateSet::new(hash, files))
            .collect();

        Ok(duplicates)
    }

    fn hash_files_parallel(
        &self,
        file_groups: Vec<Vec<FileMetadata>>,
        config: &ScanConfig,
        is_partial: bool,
    ) -> Result<Vec<Vec<FileMetadata>>> {
        let hasher = Arc::new(&self.hasher);
        let counter = Arc::new(AtomicUsize::new(0));
        let progress_ref = &self.progress;

        let hashed_groups: Vec<Vec<FileMetadata>> = file_groups
            .into_par_iter()
            .filter(|group| group.len() > 1)
            .map(|group| {
                let mut processed_files = Vec::new();
                
                for file in group {
                    let hash_result = if is_partial {
                        let adaptive_size = Self::calculate_adaptive_partial_hash_size(file.size, config.partial_hash_size);
                        hasher.hash_partial(&file.path, adaptive_size, config.hash_algorithm)
                    } else {
                        hasher.hash_file(&file.path, config.hash_algorithm)
                    };

                    match hash_result {
                        Ok(hash) => {
                            let updated_file = if is_partial {
                                file.with_partial_hash(hash)
                            } else {
                                file.with_full_hash(hash)
                            };
                            processed_files.push(updated_file);
                        }
                        Err(_) => {
                            // Skip files that can't be hashed
                            continue;
                        }
                    }
                    
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    progress_ref.update(count as u64 + 1);
                }
                
                processed_files
            })
            .collect();

        let mut results = Vec::new();
        for group in hashed_groups {
            if group.len() > 1 {
                let mut hash_groups: HashMap<String, Vec<FileMetadata>> = HashMap::new();
                for file in group {
                    if let Some(hash) = if is_partial { &file.partial_hash } else { &file.full_hash } {
                        hash_groups.entry(hash.clone()).or_default().push(file);
                    }
                }

                for (_, group) in hash_groups {
                    if group.len() > 1 {
                        results.push(group);
                    }
                }
            }
        }

        Ok(results)
    }

    fn calculate_adaptive_partial_hash_size(file_size: u64, base_size: u64) -> u64 {
        match file_size {
            // For very small files (< 4KB), use the entire file
            0..=4096 => file_size,
            // For small files (4KB - 64KB), use 1KB 
            4097..=65536 => 1024.min(file_size),
            // For medium files (64KB - 1MB), use the configured base size
            65537..=1048576 => base_size.min(file_size),
            // For large files (1MB - 100MB), use 16KB
            1048577..=104857600 => (base_size * 2).min(file_size),
            // For very large files (> 100MB), use 64KB for better discrimination
            _ => (base_size * 8).min(file_size),
        }
    }
}