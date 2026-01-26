#!/usr/bin/env python3
"""
Rush daemon benchmark client.

Sends commands to the rush daemon via Unix socket and measures latency.
Used by daemon_bench.sh for warm execution benchmarks.

Usage:
    python3 daemon_client.py "echo hello" --iterations 100 --warmup 5 --json
"""

import argparse
import json
import os
import socket
import struct
import sys
import time


SOCKET_PATH = os.path.expanduser("~/.rush/daemon.sock")


def send_message(sock, msg, msg_id=1):
    """Send a length-prefixed message with msg_id."""
    payload = json.dumps(msg).encode()
    length = len(payload) + 4  # payload + 4 bytes for msg_id
    sock.sendall(struct.pack('<I', length))
    sock.sendall(struct.pack('<I', msg_id))
    sock.sendall(payload)


def recv_message(sock):
    """Receive a length-prefixed message."""
    raw_len = b''
    while len(raw_len) < 4:
        chunk = sock.recv(4 - len(raw_len))
        if not chunk:
            raise ConnectionError("Socket closed")
        raw_len += chunk
    length = struct.unpack('<I', raw_len)[0]

    data = b''
    remaining = length
    while len(data) < remaining:
        chunk = sock.recv(remaining - len(data))
        if not chunk:
            raise ConnectionError("Socket closed")
        data += chunk

    # First 4 bytes are msg_id, rest is payload
    msg_id = struct.unpack('<I', data[:4])[0]
    payload = json.loads(data[4:].decode())
    return payload, msg_id


def execute_command(cmd, socket_path=SOCKET_PATH):
    """Execute a command via the daemon, return (result, elapsed_ms)."""
    sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    start = time.perf_counter()
    sock.connect(socket_path)

    msg = {
        "type": "session_init",
        "working_dir": os.getcwd(),
        "env": {"PATH": os.environ.get("PATH", "")},
        "args": ["-c", cmd],
        "stdin_mode": "null",
    }
    send_message(sock, msg)
    result, _ = recv_message(sock)
    elapsed = (time.perf_counter() - start) * 1000
    sock.close()
    return result, elapsed


def main():
    parser = argparse.ArgumentParser(description="Rush daemon benchmark client")
    parser.add_argument("command", help="Command to execute")
    parser.add_argument("--iterations", "-n", type=int, default=100)
    parser.add_argument("--warmup", "-w", type=int, default=5)
    parser.add_argument("--json", action="store_true", help="Output JSON results")
    parser.add_argument("--socket", default=SOCKET_PATH, help="Daemon socket path")
    args = parser.parse_args()

    # Warmup
    for _ in range(args.warmup):
        try:
            execute_command(args.command, args.socket)
        except Exception as e:
            print(f"Warmup failed: {e}", file=sys.stderr)
            sys.exit(1)

    # Benchmark
    times = []
    for _ in range(args.iterations):
        _, elapsed = execute_command(args.command, args.socket)
        times.append(elapsed)

    times.sort()
    result = {
        "command": args.command,
        "iterations": len(times),
        "times_ms": times,
        "min_ms": min(times),
        "max_ms": max(times),
        "mean_ms": sum(times) / len(times),
        "median_ms": times[len(times) // 2],
        "p95_ms": times[int(len(times) * 0.95)],
        "p99_ms": times[int(len(times) * 0.99)],
    }

    if args.json:
        print(json.dumps(result, indent=2))
    else:
        print(f"Command: {args.command}")
        print(f"Iterations: {result['iterations']}")
        print(f"  Min:    {result['min_ms']:.3f} ms")
        print(f"  Median: {result['median_ms']:.3f} ms")
        print(f"  Mean:   {result['mean_ms']:.3f} ms")
        print(f"  P95:    {result['p95_ms']:.3f} ms")
        print(f"  P99:    {result['p99_ms']:.3f} ms")
        print(f"  Max:    {result['max_ms']:.3f} ms")


if __name__ == "__main__":
    main()
