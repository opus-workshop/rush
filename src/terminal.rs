use nix::unistd::{getpgrp, Pid};
use nix::libc;
use std::os::unix::io::RawFd;
use anyhow::{anyhow, Result};

/// Terminal control for managing foreground process groups
#[derive(Clone)]
pub struct TerminalControl {
    shell_pgid: Pid,
    terminal_fd: RawFd,
    is_interactive: bool,
}

impl TerminalControl {
    /// Create a new terminal control instance
    pub fn new() -> Self {
        let terminal_fd = 0; // stdin
        let shell_pgid = getpgrp();

        // Check if we're interactive by verifying:
        // 1. stdin is a terminal
        // 2. We're in the foreground process group
        let is_interactive = unsafe { libc::isatty(terminal_fd) } == 1
            && Self::tcgetpgrp_raw(terminal_fd).map(|fg_pgid| fg_pgid == shell_pgid.as_raw()).unwrap_or(false);

        Self {
            shell_pgid,
            terminal_fd,
            is_interactive,
        }
    }

    /// Get the foreground process group using libc directly
    fn tcgetpgrp_raw(fd: RawFd) -> Result<i32> {
        let pgid = unsafe { libc::tcgetpgrp(fd) };
        if pgid < 0 {
            Err(anyhow!("tcgetpgrp failed"))
        } else {
            Ok(pgid)
        }
    }

    /// Set the foreground process group using libc directly
    fn tcsetpgrp_raw(fd: RawFd, pgid: i32) -> Result<()> {
        let result = unsafe { libc::tcsetpgrp(fd, pgid) };
        if result != 0 {
            Err(anyhow!("tcsetpgrp failed"))
        } else {
            Ok(())
        }
    }

    /// Check if the shell is running interactively with terminal control
    pub fn is_interactive(&self) -> bool {
        self.is_interactive
    }

    /// Give terminal control to the specified process group
    pub fn give_terminal_to(&self, pgid: Pid) -> Result<()> {
        if !self.is_interactive {
            return Ok(()); // Not interactive, nothing to do
        }

        Self::tcsetpgrp_raw(self.terminal_fd, pgid.as_raw())
            .map_err(|e| anyhow!("Failed to give terminal control to process group {}: {}", pgid, e))
    }

    /// Reclaim terminal control for the shell
    pub fn reclaim_terminal(&self) -> Result<()> {
        if !self.is_interactive {
            return Ok(()); // Not interactive, nothing to do
        }

        Self::tcsetpgrp_raw(self.terminal_fd, self.shell_pgid.as_raw())
            .map_err(|e| anyhow!("Failed to reclaim terminal control: {}", e))
    }

    /// Get the current foreground process group
    pub fn get_foreground_pgid(&self) -> Result<Pid> {
        Self::tcgetpgrp_raw(self.terminal_fd)
            .map(Pid::from_raw)
            .map_err(|e| anyhow!("Failed to get foreground process group: {}", e))
    }
}

impl Default for TerminalControl {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_control_creation() {
        let terminal = TerminalControl::new();
        // Should not panic
    }

    #[test]
    fn test_terminal_control_clone() {
        let terminal = TerminalControl::new();
        let terminal2 = terminal.clone();
        assert_eq!(terminal.is_interactive(), terminal2.is_interactive());
    }

    #[test]
    fn test_reclaim_terminal() {
        let terminal = TerminalControl::new();
        // Should succeed or fail gracefully
        let _ = terminal.reclaim_terminal();
    }
}
