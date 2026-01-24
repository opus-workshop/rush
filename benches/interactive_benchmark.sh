#!/bin/bash
# Interactive Shell Benchmark: Rush vs Zsh
# Measures performance in a PERSISTENT shell session (like claude-code uses)

set -e

RUSH="./target/release/rush"
ZSH="/bin/zsh"
RUNS=50

if [ ! -x "$RUSH" ]; then
    echo "Error: Rush binary not found at $RUSH"
    echo "Run: cargo build --release"
    exit 1
fi

echo "🚀 Interactive Shell Benchmark (Claude Code Use Case)"
echo "   Tests commands in a PERSISTENT shell session"
echo "   Runs per test: $RUNS"
echo ""

# Create named pipes for communication
RUSH_IN=$(mktemp -u)
RUSH_OUT=$(mktemp -u)
ZSH_IN=$(mktemp -u)
ZSH_OUT=$(mktemp -u)

mkfifo "$RUSH_IN" "$RUSH_OUT" "$ZSH_IN" "$ZSH_OUT"

# Cleanup function
cleanup() {
    rm -f "$RUSH_IN" "$RUSH_OUT" "$ZSH_IN" "$ZSH_OUT"
    # Kill background shells if they exist
    jobs -p | xargs -r kill 2>/dev/null || true
}
trap cleanup EXIT

# Start persistent shell sessions
$RUSH < "$RUSH_IN" > "$RUSH_OUT" 2>&1 &
RUSH_PID=$!

$ZSH < "$ZSH_IN" > "$ZSH_OUT" 2>&1 &
ZSH_PID=$!

# Wait for shells to start
sleep 0.5

# Function to time a command in a persistent shell
time_in_shell() {
    local in_pipe=$1
    local out_pipe=$2
    local cmd=$3

    local total=0
    for ((i=1; i<=$RUNS; i++)); do
        local start=$(python3 -c "import time; print(time.perf_counter())")

        # Send command and unique marker
        echo "$cmd; echo '__DONE__'" > "$in_pipe"

        # Wait for completion marker
        while IFS= read -r line; do
            if [[ "$line" == *"__DONE__"* ]]; then
                break
            fi
        done < "$out_pipe"

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

    local rush_time=$(time_in_shell "$RUSH_IN" "$RUSH_OUT" "$cmd")
    printf " Rush: %7.2fms" "$rush_time"

    local zsh_time=$(time_in_shell "$ZSH_IN" "$ZSH_OUT" "$cmd")
    printf "  Zsh: %7.2fms" "$zsh_time"

    local speedup=$(python3 -c "print($zsh_time / $rush_time)")
    printf "  →  %.2fx" "$speedup"

    if [ $(python3 -c "print(1 if $speedup > 1.0 else 0)") -eq 1 ]; then
        printf " 🏆"
    fi
    printf "\n"
}

echo "📊 Simple Commands:"
run_test "pwd" "pwd"
run_test "echo test" "echo test"
run_test "echo \$HOME" "echo \$HOME"
echo ""

echo "📊 Git Operations (with builtin):"
run_test "git status" "git status"
run_test "git branch" "git branch"
echo ""

echo "📊 File Operations:"
run_test "ls" "ls"
run_test "cat README.md" "cat README.md"
echo ""

echo "📊 Pipes & Substitution:"
run_test "echo test | cat" "echo test | cat"
run_test "echo \$(pwd)" "echo \$(pwd)"
echo ""

# Cleanup happens automatically via trap
echo "✅ Interactive benchmark complete!"
echo ""
echo "This benchmark measures command execution in a PERSISTENT shell"
echo "session, which is how claude-code actually uses the shell."
