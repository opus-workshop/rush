#!/bin/bash
# Quick Claude Code Shell Benchmark
# Compares basic command performance between Rush and Zsh

set -e

RUSH="./target/release/rush"
ZSH="/bin/zsh"
RUNS=20

if [ ! -x "$RUSH" ]; then
    echo "Error: Rush binary not found at $RUSH"
    echo "Run: cargo build --release"
    exit 1
fi

echo "🚀 Quick Claude Code Shell Benchmark"
echo "   Comparing: Rush vs Zsh"
echo "   Runs per test: $RUNS"
echo ""

# Function to time a command
time_command() {
    local shell=$1
    local cmd=$2

    local total=0
    for ((i=1; i<=$RUNS; i++)); do
        local start=$(python3 -c "import time; print(time.perf_counter())")
        $shell -c "$cmd" > /dev/null 2>&1
        local end=$(python3 -c "import time; print(time.perf_counter())")
        local elapsed=$(python3 -c "print(($end - $start) * 1000)")
        total=$(python3 -c "print($total + $elapsed)")
    done

    local avg=$(python3 -c "print($total / $RUNS)")
    echo "$avg"
}

run_test() {
    local name=$1
    local cmd=$2

    printf "  %-35s" "$name:"

    local rush_time=$(time_command "$RUSH" "$cmd")
    printf " Rush: %7.2fms" "$rush_time"

    local zsh_time=$(time_command "$ZSH" "$cmd")
    printf "  Zsh: %7.2fms" "$zsh_time"

    local speedup=$(python3 -c "print($zsh_time / $rush_time)")
    local is_faster=$(python3 -c "print('yes' if $speedup > 1.0 else 'no')")

    printf "  →  %.2fx" "$speedup"
    if [ "$is_faster" = "yes" ]; then
        printf " 🏆 Rush"
    fi
    printf "\n"
}

echo "📊 Shell Startup:"
run_test "Exit immediately" "exit"
run_test "Simple echo" "echo test"
echo ""

echo "📊 Simple Commands:"
run_test "pwd" "pwd"
run_test "Echo variable" "echo \$HOME"
run_test "ls command" "ls > /dev/null"
echo ""

echo "📊 Pipes & Redirects:"
run_test "Pipe to cat" "echo test | cat"
run_test "Redirect to file" "echo test > /tmp/rush_test.txt"
run_test "Append to file" "echo test >> /tmp/rush_test.txt"
echo ""

echo "📊 Command Substitution:"
run_test "Command substitution" 'echo $(pwd)'
run_test "Nested substitution" 'echo $(echo $(pwd))'
echo ""

# Cleanup
rm -f /tmp/rush_test.txt

echo "✅ Quick benchmark complete!"
echo ""
echo "For detailed statistics, run:"
echo "  python3 benches/claude_code_benchmark.py"
