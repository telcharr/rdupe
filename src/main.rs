use clap::Parser;
use rdupe::adapters::{ConsoleOutputAdapter, FileSystemAdapter, MultiAlgorithmHasher, ProgressBarAdapter};
use rdupe::cli::Cli;
use rdupe::ports::OutputPort;
use rdupe::services::DuplicateFinderService;
use std::process;

fn main() {
    let args = Cli::parse();
    let config = args.to_scan_config();
    let filesystem = FileSystemAdapter::new();
    let hasher = MultiAlgorithmHasher::new().with_mmap_threshold(config.use_mmap_threshold);
    let output = ConsoleOutputAdapter::new().with_summary_only(args.summary_only);
    let progress = if args.quiet {
        ProgressBarAdapter::new() // TODO: Create a NoOpProgressAdapter for quiet mode
    } else {
        ProgressBarAdapter::new()
    };

    let finder = DuplicateFinderService::new(filesystem, hasher, progress);
    match finder.find_duplicates(&config) {
        Ok(results) => {
            if let Err(e) = output.write_results(&results) {
                eprintln!("Error writing results: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error during scan: {}", e);
            process::exit(1);
        }
    }
}
