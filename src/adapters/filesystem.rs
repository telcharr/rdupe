use crate::domain::{FileMetadata, ScanConfig};
use crate::ports::FileSystemPort;
use anyhow::Result;
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::fs;
use std::os::unix::fs::MetadataExt;

pub struct FileSystemAdapter;

impl FileSystemAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystemPort for FileSystemAdapter {
    fn scan_files(&self, config: &ScanConfig) -> Result<Vec<FileMetadata>> {
        let files: Result<Vec<FileMetadata>> = config
            .paths
            .par_iter()
            .map(|path| -> Result<Vec<FileMetadata>> {
                let mut builder = WalkBuilder::new(path);
                if let Some(max_depth) = config.max_depth {
                    builder.max_depth(Some(max_depth));
                }
                
                builder.follow_links(config.follow_symlinks);
                for pattern in &config.ignore_patterns {
                    builder.add_ignore(&format!("{}\n", pattern));
                }
                
                if !config.cross_filesystem {
                    builder.same_file_system(true);
                }

                let walker = builder.build();
                let root_dev = if !config.cross_filesystem {
                    fs::metadata(path).ok().map(|m| m.dev())
                } else {
                    None
                };

                let entries: Vec<FileMetadata> = walker
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();
                        if !path.is_file() {
                            return None;
                        }

                        let metadata = fs::metadata(path).ok()?;
                        let size = metadata.len();
                        if size < config.min_size {
                            return None;
                        }

                        // Cross-filesystem check
                        if let Some(root_dev) = root_dev {
                            if metadata.dev() != root_dev {
                                return None;
                            }
                        }

                        let modified = metadata.modified().ok()?;
                        Some(FileMetadata::new(path.to_path_buf(), size, modified))
                    })
                    .collect();

                Ok(entries)
            })
            .collect::<Result<Vec<Vec<FileMetadata>>>>()
            .map(|vecs| vecs.into_iter().flatten().collect());

        files
    }
}