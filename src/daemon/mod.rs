/// Rush daemon implementation for sub-millisecond startup via persistent server
///
/// This module implements the daemon architecture specified in docs/daemon-architecture.md:
/// - `protocol`: Message framing and serialization (length-prefixed binary format)
/// - `server`: Unix socket server and accept loop (with fork-based session workers)
/// - `worker`: Fork-based session workers (per-client isolation)
/// - `client`: Thin client logic for daemon communication
/// - `config`: Configuration parsing from .rushrc (banner, custom stats)
/// - `pi_client`: Client for Pi agent IPC over Unix sockets
/// - `pi_rpc`: Pi RPC subprocess manager for fast `|?` execution

pub mod protocol;
pub mod server;
pub mod worker;
pub mod worker_pool;
pub mod client;
pub mod config;
pub mod pi_client;
pub mod pi_rpc;

pub use protocol::{
    Message, SessionInit, SessionInitAck, Execute, ExecutionResult, Signal, Shutdown,
    StatsRequest, StatsResponse,
    encode_message, decode_message, write_message, read_message,
    // Rush ↔ Pi IPC types
    RushToPi, PiToRush, ShellContext,
};
pub use server::{DaemonServer, SessionHandle, SessionId, StatsCache, CustomStatCached};
pub use worker_pool::{WorkerPool, Worker, WorkerState, PoolConfig, PoolStats};
pub use client::DaemonClient;
pub use config::{DaemonConfig, BannerConfig, CustomStatConfig, BannerStyle, BannerShow};
pub use pi_client::{PiClient, PiClientError};
pub use pi_rpc::{PiRpcManager, PiRpcError, PiCommand, PiEvent};
