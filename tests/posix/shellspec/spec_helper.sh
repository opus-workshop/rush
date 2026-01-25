# ShellSpec Helper Functions
# Shared utilities for POSIX compliance testing

# Get the rush binary path
rush_binary() {
    if [ -n "${RUSH_BINARY:-}" ]; then
        echo "$RUSH_BINARY"
    else
        echo "../../target/release/rush"
    fi
}

# Run a rush command
rush() {
    "$(rush_binary)" "$@"
}

# Run rush with -c flag (command string)
rush_c() {
    "$(rush_binary)" -c "$1"
}

# Check if rush binary exists
rush_exists() {
    [ -f "$(rush_binary)" ]
}

# Get rush version
rush_version() {
    "$(rush_binary)" --version 2>&1 || echo "unknown"
}
