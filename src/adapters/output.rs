use crate::domain::ScanResult;
use crate::ports::OutputPort;
use anyhow::Result;
use console::{style, Term};
use dialoguer::{theme::ColorfulTheme, Confirm, MultiSelect, Select};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

struct OutputWriter {
    output_file: Option<String>,
}

impl OutputWriter {
    fn new() -> Self {
        Self { output_file: None }
    }

    fn with_file(path: &Path) -> Result<Self> {
        Ok(Self {
            output_file: Some(path.to_string_lossy().to_string()),
        })
    }

    fn write_content(&self, content: &str) -> Result<()> {
        match &self.output_file {
            Some(path) => {
                std::fs::write(path, content)?;
            }
            None => {
                print!("{}", content);
            }
        }
        Ok(())
    }
}

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

pub struct JsonOutputAdapter {
    writer: OutputWriter,
}

impl JsonOutputAdapter {
    pub fn new() -> Self {
        Self { writer: OutputWriter::new() }
    }

    pub fn with_file(path: &Path) -> Result<Self> {
        Ok(Self {
            writer: OutputWriter::with_file(path)?,
        })
    }

    pub fn with_stdout() -> Self {
        Self {
            writer: OutputWriter::new(),
        }
    }
}

impl OutputPort for JsonOutputAdapter {
    fn write_results(&self, results: &ScanResult) -> Result<()> {
        let json = serde_json::to_string_pretty(results)?;
        self.writer.write_content(&format!("{}\n", json))
    }
}

pub struct CsvOutputAdapter {
    writer: OutputWriter,
}

impl CsvOutputAdapter {
    pub fn new() -> Self {
        Self { writer: OutputWriter::new() }
    }

    pub fn with_file(path: &Path) -> Result<Self> {
        Ok(Self {
            writer: OutputWriter::with_file(path)?,
        })
    }

    pub fn with_stdout() -> Self {
        Self {
            writer: OutputWriter::new(),
        }
    }

    fn format_csv_string(&self, results: &ScanResult) -> Result<String> {
        let mut output = String::new();
        output.push_str("group_id,hash,file_path,file_size,group_size,wasted_space\n");
        for (group_id, group) in results.duplicates.iter().enumerate() {
            for file in &group.files {
                output.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    group_id + 1,
                    group.hash,
                    file.path.display(),
                    file.size,
                    group.total_size,
                    group.wasted_space()
                ));
            }
        }
        
        Ok(output)
    }
}

impl OutputPort for CsvOutputAdapter {
    fn write_results(&self, results: &ScanResult) -> Result<()> {
        let csv_content = self.format_csv_string(results)?;
        self.writer.write_content(&csv_content)
    }
}

pub struct TreeOutputAdapter {
    writer: OutputWriter,
}

impl TreeOutputAdapter {
    pub fn new() -> Self {
        Self { writer: OutputWriter::new() }
    }

    pub fn with_file(path: &Path) -> Result<Self> {
        Ok(Self {
            writer: OutputWriter::with_file(path)?,
        })
    }

    pub fn with_stdout() -> Self {
        Self {
            writer: OutputWriter::new(),
        }
    }

    fn format_tree_output(&self, results: &ScanResult) -> String {
        let mut output = String::new();
        output.push_str("=== Duplicate File Tree ===\n");
        output.push_str(&format!("Total files scanned: {}\n", results.total_files_scanned));
        output.push_str(&format!("Total size scanned: {:.2} MB\n", results.total_size_scanned as f64 / 1_048_576.0));
        output.push_str(&format!("Duplicate groups found: {}\n", results.duplicate_groups()));
        output.push_str(&format!("Total duplicate files: {}\n", results.total_duplicate_files()));
        output.push_str(&format!("Wasted space: {:.2} MB\n\n", results.total_wasted_space as f64 / 1_048_576.0));
        
        if results.duplicates.is_empty() {
            output.push_str("No duplicates found!\n");
            return output;
        }

        for (i, group) in results.duplicates.iter().enumerate() {
            output.push_str(&format!("Duplicate Group {} [{} files, {:.2} MB each, {:.2} MB wasted]\n", 
                i + 1, 
                group.files.len(),
                group.files[0].size as f64 / 1_048_576.0,
                group.wasted_space() as f64 / 1_048_576.0
            ));
            
            output.push_str(&format!("|-- Hash: {}\n", &group.hash[..16]));
            let mut dir_files: HashMap<PathBuf, Vec<&crate::domain::FileMetadata>> = HashMap::new();
            for file in &group.files {
                if let Some(parent) = file.path.parent() {
                    dir_files.entry(parent.to_path_buf()).or_default().push(file);
                }
            }
            
            let mut sorted_dirs: Vec<_> = dir_files.keys().collect();
            sorted_dirs.sort();
            
            for (dir_idx, dir) in sorted_dirs.iter().enumerate() {
                let is_last_dir = dir_idx == sorted_dirs.len() - 1;
                let dir_prefix = if is_last_dir { "`-- " } else { "|-- " };
                let file_prefix = if is_last_dir { "    " } else { "|   " };
                
                output.push_str(&format!("{}{}/\n", dir_prefix, dir.display()));
                
                let files = &dir_files[*dir];
                for (file_idx, file) in files.iter().enumerate() {
                    let is_last_file = file_idx == files.len() - 1;
                    let file_marker = if is_last_file { "`-- " } else { "|-- " };
                    
                    if let Some(filename) = file.path.file_name() {
                        output.push_str(&format!("{}{}{}\n", file_prefix, file_marker, filename.to_string_lossy()));
                    }
                }
            }
            
            if i < results.duplicates.len() - 1 {
                output.push('\n');
            }
        }

        output
    }
}

impl OutputPort for TreeOutputAdapter {
    fn write_results(&self, results: &ScanResult) -> Result<()> {
        let output = self.format_tree_output(results);
        self.writer.write_content(&output)
    }
}

pub struct InteractiveOutputAdapter {
    term: Term,
}

impl InteractiveOutputAdapter {
    pub fn new() -> Self {
        Self {
            term: Term::stdout(),
        }
    }

    fn ensure_cursor_visible(&self) {
        let _ = self.term.show_cursor();
    }

    fn get_bulk_deletion_confirmation(&self, file_count: usize, operation_description: &str) -> Result<bool> {
        println!("\n{}", style("WARNING! BULK DELETE MAY BREAK THINGS!").bold().red());
        println!("{}", operation_description);
        println!();
        println!("Deleting duplicates can cause data loss, break applications that reference");
        println!("these files, or remove important backups and versioned copies. Files in");
        println!("different locations may serve different purposes even if they appear identical.");
        println!("This could range from losing personal documents to breaking system components.");
        println!();
        println!("This action CANNOT be undone!");
        
        let first_confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you understand that this will permanently delete files?")
            .default(false)
            .interact()?;

        if !first_confirm {
            println!("Operation cancelled.");
            self.ensure_cursor_visible();
            return Ok(false);
        }

        let second_confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Are you SURE you want to delete {} files?", file_count))
            .default(false)
            .interact()?;

        if !second_confirm {
            println!("Operation cancelled.");
            self.ensure_cursor_visible();
            return Ok(false);
        }

        let final_confirm = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Last chance, really delete these files?")
            .default(false)
            .interact()?;

        if !final_confirm {
            println!("Operation cancelled.");
            self.ensure_cursor_visible();
            return Ok(false);
        }

        Ok(true)
    }

    fn review_all_groups(&self, results: &ScanResult) -> Result<()> {
        for (i, group) in results.duplicates.iter().enumerate() {
            println!("\n{}", style(format!("Group {} of {}", i + 1, results.duplicates.len())).bold());
            println!("Size: {:.2} MB each ({:.2} MB wasted)", 
                     group.files[0].size as f64 / 1_048_576.0,
                     group.wasted_space() as f64 / 1_048_576.0);
            
            for (j, file) in group.files.iter().enumerate() {
                let metadata = fs::metadata(&file.path).ok();
                let modified = metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        match t.duration_since(std::time::UNIX_EPOCH) {
                            Ok(duration) => {
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                let file_time = duration.as_secs();
                                
                                if now > file_time {
                                    let diff = now - file_time;
                                    if diff < 60 {
                                        "just now".to_string()
                                    } else if diff < 3600 {
                                        format!("{}m ago", diff / 60)
                                    } else if diff < 86400 {
                                        format!("{}h ago", diff / 3600)
                                    } else if diff < 2592000 {
                                        format!("{}d ago", diff / 86400)
                                    } else {
                                        format!("{}mo ago", diff / 2592000)
                                    }
                                } else {
                                    "future".to_string()
                                }
                            }
                            Err(_) => "unknown".to_string()
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string());
                
                println!("  [{}] {} ({})", j + 1, file.path.display(), modified);
            }

            let file_names: Vec<String> = group.files.iter()
                .enumerate()
                .map(|(idx, f)| format!("[{}] {}", idx + 1, f.path.display()))
                .collect();

            let selections = MultiSelect::with_theme(&ColorfulTheme::default())
                .with_prompt("Select files to delete (space to select, enter to confirm)")
                .items(&file_names)
                .interact()?;

            if selections.is_empty() {
                println!("No files selected.");
                continue;
            }

            let files_to_delete: Vec<&Path> = selections.iter()
                .map(|&idx| group.files[idx].path.as_path())
                .collect();

            let confirm = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Delete {} selected files?", files_to_delete.len()))
                .default(false)
                .interact()?;

            if confirm {
                for path in files_to_delete {
                    match fs::remove_file(path) {
                        Ok(_) => println!("Deleted: {}", path.display()),
                        Err(e) => println!("Failed to delete {}: {}", path.display(), e),
                    }
                }
            } else {
                println!("Skipped.");
            }
        }

        Ok(())
    }

    fn auto_delete_by_age(&self, results: &ScanResult, keep_oldest: bool) -> Result<()> {
        let age_type = if keep_oldest { "oldest" } else { "newest" };
        let description = format!("This will permanently delete {} duplicate files. Only the {} file in each group will be kept.", 
                                 results.total_duplicate_files(), age_type);
        
        if !self.get_bulk_deletion_confirmation(results.total_duplicate_files(), &description)? {
            return Ok(());
        }

        let mut deleted_count = 0;
        let mut deleted_size = 0u64;

        for group in &results.duplicates {
            let mut sorted_files = group.files.clone();
            sorted_files.sort_by_key(|f| f.modified);
            
            let files_to_delete = if keep_oldest {
                &sorted_files[1..]
            } else {
                &sorted_files[..sorted_files.len() - 1]
            };

            for file in files_to_delete {
                match fs::remove_file(&file.path) {
                    Ok(_) => {
                        println!("{} {}", style("Deleted:").green(), file.path.display());
                        deleted_count += 1;
                        deleted_size += file.size;
                    }
                    Err(e) => {
                        println!("{} {}: {}", style("Error deleting").red(), file.path.display(), e);
                    }
                }
            }
        }

        println!("\n{}", style("DELETION SUMMARY:").bold().green());
        println!("Deleted {} files", deleted_count);
        println!("Freed {:.2} MB", deleted_size as f64 / 1_048_576.0);

        Ok(())
    }

    fn auto_delete_by_directory(&self, results: &ScanResult) -> Result<()> {
        println!("Enter directory path to keep (duplicates outside this directory will be deleted):");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let preferred_dir = input.trim();

        if preferred_dir.is_empty() {
            println!("No directory specified. Operation cancelled.");
            self.ensure_cursor_visible();
            return Ok(());
        }

        let mut files_to_delete_count = 0;
        for group in &results.duplicates {
            files_to_delete_count += group.files.iter()
                .filter(|f| !f.path.starts_with(preferred_dir))
                .count();
        }

        let description = format!("This will delete {} files outside of '{}'. Files inside '{}' will be kept. Deleting files outside your chosen directory can remove critical system files, application dependencies, or important documents stored elsewhere.", 
                                 files_to_delete_count, preferred_dir, preferred_dir);
        
        if !self.get_bulk_deletion_confirmation(files_to_delete_count, &description)? {
            return Ok(());
        }

        let mut deleted_count = 0;
        let mut deleted_size = 0u64;

        for group in &results.duplicates {
            let preferred_file = group.files.iter()
                .find(|f| f.path.starts_with(preferred_dir));

            if preferred_file.is_none() {
                println!("{}", style(format!("No files in preferred directory for group with hash {}...", &group.hash[..8])).yellow());
                continue;
            }

            let files_to_delete: Vec<_> = group.files.iter()
                .filter(|f| !f.path.starts_with(preferred_dir))
                .collect();

            for file in files_to_delete {
                match fs::remove_file(&file.path) {
                    Ok(_) => {
                        println!("{} {}", style("Deleted:").green(), file.path.display());
                        deleted_count += 1;
                        deleted_size += file.size;
                    }
                    Err(e) => {
                        println!("{} {}: {}", style("Error deleting").red(), file.path.display(), e);
                    }
                }
            }
        }

        println!("\n{}", style("DELETION SUMMARY:").bold().green());
        println!("Deleted {} files", deleted_count);
        println!("Freed {:.2} MB", deleted_size as f64 / 1_048_576.0);

        Ok(())
    }
}

impl OutputPort for InteractiveOutputAdapter {
    fn write_results(&self, results: &ScanResult) -> Result<()> {
        let term_clone = self.term.clone();
        ctrlc::set_handler(move || {
            let _ = term_clone.show_cursor();
            std::process::exit(0);
        }).expect("Error setting Ctrl+C handler");

        self.term.clear_screen()?;
        
        println!("{}", style("rdupe - Duplicate File Manager").bold());
        println!("Found {} duplicate groups ({} files, {:.2} MB wasted space)",
                 results.duplicate_groups(),
                 results.total_duplicate_files(),
                 results.total_wasted_space as f64 / 1_048_576.0);
        
        if results.duplicates.is_empty() {
            println!("No duplicates found.");
            self.ensure_cursor_visible();
            return Ok(());
        }

        println!("\nActions:");
        let actions = vec![
            "Review each group individually",
            "Delete all duplicates (keep newest)",
            "Delete all duplicates (keep oldest)", 
            "Delete duplicates outside directory",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .items(&actions)
            .default(0)
            .interact()?;

        match selection {
            0 => self.review_all_groups(results)?,
            1 => self.auto_delete_by_age(results, false)?,
            2 => self.auto_delete_by_age(results, true)?,
            3 => self.auto_delete_by_directory(results)?,
            4 => {
                println!("Exiting without changes.");
                self.ensure_cursor_visible();
                return Ok(());
            }
            _ => unreachable!(),
        }

        self.ensure_cursor_visible();
        Ok(())
    }
}