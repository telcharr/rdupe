use crate::domain::ScanResult;
use crate::ports::OutputPort;
use anyhow::Result;

pub struct ConsoleOutputAdapter {
    summary_only: bool,
}

impl ConsoleOutputAdapter {
    pub fn new() -> Self {
        Self {
            summary_only: false,
        }
    }

    pub fn with_summary_only(mut self, summary_only: bool) -> Self {
        self.summary_only = summary_only;
        self
    }
}

impl OutputPort for ConsoleOutputAdapter {
    fn write_results(&self, results: &ScanResult) -> Result<()> {
        println!("\n=== Duplicate File Scan Results ===");
        println!("Total files scanned: {}", results.total_files_scanned);
        println!("Total size scanned: {:.2} MB", results.total_size_scanned as f64 / 1_048_576.0);
        println!("Duplicate groups found: {}", results.duplicate_groups());
        println!("Total duplicate files: {}", results.total_duplicate_files());
        println!("Wasted space: {:.2} MB", results.total_wasted_space as f64 / 1_048_576.0);
        
        if results.duplicates.is_empty() {
            println!("\nNo duplicates found!");
            return Ok(());
        }

        if !self.summary_only {
            println!("\n=== Duplicate Groups ===");
            for (i, group) in results.duplicates.iter().enumerate() {
                println!("\nGroup {} (Hash: {})", i + 1, &group.hash[..16]);
                println!("  Size: {:.2} MB each", group.files[0].size as f64 / 1_048_576.0);
                println!("  Wasted space: {:.2} MB", group.wasted_space() as f64 / 1_048_576.0);
                println!("  Files:");
                
                for file in &group.files {
                    println!("    {}", file.path.display());
                }
            }
        }

        Ok(())
    }
}