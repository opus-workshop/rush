use crate::daemon::protocol::{Message, SessionInit, ExecutionResult, read_message, write_message};
use crate::daemon::worker_pool::{WorkerPool, PoolConfig};
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
use std::time::{Duration, Instant};

/// Maximum concurrent sessions
const MAX_CONCURRENT_SESSIONS: usize = 100;

/// Health check interval (how often to check workers)
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);

/// Ping timeout (worker must respond within this time)
const PING_TIMEOUT: Duration = Duration::from_secs(5);

/// Request timeout (worker must complete requests within this time)
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum respawn attempts for a failed worker
const MAX_RESPAWN_ATTEMPTS: u32 = 3;

/// Respawn cooldown period (prevent rapid respawn loops)
const RESPAWN_COOLDOWN: Duration = Duration::from_secs(60);

/// Session identifier (process ID of worker)
pub type SessionId = i32;

/// Worker health state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerHealthState {
    /// Worker is healthy and responsive
    Healthy,
    /// Worker hasn't responded to recent ping
    Unresponsive,
    /// Worker is processing a slow request (not necessarily hung)
    Slow,
    /// Worker has crashed or been killed
    Crashed,
    /// Worker is confirmed hung (no response to multiple pings)
    Hung,
}

/// Worker metrics
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    /// Total requests processed
    pub requests_processed: u64,
    /// Total requests failed
    pub requests_failed: u64,
    /// Last successful request time
    pub last_request_time: Option<Instant>,
    /// Last successful ping/pong time
    pub last_heartbeat: Instant,
    /// Number of consecutive failed health checks
    pub consecutive_failures: u32,
    /// Number of times this worker has been respawned
    pub respawn_count: u32,
    /// Time when worker was created/last respawned
    pub spawn_time: Instant,
}

impl Default for WorkerMetrics {
    fn default() -> Self {
        Self {
            requests_processed: 0,
            requests_failed: 0,
            last_request_time: None,
            last_heartbeat: Instant::now(),
            consecutive_failures: 0,
            respawn_count: 0,
            spawn_time: Instant::now(),
        }
    }
}

/// Worker state snapshot for reset between requests
#[derive(Debug, Clone)]
struct WorkerState {
    /// Original working directory at worker startup
    original_cwd: PathBuf,
    /// Original environment variables at worker startup
    original_env: HashMap<String, String>,
}

impl WorkerState {
    /// Capture current worker state
    fn capture() -> Result<Self> {
        Ok(Self {
            original_cwd: std::env::current_dir()
                .map_err(|e| anyhow!("Failed to get current directory: {}", e))?,
            original_env: std::env::vars().collect(),
        })
    }

    /// Reset worker to original state
    fn reset(&self) -> Result<()> {
        // Reset working directory
        std::env::set_current_dir(&self.original_cwd)
            .map_err(|e| anyhow!("Failed to restore working directory: {}", e))?;

        // Reset environment variables
        // 1. Remove variables that were added
        let current_env: HashMap<String, String> = std::env::vars().collect();
        for key in current_env.keys() {
            if !self.original_env.contains_key(key) {
                std::env::remove_var(key);
            }
        }

        // 2. Restore original variables (handles both modified and removed vars)
        for (key, value) in &self.original_env {
            std::env::set_var(key, value);
        }

        Ok(())
    }
}

/// Handle to a session worker
#[derive(Debug, Clone)]
pub struct SessionHandle {
    pub id: SessionId,
    pub worker_pid: Pid,
    pub created_at: Instant,
    /// Health state of the worker
    pub health_state: WorkerHealthState,
    /// Metrics for this worker
    pub metrics: WorkerMetrics,
    /// Last time we attempted to ping this worker
    pub last_ping_attempt: Option<Instant>,
}

/// Main daemon server
pub struct DaemonServer {
    socket_path: PathBuf,
    listener: Option<UnixListener>,
    sessions: HashMap<SessionId, SessionHandle>,
    shutdown: Arc<AtomicBool>,
    /// Optional worker pool (if None, uses fork-per-request)
    worker_pool: Option<WorkerPool>,
}

impl DaemonServer {
    /// Create a new daemon server
    pub fn new(socket_path: PathBuf) -> Result<Self> {
        Ok(Self {
            socket_path,
            listener: None,
            sessions: HashMap::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
            worker_pool: None,
        })
    }

    /// Enable worker pool mode with the specified configuration
    pub fn with_worker_pool(mut self, config: PoolConfig) -> Result<Self> {
        let pool = WorkerPool::new(config)?;
        eprintln!("Worker pool enabled with {} workers", pool.stats().total_workers);
        self.worker_pool = Some(pool);
        Ok(self)
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
            let mut last_health_check = Instant::now();
            
            while !self.shutdown.load(Ordering::Relaxed) {
                // Periodic health checks
                let now = Instant::now();
                if now.duration_since(last_health_check) >= HEALTH_CHECK_INTERVAL {
                    self.check_all_workers_health();
                    last_health_check = now;
                }

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
    fn accept_connection(&mut self, mut stream: UnixStream) -> Result<()> {
        // Read the session init message first
        let (msg, msg_id) = match read_message(&mut stream) {
            Ok(result) => result,
            Err(e) => {
                // Client disconnected or error reading
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Ok(()); // Normal disconnection
                }
                return Err(anyhow!("Failed to read message: {}", e));
            }
        };

        let session_init = match msg {
            Message::SessionInit(init) => init,
            _ => {
                Self::send_error(&stream, "Expected SessionInit message", msg_id)?;
                return Ok(());
            }
        };

        // Dispatch based on mode
        if let Some(ref mut pool) = self.worker_pool {
            // Worker pool mode: dispatch to pool
            pool.dispatch_request(session_init, msg_id, stream)?;
            Ok(())
        } else {
            // Fork-per-request mode: fork a worker
            self.accept_connection_fork(session_init, msg_id, stream)
        }
    }

    /// Accept connection using fork-per-request (legacy mode)
    fn accept_connection_fork(&mut self, session_init: SessionInit, msg_id: u32, stream: UnixStream) -> Result<()> {
        // Fork a worker process
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child }) => {
                // Parent: register session
                let handle = SessionHandle {
                    id: child.as_raw(),
                    worker_pid: child,
                    created_at: Instant::now(),
                    health_state: WorkerHealthState::Healthy,
                    metrics: WorkerMetrics::default(),
                    last_ping_attempt: None,
                };

                self.sessions.insert(child.as_raw(), handle);

                // Close stream in parent (worker owns it)
                drop(stream);

                Ok(())
            }
            Ok(ForkResult::Child) => {
                // Child: become session worker
                // Execute the session init we already received
                Self::run_worker_with_session(stream, session_init, msg_id);
            }
            Err(e) => {
                Err(anyhow!("Failed to fork worker: {}", e))
            }
        }
    }

    /// Run the session worker with a pre-read session init (in child process)
    fn run_worker_with_session(mut stream: UnixStream, session_init: SessionInit, msg_id: u32) -> ! {
        let exit_code = match Self::execute_session(&session_init) {
            Ok(exec_result) => {
                // Send result back to client
                let result = ExecutionResult {
                    exit_code: exec_result.exit_code,
                    stdout_len: exec_result.stdout().len() as u64,
                    stderr_len: exec_result.stderr.len() as u64,
                    stdout: exec_result.stdout(),
                    stderr: exec_result.stderr,
                };

                if let Err(e) = write_message(&mut stream, &Message::ExecutionResult(result), msg_id) {
                    eprintln!("Failed to send result: {}", e);
                    1
                } else {
                    // Ensure data is flushed before worker exits
                    let _ = stream.flush();
                    let _ = stream.shutdown(std::net::Shutdown::Write);
                    0
                }
            }
            Err(e) => {
                eprintln!("Worker error: {}", e);
                1
            }
        };

        std::process::exit(exit_code);
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
        // Capture initial worker state for reset between requests
        // (Currently workers are single-use, but this enables future worker pooling)
        let worker_state = WorkerState::capture()?;

        // Read session init message
        // Note: We don't set timeouts because they fail after fork with EINVAL
        // Status check connections will just disconnect immediately with UnexpectedEof
        let (msg, msg_id) = match read_message(&mut stream) {
            Ok(result) => result,
            Err(e) => {
                // Client disconnected without sending a message (e.g., status check)
                // This is normal, just exit quietly
                if e.kind() == std::io::ErrorKind::UnexpectedEof
                    || e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::ConnectionReset {
                    return Ok(0);
                }
                return Err(anyhow!("Failed to read message: {}", e));
            }
        };

        let session_init = match msg {
            Message::SessionInit(init) => init,
            _ => {
                Self::send_error(&stream, "Expected SessionInit message", msg_id)?;
                return Ok(1);
            }
        };

        // Execute the session
        let exec_result = Self::execute_session(&session_init)?;

        // Send result back to client
        let result = ExecutionResult {
            exit_code: exec_result.exit_code,
            stdout_len: exec_result.stdout().len() as u64,
            stderr_len: exec_result.stderr.len() as u64,
            stdout: exec_result.stdout(),
            stderr: exec_result.stderr,
        };

        write_message(&mut stream, &Message::ExecutionResult(result), msg_id)
            .map_err(|e| anyhow!("Failed to send result: {}", e))?;

        // Ensure data is flushed before worker exits
        stream.flush()?;

        // Shutdown write side to signal we're done
        let _ = stream.shutdown(std::net::Shutdown::Write);

        // Reset worker state for potential reuse (future worker pooling)
        // Currently workers exit after one request, but this ensures clean state
        if let Err(e) = worker_state.reset() {
            eprintln!("Warning: Failed to reset worker state: {}", e);
            // Non-fatal for single-use workers, but important for pooling
        }

        Ok(0)
    }

    /// Reset worker state between requests (for worker pooling)
    /// 
    /// This ensures complete isolation between requests by resetting:
    /// - Working directory to original value
    /// - Environment variables to original state
    /// - Exit code (implicit in new executor creation)
    /// - Executor state (created fresh each time)
    /// 
    /// Critical for preventing state leakage when workers handle multiple requests.
    fn reset_worker_state(worker_state: &WorkerState) -> Result<()> {
        worker_state.reset()
    }

    /// Execute a session command and return the full execution result
    fn execute_session(init: &SessionInit) -> Result<crate::executor::ExecutionResult> {
        // Set working directory
        std::env::set_current_dir(&init.working_dir)
            .map_err(|e| anyhow!("Failed to set working directory to '{}': {}", init.working_dir, e))?;

        // Set environment variables
        for (key, value) in &init.env {
            std::env::set_var(key, value);
        }

        // Parse and execute command
        // For now, just execute the args as a command via -c flag
        if init.args.len() >= 2 && init.args[0] == "-c" {
            let command = &init.args[1];

            // Create executor in embedded mode (no progress, all IO piped)
            // This prevents "Invalid argument" errors from inherited file descriptors
            // NOTE: Executor is created fresh for each request, ensuring no state leakage
            let mut executor = crate::executor::Executor::new_embedded();

            // Parse
            let tokens = match crate::lexer::Lexer::tokenize(command) {
                Ok(tokens) => tokens,
                Err(e) => {
                    let err_msg = format!("Lexer error: {}", e);
                    return Ok(crate::executor::ExecutionResult {
                        output: crate::executor::Output::Text(String::new()),
                        stderr: err_msg,
                        exit_code: 2,
        error: None,
                    });
                }
            };
            let mut parser = crate::parser::Parser::new(tokens);

            match parser.parse() {
                Ok(ast) => {
                    match executor.execute(ast) {
                        Ok(result) => Ok(result),
                        Err(e) => {
                            let err_msg = format!("Execution error: {}", e);
                            Ok(crate::executor::ExecutionResult {
                                output: crate::executor::Output::Text(String::new()),
                                stderr: err_msg,
                                exit_code: 1,
        error: None,
                            })
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("Parse error: {}", e);
                    Ok(crate::executor::ExecutionResult {
                        output: crate::executor::Output::Text(String::new()),
                        stderr: err_msg,
                        exit_code: 2,
        error: None,
                    })
                }
            }
        } else {
            Ok(crate::executor::ExecutionResult {
                output: crate::executor::Output::Text(String::new()),
                stderr: String::new(),
                exit_code: 0,
                error: None,
            })
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
            stdout: String::new(),
            stderr: message.to_string(),
        };
        write_message(&mut stream, &Message::ExecutionResult(error_result), msg_id)
            .map_err(|e| anyhow!("Failed to send error: {}", e))
    }

    /// Reap finished worker processes
    fn reap_workers(&mut self) {
        let mut finished = Vec::new();

        for (session_id, handle) in &mut self.sessions {
            match waitpid(handle.worker_pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, code)) => {
                    if code != 0 {
                        eprintln!("Worker {} exited with non-zero code: {}", session_id, code);
                        handle.metrics.requests_failed += 1;
                    }
                    handle.health_state = WorkerHealthState::Crashed;
                    finished.push(pid.as_raw());
                }
                Ok(WaitStatus::Signaled(pid, signal, _)) => {
                    eprintln!("Worker {} terminated by signal: {:?}", session_id, signal);
                    handle.health_state = WorkerHealthState::Crashed;
                    handle.metrics.requests_failed += 1;
                    finished.push(pid.as_raw());
                }
                Ok(WaitStatus::StillAlive) => {
                    // Still running, update heartbeat if recently active
                    // (In a full implementation, worker would send heartbeats)
                }
                Err(e) => {
                    eprintln!("Error waiting for process {}: {}", session_id, e);
                    handle.health_state = WorkerHealthState::Crashed;
                    finished.push(*session_id);
                }
                _ => {
                    // Other statuses, keep monitoring
                }
            }
        }

        // Remove finished sessions
        for session_id in finished {
            if let Some(handle) = self.sessions.remove(&session_id) {
                eprintln!("Cleaned up worker {} after {} requests ({} failed)", 
                    session_id, 
                    handle.metrics.requests_processed,
                    handle.metrics.requests_failed);
            }
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
        // Shutdown worker pool if enabled
        if let Some(ref mut pool) = self.worker_pool {
            eprintln!("Shutting down worker pool...");
            pool.shutdown()?;
        }

        // Send SIGTERM to all worker processes (fork-per-request mode)
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

    /// Check health of all workers
    fn check_all_workers_health(&mut self) {
        let now = Instant::now();
        let mut workers_to_check = Vec::new();

        // Collect workers that need health checking
        for (session_id, handle) in &self.sessions {
            let needs_check = match handle.last_ping_attempt {
                None => true, // Never checked
                Some(last_ping) => now.duration_since(last_ping) >= HEALTH_CHECK_INTERVAL,
            };

            if needs_check {
                workers_to_check.push(*session_id);
            }
        }

        // Check each worker's health
        for session_id in workers_to_check {
            if let Err(e) = self.check_worker_health(session_id) {
                eprintln!("Health check failed for worker {}: {}", session_id, e);
            }
        }
    }

    /// Check the health of a specific worker
    fn check_worker_health(&mut self, session_id: SessionId) -> Result<()> {
        let handle = self.sessions.get_mut(&session_id)
            .ok_or_else(|| anyhow!("Session {} not found", session_id))?;

        let now = Instant::now();
        handle.last_ping_attempt = Some(now);

        // First check: Is the process still alive?
        match waitpid(handle.worker_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(_, code)) => {
                eprintln!("Worker {} exited with code {}", session_id, code);
                handle.health_state = WorkerHealthState::Crashed;
                handle.metrics.requests_failed += 1;
                
                // Try to respawn if under limit
                if handle.metrics.respawn_count < MAX_RESPAWN_ATTEMPTS {
                    let time_since_spawn = now.duration_since(handle.metrics.spawn_time);
                    if time_since_spawn >= RESPAWN_COOLDOWN {
                        eprintln!("Respawning worker {} (attempt {})", 
                            session_id, handle.metrics.respawn_count + 1);
                        return self.respawn_worker(session_id);
                    } else {
                        eprintln!("Worker {} crashed too soon, waiting for cooldown", session_id);
                    }
                }
                return Ok(());
            }
            Ok(WaitStatus::Signaled(_, sig, _)) => {
                eprintln!("Worker {} killed by signal {:?}", session_id, sig);
                handle.health_state = WorkerHealthState::Crashed;
                handle.metrics.requests_failed += 1;
                
                // Try to respawn
                if handle.metrics.respawn_count < MAX_RESPAWN_ATTEMPTS {
                    let time_since_spawn = now.duration_since(handle.metrics.spawn_time);
                    if time_since_spawn >= RESPAWN_COOLDOWN {
                        eprintln!("Respawning worker {} after signal", session_id);
                        return self.respawn_worker(session_id);
                    }
                }
                return Ok(());
            }
            Ok(WaitStatus::StillAlive) => {
                // Process is alive, continue with health checks
            }
            _ => {
                // Other states, assume still alive
            }
        }

        // Check for hung workers (last heartbeat too old)
        let time_since_heartbeat = now.duration_since(handle.metrics.last_heartbeat);
        
        if time_since_heartbeat > REQUEST_TIMEOUT {
            // Worker hasn't responded in a long time
            if time_since_heartbeat > REQUEST_TIMEOUT * 2 {
                // Definitely hung, kill and respawn
                eprintln!("Worker {} is hung (no heartbeat for {:?}), killing", 
                    session_id, time_since_heartbeat);
                handle.health_state = WorkerHealthState::Hung;
                
                // Force kill
                let _ = signal::kill(handle.worker_pid, Signal::SIGKILL);
                
                // Try to respawn
                if handle.metrics.respawn_count < MAX_RESPAWN_ATTEMPTS {
                    return self.respawn_worker(session_id);
                }
            } else {
                // Mark as slow/unresponsive
                handle.health_state = WorkerHealthState::Unresponsive;
                handle.metrics.consecutive_failures += 1;
                eprintln!("Worker {} is unresponsive (no heartbeat for {:?})", 
                    session_id, time_since_heartbeat);
            }
        } else {
            // Worker responded recently, reset consecutive failures
            if handle.health_state != WorkerHealthState::Healthy {
                eprintln!("Worker {} recovered", session_id);
                handle.health_state = WorkerHealthState::Healthy;
                handle.metrics.consecutive_failures = 0;
            }
        }

        Ok(())
    }

    /// Respawn a failed worker
    fn respawn_worker(&mut self, session_id: SessionId) -> Result<()> {
        // Remove the old worker from sessions
        let old_handle = self.sessions.remove(&session_id)
            .ok_or_else(|| anyhow!("Session {} not found", session_id))?;

        eprintln!("Respawning worker {} (previous PID: {})", 
            session_id, old_handle.worker_pid);

        // Note: We can't truly "respawn" a worker for an existing session since
        // workers are tied to client connections. This method is here for potential
        // future use with persistent workers or worker pools.
        
        // For now, we just ensure cleanup happened
        let _ = waitpid(old_handle.worker_pid, Some(WaitPidFlag::WNOHANG));

        eprintln!("Worker {} removed from pool after {} respawn attempts", 
            session_id, old_handle.metrics.respawn_count);

        Ok(())
    }

    /// Print worker health statistics
    pub fn print_health_stats(&self) {
        println!("\nWorker Health Statistics:");
        println!("Total active workers: {}", self.sessions.len());
        
        let mut healthy = 0;
        let mut unresponsive = 0;
        let mut slow = 0;
        let mut crashed = 0;
        let mut hung = 0;

        for handle in self.sessions.values() {
            match handle.health_state {
                WorkerHealthState::Healthy => healthy += 1,
                WorkerHealthState::Unresponsive => unresponsive += 1,
                WorkerHealthState::Slow => slow += 1,
                WorkerHealthState::Crashed => crashed += 1,
                WorkerHealthState::Hung => hung += 1,
            }
        }

        println!("  Healthy: {}", healthy);
        println!("  Unresponsive: {}", unresponsive);
        println!("  Slow: {}", slow);
        println!("  Crashed: {}", crashed);
        println!("  Hung: {}", hung);

        // Print detailed metrics
        for handle in self.sessions.values() {
            let uptime = handle.created_at.elapsed().as_secs();
            let failure_rate = if handle.metrics.requests_processed > 0 {
                (handle.metrics.requests_failed as f64 / handle.metrics.requests_processed as f64) * 100.0
            } else {
                0.0
            };

            println!("\nWorker {} (PID {}):", handle.id, handle.worker_pid);
            println!("  State: {:?}", handle.health_state);
            println!("  Uptime: {}s", uptime);
            println!("  Requests: {} processed, {} failed ({:.1}% failure rate)",
                handle.metrics.requests_processed,
                handle.metrics.requests_failed,
                failure_rate);
            println!("  Respawn count: {}", handle.metrics.respawn_count);
            println!("  Consecutive failures: {}", handle.metrics.consecutive_failures);
        }
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
