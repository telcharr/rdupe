use crate::ports::ProgressPort;
use indicatif::{ProgressBar, ProgressStyle};
use std::sync::Arc;

pub struct ProgressBarAdapter {
    bar: Arc<ProgressBar>,
}

impl ProgressBarAdapter {
    pub fn new() -> Self {
        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("##-"),
        );
        Self { bar: Arc::new(bar) }
    }
}

impl ProgressPort for ProgressBarAdapter {
    fn start(&self, total: u64) {
        self.bar.set_length(total);
        self.bar.set_message("Scanning files...");
    }

    fn update(&self, processed: u64) {
        self.bar.set_position(processed);
    }

    fn finish(&self) {
        self.bar.finish_with_message("Scan complete!");
    }
}