/// Rush daemon implementation for sub-millisecond startup via persistent server
///
/// This module implements the daemon architecture specified in docs/daemon-architecture.md:
/// - `protocol`: Message framing and serialization (length-prefixed binary format)
/// - `server`: Unix socket server and accept loop (with fork-based session workers)
/// - `worker`: Fork-based session workers (per-client isolation)
/// - `client`: Thin client logic for daemon communication

pub mod protocol;
pub mod server;
pub mod worker;
pub mod worker_pool;
pub mod client;

pub use protocol::{
    Message, SessionInit, SessionInitAck, Execute, ExecutionResult, Signal, Shutdown,
    encode_message, decode_message, write_message, read_message,
};
pub use server::{DaemonServer, SessionHandle, SessionId};
pub use worker_pool::{WorkerPool, Worker, WorkerState, PoolConfig, PoolStats};
pub use client::DaemonClient;
