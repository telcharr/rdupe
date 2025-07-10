use crate::ports::ProgressPort;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

pub struct ProgressBarAdapter {
    bar: Arc<ProgressBar>,
    quiet: bool,
}

impl ProgressBarAdapter {
    pub fn new() -> Self {
        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {percent:>3}% {msg} (ETA: {eta})")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏ "),
        );
        Self { 
            bar: Arc::new(bar),
            quiet: false,
        }
    }

    pub fn new_quiet() -> Self {
        let bar = ProgressBar::hidden();
        Self { 
            bar: Arc::new(bar),
            quiet: true,
        }
    }

    pub fn with_quiet(mut self, quiet: bool) -> Self {
        self.quiet = quiet;
        if quiet {
            self.bar = Arc::new(ProgressBar::hidden());
        }
        self
    }
}

impl ProgressPort for ProgressBarAdapter {
    fn start(&self, total: u64) {
        if self.quiet {
            return;
        }
        
        self.bar.set_length(total);
        self.bar.set_message("Scanning files...");
        self.bar.enable_steady_tick(std::time::Duration::from_millis(100));
    }

    fn update(&self, processed: u64) {
        if self.quiet {
            return;
        }
        
        self.bar.set_position(processed);
        let total = self.bar.length().unwrap_or(1);
        let percent = (processed as f64 / total as f64) * 100.0;
        
        if percent < 50.0 {
            self.bar.set_message("Hashing files...");
        } else {
            self.bar.set_message("Processing duplicates...");
        }
    }

    fn finish(&self) {
        if self.quiet {
            return;
        }
        
        self.bar.disable_steady_tick();
        self.bar.finish_with_message("✓ Scan complete!");
    }
}