// Progress indicators for long-running operations

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Simple spinner animation frames
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Progress indicator that shows a spinner for long-running operations
pub struct ProgressIndicator {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    message: String,
}

impl ProgressIndicator {
    /// Create and start a new progress indicator
    pub fn new(message: impl Into<String>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let msg = message.into();
        let msg_clone = msg.clone();

        let handle = thread::spawn(move || {
            let mut frame = 0;
            let stderr = io::stderr();

            while running_clone.load(Ordering::Relaxed) {
                let spinner = SPINNER_FRAMES[frame % SPINNER_FRAMES.len()];

                // Use carriage return to overwrite, but stay on same line
                // \x1b[K clears from cursor to end of line to remove any leftover chars
                eprint!("\r\x1b[K{} {}...", spinner, msg_clone);
                let _ = stderr.lock().flush();

                thread::sleep(Duration::from_millis(80));
                frame += 1;
            }

            // Clear the line completely and move cursor back to start
            eprint!("\r\x1b[K");
            let _ = stderr.lock().flush();
        });

        Self {
            running,
            handle: Some(handle),
            message: msg,
        }
    }

    /// Stop the progress indicator
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Update the message while the indicator is running
    #[allow(dead_code)]
    pub fn update_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }
}

impl Drop for ProgressIndicator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Threshold in milliseconds - only show spinner if command takes longer than this
pub const PROGRESS_THRESHOLD_MS: u64 = 200;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_indicator() {
        let progress = ProgressIndicator::new("Testing");
        thread::sleep(Duration::from_millis(500));
        progress.stop();
        // If this doesn't crash, the test passes
    }

    #[test]
    fn test_progress_auto_cleanup() {
        {
            let _progress = ProgressIndicator::new("Auto cleanup");
            thread::sleep(Duration::from_millis(300));
        } // progress should stop here via Drop
        thread::sleep(Duration::from_millis(100));
        // If no crashes, test passes
    }
}
