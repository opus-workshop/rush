use crate::daemon::protocol::{Message, SessionInit, ExecutionResult, read_message, write_message};
use anyhow::{anyhow, Result};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{fork, ForkResult, Pid};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Maximum concurrent sessions
const MAX_CONCURRENT_SESSIONS: usize = 100;

/// Session identifier (process ID of worker)
pub type SessionId = i32;

/// Handle to a session worker
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub id: SessionId,
    pub worker_pid: Pid,
    pub created_at: Instant,
}

/// Main daemon server
pub struct DaemonServer {
    socket_path: PathBuf,
    listener: Option<UnixListener>,
    sessions: HashMap<SessionId, SessionHandle>,
    shutdown: Arc<AtomicBool>,
}

impl DaemonServer {
    /// Create a new daemon server
    pub fn new(socket_path: PathBuf) -> Result<Self> {
        Ok(Self {
            socket_path,
            listener: None,
            sessions: HashMap::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Create the daemon directory with secure permissions
    fn create_daemon_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?;

        let daemon_dir = home.join(".rush");

        if !daemon_dir.exists() {
            fs::create_dir(&daemon_dir)?;

            // Set directory permissions to 0700 (owner only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&daemon_dir)?.permissions();
                perms.set_mode(0o700);
                fs::set_permissions(&daemon_dir, perms)?;
            }
        }

        Ok(daemon_dir)
    }

    /// Start the daemon server
    pub fn start(&mut self) -> Result<()> {
        // Setup signal handlers
        self.setup_signal_handlers()?;

        // Bind to socket
        self.bind_socket()?;

        // Write PID file
        self.write_pid_file()?;

        println!("Rush daemon started on {}", self.socket_path.display());

        // Enter accept loop
        self.accept_loop()?;

        Ok(())
    }

    /// Bind the Unix socket
    fn bind_socket(&mut self) -> Result<()> {
        // Remove stale socket if it exists
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path)?;
        }

        // Create listener
        let listener = UnixListener::bind(&self.socket_path)?;

        // Set socket permissions to 0600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.socket_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.socket_path, perms)?;
        }

        self.listener = Some(listener);

        Ok(())
    }

    /// Setup signal handlers for graceful shutdown
    fn setup_signal_handlers(&self) -> Result<()> {
        let shutdown = self.shutdown.clone();

        // Handle SIGTERM for graceful shutdown
        signal_hook::flag::register(signal::SIGTERM as i32, shutdown.clone())?;

        // Handle SIGINT (Ctrl-C) for graceful shutdown
        signal_hook::flag::register(signal::SIGINT as i32, shutdown.clone())?;

        Ok(())
    }

    /// Main accept loop
    fn accept_loop(&mut self) -> Result<()> {
        // Take ownership of listener temporarily for borrowing
        let listener = self.listener.take()
            .ok_or_else(|| anyhow!("Socket not bound"))?;

        // Set non-blocking mode for accept to check shutdown flag
        listener.set_nonblocking(true)?;

        let result = (|| -> Result<()> {
            while !self.shutdown.load(Ordering::Relaxed) {
                // Try to accept a connection
                match listener.accept() {
                    Ok((stream, _addr)) => {
                        // Reset to blocking mode for the connection
                        stream.set_nonblocking(false)?;

                        // Check session limit
                        if self.sessions.len() >= MAX_CONCURRENT_SESSIONS {
                            eprintln!("Warning: Maximum concurrent sessions reached, rejecting connection");
                            let _ = Self::send_error(&stream, "Maximum concurrent sessions reached", 0);
                            continue;
                        }

                        // Accept the connection (fork worker)
                        if let Err(e) = self.accept_connection(stream) {
                            eprintln!("Error accepting connection: {}", e);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No connection ready, reap workers and sleep briefly
                        self.reap_workers();
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        eprintln!("Error accepting connection: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
            Ok(())
        })();

        // Restore listener
        self.listener = Some(listener);

        println!("Shutting down daemon...");
        self.shutdown_gracefully()?;

        result
    }

    /// Accept a client connection and fork a worker
    fn accept_connection(&mut self, stream: UnixStream) -> Result<()> {
        // Fork a worker process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent: register session
                let handle = SessionHandle {
                    id: child.as_raw(),
                    worker_pid: child,
                    created_at: Instant::now(),
                };

                self.sessions.insert(child.as_raw(), handle);

                // Close stream in parent (worker owns it)
                drop(stream);

                Ok(())
            }
            Ok(ForkResult::Child) => {
                // Child: become session worker
                // This function does not return
                Self::run_worker(stream);
            }
            Err(e) => {
                Err(anyhow!("Failed to fork worker: {}", e))
            }
        }
    }

    /// Run the session worker (in child process)
    fn run_worker(stream: UnixStream) -> ! {
        let exit_code = match Self::handle_session(stream) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Worker error: {}", e);
                1
            }
        };

        std::process::exit(exit_code);
    }

    /// Handle a session in the worker process
    fn handle_session(mut stream: UnixStream) -> Result<i32> {
        // Set a timeout for reading the initial message
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

        // Read session init message
        let (msg, msg_id) = match read_message(&mut stream) {
            Ok(result) => result,
            Err(e) => {
                // Client disconnected without sending a message (e.g., status check)
                // This is normal, just exit quietly
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut {
                    return Ok(0);
                }
                return Err(anyhow!("Failed to read message: {}", e));
            }
        };

        // Clear timeout for rest of session
        stream.set_read_timeout(None)?;

        let session_init = match msg {
            Message::SessionInit(init) => init,
            _ => {
                Self::send_error(&stream, "Expected SessionInit message", msg_id)?;
                return Ok(1);
            }
        };

        // Execute the session
        let exit_code = Self::execute_session(&session_init)?;

        // Send result back to client
        let result = ExecutionResult {
            exit_code,
            stdout_len: 0,  // TODO: capture stdout/stderr lengths
            stderr_len: 0,
        };

        write_message(&mut stream, &Message::ExecutionResult(result), msg_id)
            .map_err(|e| anyhow!("Failed to send result: {}", e))?;

        // Ensure data is flushed before worker exits
        stream.flush()?;

        Ok(0)
    }

    /// Execute a session command
    fn execute_session(init: &SessionInit) -> Result<i32> {
        // Set working directory
        std::env::set_current_dir(&init.working_dir)?;

        // Set environment variables
        for (key, value) in &init.env {
            std::env::set_var(key, value);
        }

        // Parse and execute command
        // For now, just execute the args as a command via -c flag
        if init.args.len() >= 2 && init.args[0] == "-c" {
            let command = &init.args[1];

            // Create executor
            let mut executor = crate::executor::Executor::new();

            // Parse
            let tokens = match crate::lexer::Lexer::tokenize(command) {
                Ok(tokens) => tokens,
                Err(e) => {
                    eprintln!("Lexer error: {}", e);
                    return Ok(2);
                }
            };
            let mut parser = crate::parser::Parser::new(tokens);

            match parser.parse() {
                Ok(ast) => {
                    match executor.execute(ast) {
                        Ok(result) => Ok(result.exit_code),
                        Err(e) => {
                            eprintln!("Execution error: {}", e);
                            Ok(1)
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    Ok(2)
                }
            }
        } else {
            Ok(0)
        }
    }

    /// Send an error message to the client
    fn send_error(stream: &UnixStream, message: &str, msg_id: u32) -> Result<()> {
        let mut stream = stream.try_clone()?;
        // For errors, we could define a custom error message type
        // For now, send an ExecutionResult with exit code 1
        let error_result = ExecutionResult {
            exit_code: 1,
            stdout_len: 0,
            stderr_len: message.len() as u64,
        };
        write_message(&mut stream, &Message::ExecutionResult(error_result), msg_id)
            .map_err(|e| anyhow!("Failed to send error: {}", e))
    }

    /// Reap finished worker processes
    fn reap_workers(&mut self) {
        let mut finished = Vec::new();

        for (session_id, handle) in &self.sessions {
            match waitpid(handle.worker_pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, _code)) => {
                    finished.push(pid.as_raw());
                }
                Ok(WaitStatus::Signaled(pid, _signal, _)) => {
                    finished.push(pid.as_raw());
                }
                Ok(WaitStatus::StillAlive) => {
                    // Still running
                }
                Err(e) => {
                    eprintln!("Error waiting for process {}: {}", session_id, e);
                    finished.push(*session_id);
                }
                _ => {
                    // Other statuses, keep monitoring
                }
            }
        }

        // Remove finished sessions
        for session_id in finished {
            self.sessions.remove(&session_id);
        }
    }

    /// Write PID file for daemon management
    fn write_pid_file(&self) -> Result<()> {
        let pid_path = self.socket_path.parent()
            .ok_or_else(|| anyhow!("Invalid socket path"))?
            .join("daemon.pid");

        let pid = std::process::id();
        fs::write(&pid_path, pid.to_string())?;

        Ok(())
    }

    /// Remove PID file
    fn remove_pid_file(&self) -> Result<()> {
        let pid_path = self.socket_path.parent()
            .ok_or_else(|| anyhow!("Invalid socket path"))?
            .join("daemon.pid");

        if pid_path.exists() {
            fs::remove_file(&pid_path)?;
        }

        Ok(())
    }

    /// Graceful shutdown
    fn shutdown_gracefully(&mut self) -> Result<()> {
        // Send SIGTERM to all worker processes
        for handle in self.sessions.values() {
            let _ = signal::kill(handle.worker_pid, Signal::SIGTERM);
        }

        // Wait for workers to exit (with timeout)
        let timeout = std::time::Duration::from_secs(5);
        let start = Instant::now();

        while !self.sessions.is_empty() && start.elapsed() < timeout {
            self.reap_workers();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Force kill any remaining workers
        for handle in self.sessions.values() {
            let _ = signal::kill(handle.worker_pid, Signal::SIGKILL);
        }

        // Final reap
        self.reap_workers();

        // Remove PID file
        let _ = self.remove_pid_file();

        // Remove socket file
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path)?;
        }

        Ok(())
    }

    /// Get the default socket path
    pub fn default_socket_path() -> Result<PathBuf> {
        let daemon_dir = Self::create_daemon_dir()?;
        Ok(daemon_dir.join("daemon.sock"))
    }
}

impl Drop for DaemonServer {
    fn drop(&mut self) {
        // Cleanup PID file
        let _ = self.remove_pid_file();

        // Cleanup socket on drop
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }
    }
}
