use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Path to the daemon socket
fn socket_path() -> String {
    format!("{}/.rush/daemon.sock", std::env::var("HOME").unwrap_or_default())
}

/// Check if daemon is currently running
fn is_daemon_running() -> bool {
    let path = socket_path();
    std::path::Path::new(&path).exists() && UnixStream::connect(&path).is_ok()
}

/// Start the daemon, wait for it to accept connections
fn start_daemon() {
    // Stop any existing daemon first
    let _ = Command::new("target/release/rushd")
        .arg("stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    std::thread::sleep(Duration::from_millis(300));

    // Start daemon
    let _ = Command::new("target/release/rushd")
        .arg("start")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start rushd");

    // Wait for socket to appear and be connectable
    let path = socket_path();
    for _ in 0..50 {
        if std::path::Path::new(&path).exists() {
            if UnixStream::connect(&path).is_ok() {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("Daemon failed to start within 5 seconds");
}

/// Stop the daemon
fn stop_daemon() {
    let _ = Command::new("target/release/rushd")
        .arg("stop")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    std::thread::sleep(Duration::from_millis(300));
}

/// Execute a command via the daemon using DaemonClient
fn execute_via_daemon(cmd: &str) -> i32 {
    let mut client = rush::daemon::DaemonClient::new()
        .expect("Failed to create daemon client");
    let args = vec!["-c".to_string(), cmd.to_string()];
    // Note: execute_command calls process::exit on success in the real client,
    // but we need to get the result without exiting. We'll use the lower-level
    // protocol directly instead.
    execute_via_protocol(cmd)
}

/// Execute a command via raw protocol (avoids process::exit in DaemonClient)
fn execute_via_protocol(cmd: &str) -> i32 {
    use rush::daemon::protocol::{Message, SessionInit, write_message, read_message};

    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .expect("Failed to connect to daemon socket");

    let working_dir = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .to_string_lossy()
        .to_string();

    let mut env = HashMap::new();
    env.insert("PATH".to_string(), std::env::var("PATH").unwrap_or_default());

    let session_init = SessionInit {
        working_dir,
        env,
        args: vec!["-c".to_string(), cmd.to_string()],
        stdin_mode: "null".to_string(),
    };

    let msg = Message::SessionInit(session_init);
    write_message(&mut stream, &msg, 1).expect("Failed to write message");

    let (response, _msg_id) = read_message(&mut stream).expect("Failed to read response");

    match response {
        Message::ExecutionResult(result) => result.exit_code,
        _ => panic!("Unexpected response type"),
    }
}

/// Benchmark warm daemon execution (daemon already running, repeated commands)
fn bench_daemon_warm(c: &mut Criterion) {
    if !is_daemon_running() {
        start_daemon();
    }

    let mut group = c.benchmark_group("daemon_warm");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let commands = vec![
        ("exit", "exit"),
        ("echo_hello", "echo hello"),
        ("true_builtin", "true"),
    ];

    for (name, cmd) in &commands {
        group.bench_with_input(
            BenchmarkId::new("daemon", name),
            cmd,
            |b, cmd| {
                b.iter(|| {
                    let exit_code = execute_via_protocol(black_box(cmd));
                    black_box(exit_code);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark direct execution (rush -c, no daemon)
fn bench_direct_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("direct_execution");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let commands = vec![
        ("exit", "exit"),
        ("echo_hello", "echo hello"),
        ("true_builtin", "true"),
    ];

    for (name, cmd) in &commands {
        group.bench_with_input(
            BenchmarkId::new("direct", name),
            cmd,
            |b, cmd| {
                b.iter(|| {
                    let output = Command::new("target/release/rush")
                        .arg("-c")
                        .arg(cmd)
                        .output()
                        .expect("Failed to execute rush");
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark daemon cold start (stop, restart, execute first command)
fn bench_daemon_cold_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("daemon_cold_start");
    group.measurement_time(Duration::from_secs(30));
    group.sample_size(10); // Cold starts are expensive

    group.bench_function("first_command_after_restart", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for _ in 0..iters {
                // Stop daemon
                stop_daemon();

                // Start daemon
                start_daemon();

                // Time the first command
                let start = Instant::now();
                let exit_code = execute_via_protocol("exit");
                total += start.elapsed();
                black_box(exit_code);
            }
            total
        });
    });

    group.finish();
}

/// Benchmark daemon overhead = daemon_time - direct_time
/// Runs both back-to-back for each iteration to measure the delta
fn bench_daemon_overhead(c: &mut Criterion) {
    if !is_daemon_running() {
        start_daemon();
    }

    let mut group = c.benchmark_group("daemon_overhead");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    // Compare daemon vs direct for the same command
    group.bench_function("daemon_echo", |b| {
        b.iter(|| {
            let exit_code = execute_via_protocol(black_box("echo hello"));
            black_box(exit_code);
        });
    });

    group.bench_function("direct_echo", |b| {
        b.iter(|| {
            let output = Command::new("target/release/rush")
                .arg("-c")
                .arg("echo hello")
                .output()
                .expect("Failed to execute rush");
            black_box(output);
        });
    });

    group.finish();
}

/// Benchmark daemon throughput (sequential requests as fast as possible)
fn bench_daemon_throughput(c: &mut Criterion) {
    if !is_daemon_running() {
        start_daemon();
    }

    let mut group = c.benchmark_group("daemon_throughput");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Measure how many requests we can do in a batch
    group.bench_function("batch_100_exit", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let exit_code = execute_via_protocol("exit");
                black_box(exit_code);
            }
        });
    });

    group.bench_function("batch_100_echo", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let exit_code = execute_via_protocol("echo hello");
                black_box(exit_code);
            }
        });
    });

    group.finish();
}

/// Shell comparison benchmarks
fn bench_shell_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_comparison");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(50);

    let shells: Vec<(&str, &str)> = vec![
        ("rush", "target/release/rush"),
        ("bash", "/bin/bash"),
    ];

    // Add zsh if available
    let zsh_path = Command::new("which")
        .arg("zsh")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    for (shell_name, shell_path) in &shells {
        group.bench_with_input(
            BenchmarkId::new("echo_hello", shell_name),
            shell_path,
            |b, path| {
                b.iter(|| {
                    let output = Command::new(path)
                        .arg("-c")
                        .arg("echo hello")
                        .output()
                        .expect("Failed to execute shell");
                    black_box(output);
                });
            },
        );
    }

    if let Some(ref zsh) = zsh_path {
        group.bench_with_input(
            BenchmarkId::new("echo_hello", "zsh"),
            zsh.as_str(),
            |b, path| {
                b.iter(|| {
                    let output = Command::new(path)
                        .arg("-c")
                        .arg("echo hello")
                        .output()
                        .expect("Failed to execute shell");
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    daemon_benches,
    bench_daemon_warm,
    bench_direct_execution,
    bench_daemon_cold_start,
    bench_daemon_overhead,
    bench_daemon_throughput,
    bench_shell_comparison,
);

criterion_main!(daemon_benches);
