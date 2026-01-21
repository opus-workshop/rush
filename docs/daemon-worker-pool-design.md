# Rush Daemon Worker Pool Architecture Design

## Executive Summary

This design eliminates the 2-3ms fork overhead in the Rush daemon by replacing the current fork-per-request model with a persistent worker pool. Workers are pre-forked at daemon startup and reused across multiple client requests via bi-directional Unix domain sockets.

**Key Performance Goal:** Reduce per-request latency from ~3ms to <0.5ms by eliminating fork() syscall overhead.

## Current Architecture Analysis

### Fork-Based Model (server.rs)

The current implementation:
1. Daemon listens on Unix socket (`~/.rush/daemon.sock`)
2. On each client connection: `accept_connection()` → `fork()` → worker exits
3. Worker process:
   - Receives `SessionInit` message with env, cwd, args
   - Parses and executes command via `Executor::new_embedded()`
   - Sends `ExecutionResult` back
   - Exits immediately (process lifetime ~3ms)
4. Parent reaps workers via `waitpid(WNOHANG)` in accept loop

### Performance Bottleneck

```rust
// Current: server.rs:182-206
match unsafe { fork() } {
    Ok(ForkResult::Parent { child }) => {
        // 2-3ms overhead here
        self.sessions.insert(child.as_raw(), handle);
        drop(stream);
    }
    Ok(ForkResult::Child) => {
        Self::run_worker(stream);  // Never returns
    }
}
```

The fork overhead includes:
- Process creation (~1ms)
- Copy-on-write page table setup (~0.5ms)
- Initial exec/schedule latency (~0.5-1ms)
- Binary state initialization (~0.5ms)

## Worker Pool Architecture

### Overview

Replace fork-per-request with a pool of persistent workers that communicate with the main daemon via bi-directional Unix socket pairs created at startup.

```
┌──────────────────────────────────────────────────────────────┐
│  Main Daemon Process                                          │
│  ┌────────────────┐        ┌─────────────────────┐          │
│  │  Accept Loop   │        │   Worker Pool       │          │
│  │                │        │                     │          │
│  │  - Client      │───────▶│  - Dispatch Queue   │          │
│  │    connections │        │  - Worker Registry  │          │
│  │  - Health      │◀───────│  - Health Monitor   │          │
│  │    monitoring  │        │                     │          │
│  └────────────────┘        └─────────────────────┘          │
│                                     │                         │
│                     ┌───────────────┼───────────────┐        │
│                     ▼               ▼               ▼         │
│            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐  │
│            │  Worker 1   │ │  Worker 2   │ │  Worker N   │  │
│            │  (Idle)     │ │  (Busy)     │ │  (Idle)     │  │
│            └─────────────┘ └─────────────┘ └─────────────┘  │
│                 ▲                 ▲                 ▲         │
│                 │                 │                 │         │
└─────────────────┼─────────────────┼─────────────────┼─────────┘
                  │                 │                 │
              socketpair()      socketpair()      socketpair()
                  │                 │                 │
                  ▼                 ▼                 ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │  Client A   │ │  Client B   │ │  Client C   │
            │ (waiting)   │ │ (executing) │ │ (waiting)   │
            └─────────────┘ └─────────────┘ └─────────────┘
```

### Core Data Structures

#### WorkerPool

```rust
pub struct WorkerPool {
    /// Pool configuration
    config: PoolConfig,

    /// All workers (idle + busy)
    workers: Vec<Worker>,

    /// Queue of idle worker IDs ready for dispatch
    idle_queue: VecDeque<WorkerId>,

    /// Map of busy workers to their client stream
    busy_workers: HashMap<WorkerId, ClientConnection>,

    /// Shutdown signal
    shutdown: Arc<AtomicBool>,

    /// Worker health monitor thread handle
    health_monitor: Option<JoinHandle<()>>,

    /// Metrics
    metrics: PoolMetrics,
}

pub struct PoolConfig {
    /// Minimum workers to maintain
    min_workers: usize,           // Default: 4

    /// Maximum workers allowed
    max_workers: usize,           // Default: 8

    /// Worker idle timeout (retire if idle too long)
    idle_timeout: Duration,       // Default: 60s

    /// Health check interval
    health_check_interval: Duration,  // Default: 5s

    /// Max requests per worker before retirement
    max_requests_per_worker: u64,     // Default: 1000 (prevent memory leaks)
}
```

#### Worker

```rust
pub struct Worker {
    /// Unique worker ID
    id: WorkerId,

    /// Worker process PID
    pid: Pid,

    /// Daemon-side socket for communication with worker
    daemon_socket: UnixStream,

    /// Worker state
    state: WorkerState,

    /// Statistics
    stats: WorkerStats,
}

pub type WorkerId = u32;

pub enum WorkerState {
    /// Worker is idle and available
    Idle {
        since: Instant,
    },

    /// Worker is executing a request
    Busy {
        since: Instant,
        client_addr: String,
    },

    /// Worker has been marked for retirement
    Retiring,

    /// Worker has crashed or become unresponsive
    Dead,
}

pub struct WorkerStats {
    /// When worker was spawned
    created_at: Instant,

    /// Total requests processed
    requests_handled: u64,

    /// Last request completed at
    last_request_at: Option<Instant>,

    /// Total CPU time (if available)
    cpu_time_ms: u64,
}
```

#### ClientConnection

```rust
pub struct ClientConnection {
    /// Client's Unix socket stream
    stream: UnixStream,

    /// Message ID for request/response correlation
    message_id: MessageId,

    /// When request was dispatched to worker
    dispatched_at: Instant,

    /// Timeout for this request
    timeout: Duration,
}
```

### Worker Pool Lifecycle

#### 1. Initialization (Daemon Startup)

```rust
impl WorkerPool {
    pub fn new(config: PoolConfig) -> Result<Self> {
        let mut pool = Self {
            config,
            workers: Vec::new(),
            idle_queue: VecDeque::new(),
            busy_workers: HashMap::new(),
            shutdown: Arc::new(AtomicBool::new(false)),
            health_monitor: None,
            metrics: PoolMetrics::default(),
        };

        // Pre-fork minimum workers
        for _ in 0..pool.config.min_workers {
            pool.spawn_worker()?;
        }

        // Start health monitoring thread
        pool.start_health_monitor();

        Ok(pool)
    }
}
```

#### 2. Worker Spawning

```rust
fn spawn_worker(&mut self) -> Result<WorkerId> {
    // Create Unix socket pair for daemon ↔ worker communication
    let (daemon_socket, worker_socket) = UnixStream::pair()?;

    // Configure sockets (non-blocking on daemon side)
    daemon_socket.set_nonblocking(true)?;
    worker_socket.set_nonblocking(false)?;

    // Fork worker process
    match unsafe { fork()? } {
        ForkResult::Parent { child } => {
            // Parent: close worker's socket end
            drop(worker_socket);

            let worker_id = self.next_worker_id();
            let worker = Worker {
                id: worker_id,
                pid: child,
                daemon_socket,
                state: WorkerState::Idle { since: Instant::now() },
                stats: WorkerStats::new(),
            };

            self.workers.push(worker);
            self.idle_queue.push_back(worker_id);

            Ok(worker_id)
        }
        ForkResult::Child => {
            // Child: become worker process
            drop(daemon_socket);
            Self::run_persistent_worker(worker_socket);
            // Never returns
        }
    }
}
```

#### 3. Persistent Worker Event Loop

The key difference: workers don't exit after one request.

```rust
fn run_persistent_worker(daemon_socket: UnixStream) -> ! {
    let mut executor = Executor::new_embedded();
    let mut request_count = 0u64;

    loop {
        // Read work assignment from daemon
        let (message, msg_id) = match read_message(&mut daemon_socket) {
            Ok(msg) => msg,
            Err(_) => {
                // Daemon closed connection → shutdown
                std::process::exit(0);
            }
        };

        match message {
            Message::SessionInit(session_init) => {
                // Execute request (same as current implementation)
                let result = Self::execute_session(&mut executor, &session_init);

                // Send result back to daemon
                let _ = write_message(
                    &mut daemon_socket,
                    &Message::ExecutionResult(result),
                    msg_id
                );

                request_count += 1;
            }
            Message::Shutdown(_) => {
                // Graceful shutdown
                std::process::exit(0);
            }
            _ => {
                // Invalid message type
                eprintln!("Worker: unexpected message type");
            }
        }
    }
}
```

### Request Dispatch Algorithm

#### Fast Path: Idle Worker Available

```rust
pub fn dispatch_request(
    &mut self,
    client_stream: UnixStream,
    session_init: SessionInit,
    message_id: MessageId,
) -> Result<()> {
    // 1. Try to get idle worker (O(1) pop from queue)
    let worker_id = match self.idle_queue.pop_front() {
        Some(id) => id,
        None => {
            // No idle workers available
            if self.workers.len() < self.config.max_workers {
                // Spawn new worker dynamically
                self.spawn_worker()?
            } else {
                // Pool saturated → return error to client
                return Self::send_pool_saturated_error(&client_stream, message_id);
            }
        }
    };

    // 2. Get worker and forward request
    let worker = self.get_worker_mut(worker_id)?;

    // 3. Send SessionInit to worker
    write_message(
        &mut worker.daemon_socket,
        &Message::SessionInit(session_init),
        message_id,
    )?;

    // 4. Mark worker as busy
    worker.state = WorkerState::Busy {
        since: Instant::now(),
        client_addr: format!("{:?}", client_stream.peer_addr()),
    };

    // 5. Store client connection for response forwarding
    self.busy_workers.insert(worker_id, ClientConnection {
        stream: client_stream,
        message_id,
        dispatched_at: Instant::now(),
        timeout: Duration::from_secs(30),
    });

    self.metrics.requests_dispatched += 1;

    Ok(())
}
```

#### Response Collection (Non-Blocking Event Loop)

The daemon must multiplex responses from multiple busy workers:

```rust
pub fn collect_responses(&mut self) -> Result<()> {
    // Use poll/epoll to wait on all busy worker sockets
    let mut poll = Poll::new()?;

    for (worker_id, _) in &self.busy_workers {
        let worker = self.get_worker(*worker_id)?;
        poll.registry().register(
            &mut worker.daemon_socket,
            Token(*worker_id as usize),
            Interest::READABLE,
        )?;
    }

    let mut events = Events::with_capacity(self.busy_workers.len());
    poll.poll(&mut events, Some(Duration::from_millis(10)))?;

    for event in events.iter() {
        let worker_id = event.token().0 as WorkerId;
        self.handle_worker_response(worker_id)?;
    }

    Ok(())
}

fn handle_worker_response(&mut self, worker_id: WorkerId) -> Result<()> {
    let worker = self.get_worker_mut(worker_id)?;

    // Read response from worker
    let (message, msg_id) = read_message(&mut worker.daemon_socket)?;

    // Get client connection
    let client_conn = self.busy_workers.remove(&worker_id)
        .ok_or_else(|| anyhow!("Worker {} not in busy set", worker_id))?;

    // Forward response to client
    match message {
        Message::ExecutionResult(result) => {
            write_message(&client_conn.stream, &message, msg_id)?;

            // Update worker stats
            worker.stats.requests_handled += 1;
            worker.stats.last_request_at = Some(Instant::now());

            // Return worker to idle pool
            worker.state = WorkerState::Idle { since: Instant::now() };
            self.idle_queue.push_back(worker_id);

            self.metrics.requests_completed += 1;
        }
        _ => {
            return Err(anyhow!("Unexpected message from worker"));
        }
    }

    Ok(())
}
```

### Concurrency Model

#### Single-Threaded with Polling (Recommended)

The main daemon runs a single event loop using `mio::Poll` to multiplex:
- Client connections (accept loop)
- Worker responses (response collection)
- Health monitoring (periodic checks)

**Advantages:**
- No synchronization overhead (no mutexes)
- Simple reasoning about state
- Efficient CPU usage
- Matches current single-threaded daemon design

**Structure:**
```rust
impl DaemonServer {
    fn event_loop(&mut self) -> Result<()> {
        let poll = Poll::new()?;

        // Register client listener
        poll.registry().register(
            &mut self.listener,
            CLIENT_TOKEN,
            Interest::READABLE,
        )?;

        while !self.shutdown.load(Ordering::Relaxed) {
            // 1. Accept new clients
            if let Ok((stream, _)) = self.listener.accept() {
                self.handle_new_client(stream)?;
            }

            // 2. Collect worker responses (forward to clients)
            self.worker_pool.collect_responses()?;

            // 3. Periodic: health checks, worker retirement
            if self.should_run_health_check() {
                self.worker_pool.health_check()?;
            }
        }

        Ok(())
    }
}
```

#### Alternative: Multi-Threaded (If Needed)

If profiling shows the single-threaded model can't keep up:

```rust
struct WorkerPool {
    // Shared state protected by RwLock
    workers: Arc<RwLock<Vec<Worker>>>,
    idle_queue: Arc<Mutex<VecDeque<WorkerId>>>,
    busy_workers: Arc<Mutex<HashMap<WorkerId, ClientConnection>>>,

    // Dispatcher thread pool (1-2 threads)
    dispatcher_threads: Vec<JoinHandle<()>>,
}
```

But this adds complexity without proven benefit. Start simple.

### Worker Health Monitoring

Background thread that runs every 5 seconds:

```rust
fn health_monitor_loop(pool: Arc<Mutex<WorkerPool>>) {
    loop {
        thread::sleep(Duration::from_secs(5));

        let mut pool = pool.lock().unwrap();
        let now = Instant::now();

        // 1. Check for hung workers (busy > 30s)
        for (worker_id, client_conn) in &pool.busy_workers {
            if now.duration_since(client_conn.dispatched_at) > client_conn.timeout {
                eprintln!("Worker {} hung, killing", worker_id);
                pool.kill_worker(*worker_id);
            }
        }

        // 2. Retire idle workers exceeding timeout
        pool.workers.retain(|worker| {
            if let WorkerState::Idle { since } = worker.state {
                if now.duration_since(since) > pool.config.idle_timeout {
                    // Too many workers, retire this one
                    if pool.workers.len() > pool.config.min_workers {
                        pool.retire_worker(worker.id);
                        return false;
                    }
                }
            }
            true
        });

        // 3. Retire workers that have handled too many requests
        for worker in &pool.workers {
            if worker.stats.requests_handled >= pool.config.max_requests_per_worker {
                pool.retire_worker(worker.id);
            }
        }

        // 4. Check worker liveness (send ping, expect pong)
        for worker in &pool.workers {
            if !pool.is_worker_alive(worker.id) {
                eprintln!("Worker {} dead, respawning", worker.id);
                pool.kill_worker(worker.id);
            }
        }

        // 5. Ensure minimum workers
        while pool.idle_queue.len() + pool.busy_workers.len() < pool.config.min_workers {
            let _ = pool.spawn_worker();
        }
    }
}
```

### Worker Retirement and Crash Recovery

#### Graceful Retirement

```rust
fn retire_worker(&mut self, worker_id: WorkerId) {
    let worker = match self.get_worker_mut(worker_id) {
        Ok(w) => w,
        Err(_) => return,
    };

    match worker.state {
        WorkerState::Idle { .. } => {
            // Send shutdown message
            let _ = write_message(
                &mut worker.daemon_socket,
                &Message::Shutdown(Shutdown { force: false }),
                0,
            );

            worker.state = WorkerState::Retiring;

            // Wait for graceful exit (non-blocking)
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(2));
                // Force kill if still alive
                let _ = signal::kill(worker.pid, Signal::SIGKILL);
            });
        }
        WorkerState::Busy { .. } => {
            // Mark for retirement after current request
            worker.state = WorkerState::Retiring;
        }
        _ => {}
    }
}
```

#### Crash Recovery

```rust
fn handle_worker_crash(&mut self, worker_id: WorkerId) {
    eprintln!("Worker {} crashed", worker_id);

    // 1. Remove from all tracking structures
    self.idle_queue.retain(|&id| id != worker_id);

    // 2. If worker was busy, return error to client
    if let Some(client_conn) = self.busy_workers.remove(&worker_id) {
        let error = ExecutionResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "Worker crashed during execution".to_string(),
            stdout_len: 0,
            stderr_len: 37,
        };
        let _ = write_message(
            &client_conn.stream,
            &Message::ExecutionResult(error),
            client_conn.message_id,
        );

        self.metrics.worker_crashes += 1;
    }

    // 3. Remove worker from pool
    self.workers.retain(|w| w.id != worker_id);

    // 4. Spawn replacement worker
    let _ = self.spawn_worker();
}
```

### Integration Points

#### Modified DaemonServer Structure

```rust
pub struct DaemonServer {
    socket_path: PathBuf,
    listener: Option<UnixListener>,
    worker_pool: WorkerPool,  // Replace sessions HashMap
    shutdown: Arc<AtomicBool>,
}
```

#### Updated Accept Loop

```rust
fn accept_loop(&mut self) -> Result<()> {
    let listener = self.listener.take().unwrap();
    listener.set_nonblocking(true)?;

    while !self.shutdown.load(Ordering::Relaxed) {
        // Accept client connection
        match listener.accept() {
            Ok((stream, _)) => {
                // Read SessionInit message
                let (msg, msg_id) = read_message(&stream)?;

                match msg {
                    Message::SessionInit(session_init) => {
                        // Dispatch to worker pool
                        self.worker_pool.dispatch_request(
                            stream,
                            session_init,
                            msg_id,
                        )?;
                    }
                    _ => {
                        // Invalid message
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No client connection, collect worker responses
                self.worker_pool.collect_responses()?;
                thread::sleep(Duration::from_millis(1));
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
            }
        }
    }

    Ok(())
}
```

### Error Handling Strategy

#### 1. Worker Unresponsive

**Detection:** Health monitor detects worker hasn't responded in 30+ seconds.

**Action:**
1. Send SIGTERM to worker process
2. Wait 2 seconds
3. Send SIGKILL if still alive
4. Return error to waiting client
5. Spawn replacement worker

#### 2. Worker Crashes During Request

**Detection:** `read_message()` returns `UnexpectedEof` or worker socket becomes readable with 0 bytes.

**Action:**
1. Detect via `waitpid(WNOHANG)` returning `Exited` status
2. Send error response to client
3. Remove worker from pool
4. Spawn replacement worker
5. Log crash for debugging

#### 3. Pool Saturation

**Detection:** All workers busy and `workers.len() == max_workers`.

**Action:**
1. Send `ExecutionResult` with error:
   ```rust
   ExecutionResult {
       exit_code: 1,
       stderr: "Daemon pool saturated, try again",
       ...
   }
   ```
2. Increment `metrics.pool_saturated_count`
3. Client can retry after backoff

#### 4. Socket Communication Errors

**Detection:** `write_message()` or `read_message()` fails.

**Action:**
- If daemon → worker: Mark worker as dead, kill and replace
- If daemon → client: Log error, close client connection
- If worker → daemon: Worker will detect closed socket and exit

#### 5. Fork Failure

**Detection:** `fork()` returns `Err`.

**Action:**
1. Log error (likely EAGAIN - process limit)
2. Return pool saturation error to client
3. Don't retry automatically (system-level issue)

### Metrics and Observability

```rust
pub struct PoolMetrics {
    /// Total requests dispatched to workers
    pub requests_dispatched: u64,

    /// Total requests completed successfully
    pub requests_completed: u64,

    /// Number of times pool was saturated
    pub pool_saturated_count: u64,

    /// Number of worker crashes
    pub worker_crashes: u64,

    /// Number of workers spawned (total)
    pub workers_spawned: u64,

    /// Number of workers retired
    pub workers_retired: u64,

    /// Average request latency (EWMA)
    pub avg_request_latency_ms: f64,
}

impl WorkerPool {
    pub fn get_metrics(&self) -> &PoolMetrics {
        &self.metrics
    }

    pub fn print_status(&self) {
        println!("Worker Pool Status:");
        println!("  Idle workers: {}", self.idle_queue.len());
        println!("  Busy workers: {}", self.busy_workers.len());
        println!("  Total workers: {}", self.workers.len());
        println!("  Requests handled: {}", self.metrics.requests_completed);
        println!("  Pool saturations: {}", self.metrics.pool_saturated_count);
        println!("  Worker crashes: {}", self.metrics.worker_crashes);
    }
}
```

## Communication Mechanism: Unix Sockets vs Channels

### Selected: Unix Domain Sockets (socketpair)

**Rationale:**
1. **Cross-process:** Channels don't work across fork boundaries
2. **Existing protocol:** Reuse length-prefixed JSON messages
3. **Non-blocking I/O:** Can use `mio::Poll` for efficient multiplexing
4. **Proven:** Current client ↔ daemon already uses Unix sockets

**Implementation:**
```rust
let (daemon_socket, worker_socket) = UnixStream::pair()?;
```

Each worker gets a dedicated socket pair:
- **Daemon side:** Non-blocking, registered with epoll for responses
- **Worker side:** Blocking, simple read-execute-write loop

### Alternative Considered: Shared Memory + Atomic Flags

Not chosen because:
- More complex synchronization
- No benefit over sockets (sockets are already zero-copy within kernel)
- Harder to debug

## Rollout Plan

### Phase 1: Basic Pool (Minimum Viable)
- Implement `WorkerPool` with fixed-size pool (4 workers)
- Spawn workers at daemon startup
- Single-threaded dispatch and response collection
- No health monitoring yet

### Phase 2: Dynamic Scaling
- Add idle worker queue
- Dynamic worker spawning on demand
- Worker retirement after idle timeout
- Health monitor thread

### Phase 3: Robustness
- Crash detection and recovery
- Hung worker detection and timeouts
- Pool saturation handling
- Comprehensive metrics

### Phase 4: Optimization
- Benchmark and tune pool size
- Consider SOCK_SEQPACKET for message boundaries
- Profile and optimize hot paths
- Add request queuing if needed

## Performance Expectations

### Current Performance
- Fork overhead: ~2-3ms per request
- 300-400 requests/second max throughput

### Expected Performance (Worker Pool)
- Dispatch overhead: ~0.2-0.5ms (socket write + epoll)
- 2000-5000 requests/second throughput
- **6-10x improvement** in latency
- **5-12x improvement** in throughput

### Benchmark Plan
```bash
# Before (fork-based)
$ hyperfine --warmup 5 'rush -c "echo test"'
Time (mean ± σ):       3.2 ms ±   0.4 ms

# After (worker pool)
$ hyperfine --warmup 5 'rush -c "echo test"'
Time (mean ± σ):       0.5 ms ±   0.1 ms   # Target
```

## Open Questions

1. **Pool size tuning:** How to auto-tune `min_workers` and `max_workers` based on load?
   - Start with fixed 4-8 range
   - Consider adaptive scaling in Phase 4

2. **Request queuing:** Should we queue requests when pool is saturated?
   - No for Phase 1 (fail fast)
   - Maybe in Phase 4 if needed

3. **Worker memory leaks:** How to detect and handle memory leaks in workers?
   - Retire workers after 1000 requests (`max_requests_per_worker`)
   - Monitor RSS in health checks (future optimization)

4. **Session affinity:** Do clients need to be routed to the same worker?
   - No - workers are stateless
   - Each request is independent

## Conclusion

This worker pool design eliminates the fork-per-request bottleneck while maintaining:
- **Isolation:** Each worker is a separate process (same as current)
- **Simplicity:** Single-threaded event loop (no complex locking)
- **Robustness:** Crash recovery and health monitoring
- **Performance:** 6-10x latency improvement, 5-12x throughput improvement

The design reuses existing protocol infrastructure and integrates cleanly with the current codebase architecture.
