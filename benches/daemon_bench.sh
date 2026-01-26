#!/usr/bin/env bash
#
# Rush Daemon Performance Benchmark Suite
# ========================================
#
# Measures daemon vs direct execution latency for the rush shell.
#
# What it measures:
#   1. Cold start -- first command after daemon starts (includes fork + init)
#   2. Warm execution -- subsequent commands via daemon (pre-warmed workers)
#   3. Direct execution -- rush -c (fast path, no daemon)
#   4. Shell comparison -- bash -c and zsh -c baselines
#   5. Throughput -- sustained command execution rate
#
# Usage:
#   ./benches/daemon_bench.sh              # Full benchmark suite
#   ./benches/daemon_bench.sh --quick      # Quick smoke test (fewer iterations)
#   ./benches/daemon_bench.sh --no-build   # Skip cargo build step
#   ./benches/daemon_bench.sh --help       # Show help
#
# Requirements:
#   - cargo (for building release binary, unless --no-build)
#   - python3 (for daemon client + statistics)
#   - hyperfine (optional, for shell comparison)
#

set -euo pipefail

# --- Configuration ---

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUSH="${PROJECT_DIR}/target/release/rush"
RUSHD="${PROJECT_DIR}/target/release/rushd"
DAEMON_CLIENT="${SCRIPT_DIR}/daemon_client.py"
SOCKET_PATH="${HOME}/.rush/daemon.sock"
PID_PATH="${HOME}/.rush/daemon.pid"
RESULTS_DIR="${PROJECT_DIR}/benchmarks"
RESULTS_FILE="${RESULTS_DIR}/DAEMON_PERF.md"

# Benchmark parameters
WARMUP=5
ITERATIONS=100
QUICK_WARMUP=2
QUICK_ITERATIONS=20

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' BLUE='' BOLD='' NC=''
fi

# --- Helpers ---

info()  { printf "${BLUE}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
err()   { printf "${RED}[ERR]${NC}   %s\n" "$*" >&2; }
header(){ printf "\n${BOLD}=== %s ===${NC}\n\n" "$*"; }

DAEMON_PID=""

cleanup() {
    # Stop daemon if we started it
    if [ -n "${DAEMON_PID}" ]; then
        info "Stopping daemon (PID ${DAEMON_PID})..."
        kill "$DAEMON_PID" 2>/dev/null || true
        wait "$DAEMON_PID" 2>/dev/null || true
        rm -f "$SOCKET_PATH" "$PID_PATH"
    fi
    # Clean up temp files
    rm -f /tmp/rush_bench_*.tmp
}

trap cleanup EXIT

wait_for_daemon() {
    local max_wait=50  # 5 seconds
    local i=0
    while [ $i -lt $max_wait ]; do
        if [ -S "$SOCKET_PATH" ]; then
            # Verify we can connect
            if python3 -c "
import socket, sys
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
try:
    s.connect('$SOCKET_PATH')
    s.close()
    sys.exit(0)
except:
    sys.exit(1)
" 2>/dev/null; then
                return 0
            fi
        fi
        sleep 0.1
        i=$((i + 1))
    done
    return 1
}

start_daemon() {
    info "Starting rush daemon..."

    # Stop any existing daemon
    if [ -S "$SOCKET_PATH" ]; then
        "$RUSHD" stop 2>/dev/null || true
        sleep 0.5
    fi

    # Start daemon in background
    "$RUSHD" start &
    DAEMON_PID=$!

    if wait_for_daemon; then
        ok "Daemon started (PID ${DAEMON_PID})"
    else
        err "Failed to start daemon within 5 seconds"
        exit 1
    fi
}

stop_daemon() {
    if [ -n "${DAEMON_PID}" ]; then
        info "Stopping daemon..."
        kill "$DAEMON_PID" 2>/dev/null || true
        wait "$DAEMON_PID" 2>/dev/null || true
        DAEMON_PID=""
        rm -f "$SOCKET_PATH" "$PID_PATH"
        sleep 0.3
        ok "Daemon stopped"
    fi
}

# Run daemon benchmark via Python client, output JSON
daemon_bench() {
    local cmd="$1"
    local iters="${2:-$ITERATIONS}"
    local warmup="${3:-$WARMUP}"

    python3 "$DAEMON_CLIENT" "$cmd" \
        --iterations "$iters" \
        --warmup "$warmup" \
        --json
}

# Run direct (rush -c) benchmark with timing
direct_bench() {
    local cmd="$1"
    local iters="${2:-$ITERATIONS}"
    local warmup="${3:-$WARMUP}"

    python3 -c "
import subprocess, time, json, sys

cmd = sys.argv[1]
rush = sys.argv[2]
iters = int(sys.argv[3])
warmup = int(sys.argv[4])

# Warmup
for _ in range(warmup):
    subprocess.run([rush, '-c', cmd], capture_output=True)

# Benchmark
times = []
for _ in range(iters):
    start = time.perf_counter()
    subprocess.run([rush, '-c', cmd], capture_output=True)
    elapsed = (time.perf_counter() - start) * 1000
    times.append(elapsed)

times.sort()
result = {
    'command': cmd,
    'iterations': len(times),
    'times_ms': times,
    'min_ms': min(times),
    'max_ms': max(times),
    'mean_ms': sum(times) / len(times),
    'median_ms': times[len(times) // 2],
}
print(json.dumps(result, indent=2))
" "$cmd" "$RUSH" "$iters" "$warmup"
}

# Run shell benchmark (bash/zsh -c) with timing
shell_bench() {
    local shell="$1"
    local cmd="$2"
    local iters="${3:-$ITERATIONS}"
    local warmup="${4:-$WARMUP}"

    python3 -c "
import subprocess, time, json, sys

cmd = sys.argv[1]
shell = sys.argv[2]
iters = int(sys.argv[3])
warmup = int(sys.argv[4])

# Warmup
for _ in range(warmup):
    subprocess.run([shell, '-c', cmd], capture_output=True)

# Benchmark
times = []
for _ in range(iters):
    start = time.perf_counter()
    subprocess.run([shell, '-c', cmd], capture_output=True)
    elapsed = (time.perf_counter() - start) * 1000
    times.append(elapsed)

times.sort()
result = {
    'command': cmd,
    'iterations': len(times),
    'times_ms': times,
    'min_ms': min(times),
    'max_ms': max(times),
    'mean_ms': sum(times) / len(times),
    'median_ms': times[len(times) // 2],
}
print(json.dumps(result, indent=2))
" "$cmd" "$shell" "$iters" "$warmup"
}

# Extract stat from JSON output
jq_stat() {
    local json_data="$1"
    local field="$2"
    echo "$json_data" | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{d[\"$field\"]:.3f}')"
}

# Print comparison row
print_row() {
    local label="$1"
    local daemon_json="$2"
    local direct_json="$3"

    local d_median=$(jq_stat "$daemon_json" "median_ms")
    local d_min=$(jq_stat "$daemon_json" "min_ms")
    local r_median=$(jq_stat "$direct_json" "median_ms")
    local r_min=$(jq_stat "$direct_json" "min_ms")

    local overhead
    overhead=$(python3 -c "print(f'{$d_median - $r_median:+.3f}')")

    printf "  %-25s  %8s ms  %8s ms  %8s ms  %10s ms\n" \
        "$label" "$d_median" "$d_min" "$r_median" "$overhead"
}

# --- Cold Start Measurement ---

measure_cold_start() {
    local cmd="$1"
    local iters="${2:-5}"

    python3 -c "
import json, os, signal, socket, struct, subprocess, sys, time

cmd = sys.argv[1]
rushd = sys.argv[2]
socket_path = sys.argv[3]
pid_path = sys.argv[4]
iters = int(sys.argv[5])

def stop_daemon():
    if os.path.exists(pid_path):
        try:
            pid = int(open(pid_path).read().strip())
            os.kill(pid, signal.SIGTERM)
            time.sleep(0.5)
        except:
            pass
    for f in [socket_path, pid_path]:
        if os.path.exists(f):
            try:
                os.remove(f)
            except:
                pass

def start_daemon():
    proc = subprocess.Popen([rushd, 'start'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    for _ in range(50):
        if os.path.exists(socket_path):
            s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            try:
                s.connect(socket_path)
                s.close()
                return proc
            except:
                pass
        time.sleep(0.1)
    raise RuntimeError('Daemon failed to start')

def send_msg(sock, msg, msg_id=1):
    payload = json.dumps(msg).encode()
    length = len(payload) + 4
    sock.sendall(struct.pack('<II', length, msg_id) + payload)

def recv_msg(sock):
    raw_len = b''
    while len(raw_len) < 4:
        raw_len += sock.recv(4 - len(raw_len))
    length = struct.unpack('<I', raw_len)[0]
    data = b''
    remaining = length
    while len(data) < remaining:
        data += sock.recv(remaining - len(data))
    return json.loads(data[4:].decode())

times = []
for i in range(iters):
    stop_daemon()
    time.sleep(0.3)
    proc = start_daemon()

    # First command after daemon start = cold start
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    start = time.perf_counter()
    s.connect(socket_path)
    msg = {
        'type': 'session_init',
        'working_dir': os.getcwd(),
        'env': {'PATH': os.environ.get('PATH', '')},
        'args': ['-c', cmd],
        'stdin_mode': 'null',
    }
    send_msg(s, msg)
    result = recv_msg(s)
    elapsed = (time.perf_counter() - start) * 1000
    s.close()
    times.append(elapsed)

    stop_daemon()

result = {
    'command': cmd,
    'type': 'cold_start',
    'iterations': len(times),
    'times_ms': times,
    'min_ms': min(times),
    'max_ms': max(times),
    'mean_ms': sum(times) / len(times),
    'median_ms': sorted(times)[len(times) // 2],
}
print(json.dumps(result, indent=2))
" "$cmd" "$RUSHD" "$SOCKET_PATH" "$PID_PATH" "$iters"
}

# --- Throughput Measurement ---

measure_throughput() {
    local cmd="$1"
    local duration_secs="${2:-3}"

    python3 -c "
import json, os, socket, struct, sys, time

cmd = sys.argv[1]
socket_path = sys.argv[2]
duration = float(sys.argv[3])

def send_msg(sock, msg, msg_id=1):
    payload = json.dumps(msg).encode()
    length = len(payload) + 4
    sock.sendall(struct.pack('<II', length, msg_id) + payload)

def recv_msg(sock):
    raw_len = b''
    while len(raw_len) < 4:
        raw_len += sock.recv(4 - len(raw_len))
    length = struct.unpack('<I', raw_len)[0]
    data = b''
    remaining = length
    while len(data) < remaining:
        data += sock.recv(remaining - len(data))
    return json.loads(data[4:].decode())

count = 0
start = time.perf_counter()
times = []

while (time.perf_counter() - start) < duration:
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    t0 = time.perf_counter()
    s.connect(socket_path)
    msg = {
        'type': 'session_init',
        'working_dir': os.getcwd(),
        'env': {'PATH': os.environ.get('PATH', '')},
        'args': ['-c', cmd],
        'stdin_mode': 'null',
    }
    send_msg(s, msg)
    result = recv_msg(s)
    t1 = time.perf_counter()
    s.close()
    times.append((t1 - t0) * 1000)
    count += 1

elapsed = time.perf_counter() - start
rps = count / elapsed

result = {
    'command': cmd,
    'type': 'throughput',
    'total_requests': count,
    'elapsed_secs': round(elapsed, 3),
    'requests_per_sec': round(rps, 1),
    'mean_latency_ms': round(sum(times) / len(times), 3) if times else 0,
    'p50_ms': round(sorted(times)[len(times) // 2], 3) if times else 0,
    'p99_ms': round(sorted(times)[int(len(times) * 0.99)], 3) if times else 0,
}
print(json.dumps(result, indent=2))
" "$cmd" "$SOCKET_PATH" "$duration_secs"
}

# --- Main ---

usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Rush daemon performance benchmark suite."
    echo ""
    echo "Options:"
    echo "  --quick     Run with fewer iterations (smoke test)"
    echo "  --no-build  Skip cargo build --release"
    echo "  --help      Show this help"
}

main() {
    local quick=false
    local skip_build=false

    while [ $# -gt 0 ]; do
        case "$1" in
            --quick)    quick=true; shift ;;
            --no-build) skip_build=true; shift ;;
            --help|-h)  usage; exit 0 ;;
            *)          err "Unknown option: $1"; usage; exit 1 ;;
        esac
    done

    if $quick; then
        WARMUP=$QUICK_WARMUP
        ITERATIONS=$QUICK_ITERATIONS
    fi

    header "Rush Daemon Performance Benchmark"
    info "Date: $(date '+%Y-%m-%d %H:%M:%S')"
    info "System: $(uname -srm)"
    info "Iterations: $ITERATIONS (warmup: $WARMUP)"

    # --- Build ---
    if ! $skip_build; then
        header "Building release binary"
        if (cd "$PROJECT_DIR" && cargo build --release 2>&1); then
            ok "Build complete"
        else
            warn "cargo build --release failed; checking for existing binaries..."
            if [ -x "$RUSH" ] && [ -x "$RUSHD" ]; then
                ok "Using existing release binaries"
            else
                err "No release binaries available. Run 'cargo build --release' manually."
                exit 1
            fi
        fi
    fi

    # Verify binaries exist
    for bin in "$RUSH" "$RUSHD"; do
        if [ ! -x "$bin" ]; then
            err "Binary not found: $bin"
            exit 1
        fi
    done

    ok "rush binary: $RUSH"
    ok "rushd binary: $RUSHD"

    # Check for hyperfine
    local has_hyperfine=false
    if command -v hyperfine &>/dev/null; then
        has_hyperfine=true
        ok "hyperfine available"
    else
        warn "hyperfine not found; shell comparison will use Python timing"
    fi

    # Test commands to benchmark
    COMMANDS=("exit" "echo hello" "true" "/bin/ls /tmp" "echo foo | cat")
    LABELS=("exit (no-op)" "echo hello" "true builtin" "ls (external)" "echo | cat (pipe)")

    # =========================================================================
    # 1. Cold Start
    # =========================================================================
    header "1. Cold Start (first command after daemon start)"

    local cold_iters=3
    if $quick; then cold_iters=2; fi

    printf "  %-25s  %10s  %10s  %10s\n" "Command" "Median" "Min" "Max"
    printf "  %-25s  %10s  %10s  %10s\n" "-------" "------" "---" "---"

    # Store results in temp files since bash arrays don't handle spaces well
    for i in "${!COMMANDS[@]}"; do
        local cmd="${COMMANDS[$i]}"
        local label="${LABELS[$i]}"
        local result
        result=$(measure_cold_start "$cmd" "$cold_iters" 2>/dev/null) || {
            warn "Cold start measurement failed for '$label'"
            result='{"median_ms":0,"min_ms":0,"max_ms":0}'
        }
        echo "$result" > "/tmp/rush_bench_cold_${i}.tmp"

        local median=$(jq_stat "$result" "median_ms")
        local min_t=$(jq_stat "$result" "min_ms")
        local max_t=$(jq_stat "$result" "max_ms")
        printf "  %-25s  %7s ms  %7s ms  %7s ms\n" "$label" "$median" "$min_t" "$max_t"
    done

    # =========================================================================
    # 2. Warm Daemon Execution
    # =========================================================================
    header "2. Daemon vs Direct Execution (warm)"

    start_daemon
    echo ""

    printf "  %-25s  %12s  %12s  %12s  %14s\n" \
        "Command" "Daemon Med" "Daemon Min" "Direct Med" "Overhead"
    printf "  %-25s  %12s  %12s  %12s  %14s\n" \
        "-------" "----------" "----------" "----------" "--------"

    for i in "${!COMMANDS[@]}"; do
        local cmd="${COMMANDS[$i]}"
        local label="${LABELS[$i]}"

        local d_result
        d_result=$(daemon_bench "$cmd" "$ITERATIONS" "$WARMUP" 2>/dev/null) || {
            warn "Daemon benchmark failed for '$label'"
            d_result='{"median_ms":0,"min_ms":0}'
        }
        echo "$d_result" > "/tmp/rush_bench_daemon_${i}.tmp"

        local r_result
        r_result=$(direct_bench "$cmd" "$ITERATIONS" "$WARMUP" 2>/dev/null) || {
            warn "Direct benchmark failed for '$label'"
            r_result='{"median_ms":0,"min_ms":0}'
        }
        echo "$r_result" > "/tmp/rush_bench_direct_${i}.tmp"

        print_row "$label" "$d_result" "$r_result"
    done

    # =========================================================================
    # 3. Shell Comparison (with hyperfine or Python timing)
    # =========================================================================
    header "3. Shell Comparison"

    if $has_hyperfine; then
        info "Running hyperfine comparison..."
        echo ""

        local -a shells=("$RUSH")
        local -a shell_names=("rush")

        if command -v bash &>/dev/null; then
            shells+=("$(command -v bash)")
            shell_names+=("bash")
        fi
        if command -v zsh &>/dev/null; then
            shells+=("$(command -v zsh)")
            shell_names+=("zsh")
        fi

        # Benchmark: echo hello
        local -a hyperfine_args=("--warmup" "5" "--min-runs" "50")
        for j in "${!shells[@]}"; do
            hyperfine_args+=("-n" "${shell_names[$j]}" "${shells[$j]} -c 'echo hello'")
        done

        hyperfine "${hyperfine_args[@]}" 2>&1 || true
        echo ""

        # Benchmark: exit (pure startup cost)
        local -a hyperfine_args2=("--warmup" "5" "--min-runs" "50")
        for j in "${!shells[@]}"; do
            hyperfine_args2+=("-n" "${shell_names[$j]}" "${shells[$j]} -c 'exit'")
        done

        hyperfine "${hyperfine_args2[@]}" 2>&1 || true
    else
        # Fallback: Python-based timing
        printf "  %-25s  %12s  %12s  %12s\n" "Command" "Rush" "Bash" "Zsh"
        printf "  %-25s  %12s  %12s  %12s\n" "-------" "----" "----" "---"

        local iters=$((ITERATIONS / 2))
        local warm=$((WARMUP / 2))
        if [ "$iters" -lt 10 ]; then iters=10; fi
        if [ "$warm" -lt 1 ]; then warm=1; fi

        for i in "${!COMMANDS[@]}"; do
            local cmd="${COMMANDS[$i]}"
            local label="${LABELS[$i]}"

            local rush_r
            rush_r=$(shell_bench "$RUSH" "$cmd" "$iters" "$warm" 2>/dev/null)
            local rush_med=$(jq_stat "$rush_r" "median_ms")

            local bash_med="-"
            if command -v bash &>/dev/null; then
                local bash_r
                bash_r=$(shell_bench "$(command -v bash)" "$cmd" "$iters" "$warm" 2>/dev/null)
                bash_med=$(jq_stat "$bash_r" "median_ms")
            fi

            local zsh_med="-"
            if command -v zsh &>/dev/null; then
                local zsh_r
                zsh_r=$(shell_bench "$(command -v zsh)" "$cmd" "$iters" "$warm" 2>/dev/null)
                zsh_med=$(jq_stat "$zsh_r" "median_ms")
            fi

            printf "  %-25s  %8s ms  %8s ms  %8s ms\n" "$label" "$rush_med" "$bash_med" "$zsh_med"
        done
    fi

    # =========================================================================
    # 4. Throughput
    # =========================================================================
    header "4. Daemon Throughput"

    # Make sure daemon is running
    if [ ! -S "$SOCKET_PATH" ]; then
        start_daemon
    fi

    local tput_duration=3
    if $quick; then tput_duration=1; fi

    printf "  %-25s  %8s  %12s  %8s  %8s\n" \
        "Command" "RPS" "Mean Latency" "P50" "P99"
    printf "  %-25s  %8s  %12s  %8s  %8s\n" \
        "-------" "---" "------------" "---" "---"

    for i in "${!COMMANDS[@]}"; do
        local cmd="${COMMANDS[$i]}"
        local label="${LABELS[$i]}"

        local tp_result
        tp_result=$(measure_throughput "$cmd" "$tput_duration" 2>/dev/null) || {
            warn "Throughput measurement failed for '$label'"
            tp_result='{"requests_per_sec":0,"mean_latency_ms":0,"p50_ms":0,"p99_ms":0}'
        }

        local rps mean_l p50 p99
        rps=$(echo "$tp_result" | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{d[\"requests_per_sec\"]:.1f}')")
        mean_l=$(echo "$tp_result" | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{d[\"mean_latency_ms\"]:.3f}')")
        p50=$(echo "$tp_result" | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{d[\"p50_ms\"]:.3f}')")
        p99=$(echo "$tp_result" | python3 -c "import json,sys; d=json.load(sys.stdin); print(f'{d[\"p99_ms\"]:.3f}')")

        printf "  %-25s  %7s  %9s ms  %6s ms  %6s ms\n" \
            "$label" "$rps" "$mean_l" "$p50" "$p99"
    done

    # =========================================================================
    # 5. Summary
    # =========================================================================
    header "5. Summary"

    # Read stored results
    local exit_daemon_json exit_direct_json echo_daemon_json echo_direct_json cold_exit_json
    exit_daemon_json=$(cat /tmp/rush_bench_daemon_0.tmp 2>/dev/null || echo '{"median_ms":0}')
    exit_direct_json=$(cat /tmp/rush_bench_direct_0.tmp 2>/dev/null || echo '{"median_ms":0}')
    echo_daemon_json=$(cat /tmp/rush_bench_daemon_1.tmp 2>/dev/null || echo '{"median_ms":0}')
    echo_direct_json=$(cat /tmp/rush_bench_direct_1.tmp 2>/dev/null || echo '{"median_ms":0}')
    cold_exit_json=$(cat /tmp/rush_bench_cold_0.tmp 2>/dev/null || echo '{"median_ms":0}')

    local exit_daemon_med exit_direct_med echo_daemon_med echo_direct_med cold_exit_med
    exit_daemon_med=$(jq_stat "$exit_daemon_json" "median_ms")
    exit_direct_med=$(jq_stat "$exit_direct_json" "median_ms")
    echo_daemon_med=$(jq_stat "$echo_daemon_json" "median_ms")
    echo_direct_med=$(jq_stat "$echo_direct_json" "median_ms")
    cold_exit_med=$(jq_stat "$cold_exit_json" "median_ms")

    local exit_overhead echo_overhead
    exit_overhead=$(python3 -c "print(f'{$exit_daemon_med - $exit_direct_med:+.3f}')")
    echo_overhead=$(python3 -c "print(f'{$echo_daemon_med - $echo_direct_med:+.3f}')")

    echo "  Daemon execution overhead (median):"
    echo "    exit:  daemon=${exit_daemon_med}ms  direct=${exit_direct_med}ms  overhead=${exit_overhead} ms"
    echo "    echo:  daemon=${echo_daemon_med}ms  direct=${echo_direct_med}ms  overhead=${echo_overhead} ms"
    echo ""
    echo "  Cold start (first command after daemon restart):"
    echo "    exit:  ${cold_exit_med}ms"
    echo ""

    ok "Benchmark complete!"
    echo ""

    # Stop daemon
    stop_daemon

    # --- Generate results file ---
    generate_results_doc \
        "$exit_daemon_med" "$exit_direct_med" "$exit_overhead" \
        "$echo_daemon_med" "$echo_direct_med" "$echo_overhead" \
        "$cold_exit_med"
}

# --- Generate Results Documentation ---

generate_results_doc() {
    local exit_daemon="$1" exit_direct="$2" exit_overhead="$3"
    local echo_daemon="$4" echo_direct="$5" echo_overhead="$6"
    local cold_exit="$7"

    mkdir -p "$RESULTS_DIR"

    info "Writing results to ${RESULTS_FILE}"

    local cpu_info
    cpu_info=$(sysctl -n machdep.cpu.brand_string 2>/dev/null || lscpu 2>/dev/null | grep 'Model name' | sed 's/.*: *//' || echo "unknown")
    local system_info
    system_info=$(uname -srm)
    local date_str
    date_str=$(date '+%Y-%m-%d %H:%M:%S')

    cat > "$RESULTS_FILE" << MARKDOWN_EOF
# Rush Daemon Performance Results

> Generated: ${date_str}
> System: ${system_info}
> CPU: ${cpu_info}

## Overview

The rush daemon provides persistent process pooling to amortize shell
initialization cost across multiple command invocations. Commands routed
through the daemon skip binary loading, allocator init, and module setup
that direct invocation (\`rush -c\`) must pay on every call.

## Architecture

\`\`\`
Direct:   [client process] -> fork+exec rush -> lex/parse/exec -> exit
Daemon:   [client process] -> unix socket -> [pre-forked worker] -> lex/parse/exec -> respond
\`\`\`

- **rushd start** -- starts the daemon, listens on \`~/.rush/daemon.sock\`
- **rush -c "cmd"** -- fast path, direct execution (no daemon)
- **DaemonClient** -- library client: connects to socket, sends SessionInit, receives ExecutionResult

The daemon protocol uses length-prefixed JSON over a Unix domain socket:
\`\`\`
[4-byte LE length] [4-byte LE message_id] [JSON payload]
\`\`\`

## Results

### Cold Start (first command after daemon start)

First command latency includes daemon accept + fork + worker init.

| Command | Median |
|---------|--------|
| exit (no-op) | ${cold_exit} ms |

### Warm Execution (daemon vs direct)

Steady-state latency after daemon is warmed up.

| Command | Daemon (median) | Direct (median) | Overhead |
|---------|-----------------|-----------------|----------|
| exit (no-op) | ${exit_daemon} ms | ${exit_direct} ms | ${exit_overhead} ms |
| echo hello | ${echo_daemon} ms | ${echo_direct} ms | ${echo_overhead} ms |

### Interpretation

- **Daemon path**: Unix socket connect + JSON message serialization + daemon accept
  + fork/dispatch worker + lex/parse/exec + JSON response + socket close.
- **Direct path**: Process spawn (fork+exec) + rush binary load + allocator init +
  lex/parse/exec + process exit.
- The daemon amortizes binary loading and initialization across requests. When daemon
  overhead < direct overhead, the daemon provides a net latency win.
- The fast path (\`rush -c\`) skips signal handlers, process groups, environment init,
  and daemon probe, making it extremely fast for direct execution.

## Reproducing

\`\`\`bash
# Full suite (100 iterations per test)
./benches/daemon_bench.sh

# Quick smoke test (20 iterations)
./benches/daemon_bench.sh --quick

# Skip rebuild (use existing binaries)
./benches/daemon_bench.sh --no-build
\`\`\`

## Benchmark Methodology

- **Cold start**: Daemon is stopped and restarted before each measurement.
  The first command sent after restart is timed end-to-end (socket connect
  through response received). Repeated 3 times, median reported.
- **Warm execution**: Daemon is started once, warmup commands are sent,
  then N iterations are timed. Reports median to reduce outlier impact.
- **Direct execution**: \`rush -c "cmd"\` is invoked as a subprocess via
  Python's subprocess module, measuring full process lifecycle (fork+exec
  to exit).
- **Throughput**: Sequential requests sent over a fixed duration, measuring
  requests per second and latency percentiles (P50, P99).
- **Shell comparison**: \`hyperfine\` (if available) or Python timing compares
  rush direct execution against bash and zsh.
- All timing uses \`time.perf_counter()\` (monotonic, sub-microsecond resolution).
MARKDOWN_EOF

    ok "Results written to ${RESULTS_FILE}"
}

main "$@"
