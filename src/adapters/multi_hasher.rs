use crate::domain::HashAlgorithm;
use crate::ports::HashingPort;
use anyhow::Result;
use blake3::Hasher as Blake3Hasher;
use md5;
use memmap2::MmapOptions;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub struct MultiAlgorithmHasher {
    mmap_threshold: u64,
}

impl MultiAlgorithmHasher {
    pub fn new() -> Self {
        Self {
            mmap_threshold: 64 * 1024 * 1024,
        }
    }

    pub fn with_mmap_threshold(mut self, threshold: u64) -> Self {
        self.mmap_threshold = threshold;
        self
    }

    fn hash_with_mmap(&self, path: &Path, limit: Option<u64>, algorithm: HashAlgorithm) -> Result<String> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file)? };
        
        let data = match limit {
            Some(bytes) => &mmap[..bytes.min(mmap.len() as u64) as usize],
            None => &mmap[..],
        };
        
        let hash = match algorithm {
            HashAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                hasher.finalize().to_hex().to_string()
            }
            HashAlgorithm::Md5 => {
                let mut hasher = md5::Context::new();
                hasher.consume(data);
                format!("{:x}", hasher.compute())
            }
            HashAlgorithm::Sha1 => {
                let mut hasher = Sha1::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            }
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                format!("{:x}", hasher.finalize())
            }
        };
        
        Ok(hash)
    }

    fn hash_with_buffered_io(&self, path: &Path, limit: Option<u64>, algorithm: HashAlgorithm) -> Result<String> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0; 8192];
        let mut bytes_processed = 0u64;

        match algorithm {
            HashAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                self.process_buffered_data(&mut reader, &mut buffer, limit, &mut bytes_processed, |data| {
                    hasher.update(data);
                })?;
                Ok(hasher.finalize().to_hex().to_string())
            }
            HashAlgorithm::Md5 => {
                let mut hasher = md5::Context::new();
                self.process_buffered_data(&mut reader, &mut buffer, limit, &mut bytes_processed, |data| {
                    hasher.consume(data);
                })?;
                Ok(format!("{:x}", hasher.compute()))
            }
            HashAlgorithm::Sha1 => {
                let mut hasher = Sha1::new();
                self.process_buffered_data(&mut reader, &mut buffer, limit, &mut bytes_processed, |data| {
                    hasher.update(data);
                })?;
                Ok(format!("{:x}", hasher.finalize()))
            }
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                self.process_buffered_data(&mut reader, &mut buffer, limit, &mut bytes_processed, |data| {
                    hasher.update(data);
                })?;
                Ok(format!("{:x}", hasher.finalize()))
            }
        }
    }

    fn process_buffered_data<F>(&self, reader: &mut BufReader<File>, buffer: &mut [u8], limit: Option<u64>, bytes_processed: &mut u64, mut update_fn: F) -> Result<()>
    where
        F: FnMut(&[u8]),
    {
        loop {
            let bytes_read = reader.read(buffer)?;
            if bytes_read == 0 {
                break;
            }

            let bytes_to_process = if let Some(limit) = limit {
                if *bytes_processed >= limit {
                    break;
                }
                bytes_read.min((limit - *bytes_processed) as usize)
            } else {
                bytes_read
            };

            update_fn(&buffer[..bytes_to_process]);
            *bytes_processed += bytes_to_process as u64;
        }
        Ok(())
    }
}

impl HashingPort for MultiAlgorithmHasher {
    fn hash_file(&self, path: &Path, algorithm: HashAlgorithm) -> Result<String> {
        let file_size = std::fs::metadata(path)?.len();
        
        if file_size >= self.mmap_threshold {
            self.hash_with_mmap(path, None, algorithm)
        } else {
            self.hash_with_buffered_io(path, None, algorithm)
        }
    }

    fn hash_partial(&self, path: &Path, bytes: u64, algorithm: HashAlgorithm) -> Result<String> {
        let file_size = std::fs::metadata(path)?.len();
        
        if file_size >= self.mmap_threshold {
            self.hash_with_mmap(path, Some(bytes), algorithm)
        } else {
            self.hash_with_buffered_io(path, Some(bytes), algorithm)
        }
    }
}