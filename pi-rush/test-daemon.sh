#!/bin/bash
# Test script for Pi-Rush daemon
# 
# This script verifies the daemon socket is working by sending a simple query.
# 
# Usage:
#   1. Start Pi with daemon extension: pi -e ./pi-rush/extensions/daemon.ts
#   2. In another terminal: ./pi-rush/test-daemon.sh
#
# Expected: The script sends a query and receives streaming chunks back.

SOCKET_PATH="$HOME/.pi/rush.sock"

# Check if socket exists
if [ ! -S "$SOCKET_PATH" ]; then
    echo "Error: Socket not found at $SOCKET_PATH"
    echo "Make sure Pi is running with the daemon extension:"
    echo "  pi -e ./pi-rush/extensions/daemon.ts"
    exit 1
fi

echo "Testing Pi-Rush daemon at $SOCKET_PATH"
echo "Sending query..."
echo

# Send a test query
# Using socat if available, otherwise nc
if command -v socat &> /dev/null; then
    echo '{"type":"query","id":"test-1","prompt":"Say hello in exactly 3 words","stdin":null,"context":{"cwd":"/tmp","last_command":"echo test","last_exit_code":0,"history":["cd /tmp","echo test"],"env":{"SHELL":"/bin/zsh","USER":"'"$USER"'"}}}' | \
        socat -t 30 - UNIX-CONNECT:"$SOCKET_PATH"
elif command -v nc &> /dev/null; then
    echo '{"type":"query","id":"test-1","prompt":"Say hello in exactly 3 words","stdin":null,"context":{"cwd":"/tmp","last_command":"echo test","last_exit_code":0,"history":["cd /tmp","echo test"],"env":{"SHELL":"/bin/zsh","USER":"'"$USER"'"}}}' | \
        nc -U "$SOCKET_PATH"
else
    echo "Error: Neither socat nor nc (netcat) found. Install one to test the socket."
    exit 1
fi

echo
echo "---"
echo "Testing intent-to-command (? prefix)..."
echo

# Send an intent query
if command -v socat &> /dev/null; then
    echo '{"type":"intent","id":"intent-1","intent":"find all rust files modified today","context":{"cwd":"'"$PWD"'","last_command":"ls","last_exit_code":0,"history":["cd project","ls"],"env":{"SHELL":"/bin/zsh","USER":"'"$USER"'"}},"project_type":"rust"}' | \
        socat -t 30 - UNIX-CONNECT:"$SOCKET_PATH"
elif command -v nc &> /dev/null; then
    echo '{"type":"intent","id":"intent-1","intent":"find all rust files modified today","context":{"cwd":"'"$PWD"'","last_command":"ls","last_exit_code":0,"history":["cd project","ls"],"env":{"SHELL":"/bin/zsh","USER":"'"$USER"'"}},"project_type":"rust"}' | \
        nc -U "$SOCKET_PATH"
fi

echo
echo "Test complete."
