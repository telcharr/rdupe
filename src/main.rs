use clap::Parser;
use rdupe::adapters::{
    ConsoleOutputAdapter, CsvOutputAdapter, FileSystemAdapter, InteractiveOutputAdapter, 
    JsonOutputAdapter, MultiAlgorithmHasher, ProgressBarAdapter, TreeOutputAdapter
};
use rdupe::cli::{Cli, OutputFormat};
use rdupe::ports::OutputPort;
use rdupe::services::DuplicateFinderService;
use std::process;

fn main() {
    let args = Cli::parse();
    let config = args.to_scan_config();
    let filesystem = FileSystemAdapter::new();
    let hasher = MultiAlgorithmHasher::new().with_mmap_threshold(config.use_mmap_threshold);
    let progress = ProgressBarAdapter::new().with_quiet(args.quiet);

    let finder = DuplicateFinderService::new(filesystem, hasher, progress);
    
    match finder.find_duplicates(&config) {
        Ok(results) => {
            if args.interactive {
                let interactive_output = InteractiveOutputAdapter::new();
                if let Err(e) = interactive_output.write_results(&results) {
                    eprintln!("Error in interactive mode: {}", e);
                    process::exit(1);
                }
            } else {
                let output: Box<dyn OutputPort> = match args.output_format {
                    OutputFormat::Text => Box::new(ConsoleOutputAdapter::new().with_summary_only(args.summary_only)),
                    OutputFormat::Json => {
                        if let Some(ref path) = args.output_file {
                            Box::new(JsonOutputAdapter::with_file(path).unwrap_or_else(|e| {
                                eprintln!("Error creating output file: {}", e);
                                process::exit(1);
                            }))
                        } else {
                            Box::new(JsonOutputAdapter::with_stdout())
                        }
                    }
                    OutputFormat::Csv => {
                        if let Some(ref path) = args.output_file {
                            Box::new(CsvOutputAdapter::with_file(path).unwrap_or_else(|e| {
                                eprintln!("Error creating output file: {}", e);
                                process::exit(1);
                            }))
                        } else {
                            Box::new(CsvOutputAdapter::with_stdout())
                        }
                    }
                    OutputFormat::Tree => {
                        if let Some(ref path) = args.output_file {
                            Box::new(TreeOutputAdapter::with_file(path).unwrap_or_else(|e| {
                                eprintln!("Error creating output file: {}", e);
                                process::exit(1);
                            }))
                        } else {
                            Box::new(TreeOutputAdapter::with_stdout())
                        }
                    }
                };

                if let Err(e) = output.write_results(&results) {
                    eprintln!("Error writing results: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Error during scan: {}", e);
            process::exit(1);
        }
    }
}
