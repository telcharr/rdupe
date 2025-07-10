use crate::domain::{HashAlgorithm, ScanConfig};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum HashAlgorithmChoice {
    #[value(help = "Fast non-cryptographic hash")]
    Xxhash64,
    #[value(help = "xxHash variant, fast non-cryptographic hash")]
    Xxhash3,
    #[value(help = "Fast non-cryptographic hash")]
    Wyhash,
    #[value(help = "Fast non-cryptographic hash")]
    Twox64,
    #[value(help = "Cryptographic hash")]
    Blake3,
    #[value(help = "Cryptographic hash")]
    Sha256,
    #[value(help = "Slow legacy hash")]
    Md5,
    #[value(help = "Slow legacy hash")]
    Sha1,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
    Tree,
}

impl From<HashAlgorithmChoice> for HashAlgorithm {
    fn from(choice: HashAlgorithmChoice) -> Self {
        match choice {
            HashAlgorithmChoice::Xxhash64 => HashAlgorithm::XxHash64,
            HashAlgorithmChoice::Xxhash3 => HashAlgorithm::XxHash3,
            HashAlgorithmChoice::Wyhash => HashAlgorithm::WyHash,
            HashAlgorithmChoice::Twox64 => HashAlgorithm::TwoXHash64,
            HashAlgorithmChoice::Blake3 => HashAlgorithm::Blake3,
            HashAlgorithmChoice::Sha256 => HashAlgorithm::Sha256,
            HashAlgorithmChoice::Md5 => HashAlgorithm::Md5,
            HashAlgorithmChoice::Sha1 => HashAlgorithm::Sha1,
        }
    }
}

#[derive(Parser)]
#[command(name = "rdupe")]
#[command(about = "A fast duplicate file finder")]
#[command(version)]
pub struct Cli {
    #[arg(help = "Paths to scan for duplicates")]
    pub paths: Vec<PathBuf>,

    #[arg(
        short = 's',
        long = "min-size",
        help = "Minimum file size in bytes to consider",
        default_value = "0"
    )]
    pub min_size: u64,

    #[arg(
        short = 'd',
        long = "max-depth",
        help = "Maximum directory depth to scan"
    )]
    pub max_depth: Option<usize>,

    #[arg(
        short = 'L',
        long = "follow-symlinks",
        help = "Follow symbolic links"
    )]
    pub follow_symlinks: bool,

    #[arg(
        short = 'i',
        long = "ignore",
        help = "Ignore files containing this pattern in their name",
        action = clap::ArgAction::Append
    )]
    pub ignore_patterns: Vec<String>,

    #[arg(
        short = 'q',
        long = "quiet",
        help = "Suppress progress output"
    )]
    pub quiet: bool,

    #[arg(
        short = 'j',
        long = "threads",
        help = "Number of threads to use for hashing"
    )]
    pub threads: Option<usize>,

    #[arg(
        long = "partial-hash-size",
        help = "Size in bytes for partial hash",
        default_value = "8192"
    )]
    pub partial_hash_size: u64,

    #[arg(
        long = "mmap-threshold",
        help = "File size threshold for using memory mapping",
        default_value = "67108864"
    )]
    pub mmap_threshold: u64,

    #[arg(
        short = 'a',
        long = "algorithm",
        help = "Hash algorithm to use",
        value_enum,
        default_value = "xxhash64"
    )]
    pub hash_algorithm: HashAlgorithmChoice,

    #[arg(
        long = "no-cross-filesystem",
        help = "Do not cross filesystem boundaries"
    )]
    pub no_cross_filesystem: bool,

    #[arg(
        short = 'c',
        long = "cache",
        help = "Cache file path for resume capability"
    )]
    pub cache_file: Option<PathBuf>,

    #[arg(
        long = "incremental",
        help = "Perform incremental scan using cached data"
    )]
    pub incremental: bool,

    #[arg(
        long = "summary-only",
        help = "Show only summary statistics, not detailed duplicate groups"
    )]
    pub summary_only: bool,

    #[arg(
        short = 'f',
        long = "format",
        help = "Output format",
        value_enum,
        default_value = "text"
    )]
    pub output_format: OutputFormat,

    #[arg(
        short = 'o',
        long = "output",
        help = "Output file path (stdout if not specified)"
    )]
    pub output_file: Option<PathBuf>,

    #[arg(
        long = "interactive",
        help = "Interactive mode for duplicate resolution"
    )]
    pub interactive: bool,
}

impl Cli {
    pub fn to_scan_config(&self) -> ScanConfig {
        let paths = if self.paths.is_empty() {
            vec![PathBuf::from(".")]
        } else {
            self.paths.clone()
        };

        let mut config = ScanConfig::new()
            .with_paths(paths)
            .with_min_size(self.min_size)
            .with_follow_symlinks(self.follow_symlinks);

        if let Some(max_depth) = self.max_depth {
            config = config.with_max_depth(max_depth);
        }

        config.ignore_patterns.extend(self.ignore_patterns.iter().cloned());
        config.partial_hash_size = self.partial_hash_size;
        config.use_mmap_threshold = self.mmap_threshold;
        config.thread_count = self.threads;
        config.hash_algorithm = self.hash_algorithm.clone().into();
        config.cross_filesystem = !self.no_cross_filesystem;
        config.cache_file = self.cache_file.clone();
        config.incremental = self.incremental;

        config
    }
}