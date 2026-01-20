use anyhow::Result;
use signal_hook::consts::{SIGINT, SIGTERM, SIGHUP};
use signal_hook::iterator::Signals;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

/// Global flag indicating if a signal was received
static SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

/// The signal that was received (0 if none)
static SIGNAL_NUMBER: AtomicI32 = AtomicI32::new(0);

/// Signal handler state shared between main thread and signal handler thread
#[derive(Clone)]
pub struct SignalHandler {
    shutdown_flag: Arc<AtomicBool>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new() -> Self {
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Setup signal handlers for SIGINT, SIGTERM, and SIGHUP
    pub fn setup(&self) -> Result<()> {
        let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGHUP])?;
        let shutdown_flag = Arc::clone(&self.shutdown_flag);

        thread::spawn(move || {
            for sig in signals.forever() {
                match sig {
                    SIGINT => {
                        SIGNAL_RECEIVED.store(true, Ordering::SeqCst);
                        SIGNAL_NUMBER.store(SIGINT, Ordering::SeqCst);
                        shutdown_flag.store(true, Ordering::SeqCst);
                    }
                    SIGTERM => {
                        SIGNAL_RECEIVED.store(true, Ordering::SeqCst);
                        SIGNAL_NUMBER.store(SIGTERM, Ordering::SeqCst);
                        shutdown_flag.store(true, Ordering::SeqCst);
                    }
                    SIGHUP => {
                        SIGNAL_RECEIVED.store(true, Ordering::SeqCst);
                        SIGNAL_NUMBER.store(SIGHUP, Ordering::SeqCst);
                        shutdown_flag.store(true, Ordering::SeqCst);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Check if a signal was received
    pub fn signal_received(&self) -> bool {
        SIGNAL_RECEIVED.load(Ordering::SeqCst)
    }

    /// Get the signal number that was received
    pub fn signal_number(&self) -> i32 {
        SIGNAL_NUMBER.load(Ordering::SeqCst)
    }

    /// Check if shutdown was requested
    pub fn should_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Reset the signal state
    pub fn reset(&self) {
        SIGNAL_RECEIVED.store(false, Ordering::SeqCst);
        SIGNAL_NUMBER.store(0, Ordering::SeqCst);
        self.shutdown_flag.store(false, Ordering::SeqCst);
    }

    /// Get the exit code for the received signal
    pub fn exit_code(&self) -> i32 {
        match self.signal_number() {
            SIGINT => 130,   // Standard exit code for SIGINT (128 + 2)
            SIGTERM => 143,  // Standard exit code for SIGTERM (128 + 15)
            SIGHUP => 129,   // Standard exit code for SIGHUP (128 + 1)
            _ => 1,
        }
    }
}

impl Default for SignalHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Global signal checking function
pub fn signal_received() -> bool {
    SIGNAL_RECEIVED.load(Ordering::SeqCst)
}

/// Get the current signal number
pub fn signal_number() -> i32 {
    SIGNAL_NUMBER.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_handler_creation() {
        let handler = SignalHandler::new();
        assert!(!handler.should_shutdown());
        assert!(!handler.signal_received());
        assert_eq!(handler.signal_number(), 0);
    }

    #[test]
    fn test_signal_handler_setup() {
        let handler = SignalHandler::new();
        assert!(handler.setup().is_ok());
    }

    #[test]
    fn test_signal_handler_reset() {
        let handler = SignalHandler::new();
        handler.reset();
        assert!(!handler.signal_received());
        assert_eq!(handler.signal_number(), 0);
    }

    #[test]
    fn test_exit_codes() {
        let handler = SignalHandler::new();

        // Set SIGINT and check exit code
        SIGNAL_NUMBER.store(SIGINT, Ordering::SeqCst);
        assert_eq!(handler.exit_code(), 130);

        // Set SIGTERM and check exit code
        SIGNAL_NUMBER.store(SIGTERM, Ordering::SeqCst);
        assert_eq!(handler.exit_code(), 143);

        // Set SIGHUP and check exit code
        SIGNAL_NUMBER.store(SIGHUP, Ordering::SeqCst);
        assert_eq!(handler.exit_code(), 129);

        // Reset
        handler.reset();
    }

    #[test]
    fn test_sigint_handling() {
        // Verify the handler can be created and setup
        // Full integration testing is done in tests/signal_handling_tests.rs
        let handler = SignalHandler::new();
        assert!(handler.setup().is_ok());
    }
}
