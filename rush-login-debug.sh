#!/bin/bash
# Debug wrapper to capture rush login shell issues

exec 2>/tmp/rush-login-debug.log
set -x

echo "=== Rush Login Debug ===" >&2
echo "Date: $(date)" >&2
echo "argv[0]: $0" >&2
echo "PID: $$" >&2
echo "PPID: $PPID" >&2
echo "TTY: $(tty)" >&2
echo "TERM: $TERM" >&2
echo "SHELL: $SHELL" >&2
echo "Process group: $(ps -o pgid= -p $$)" >&2
echo "Foreground pgid: $(ps -o tpgid= -p $$)" >&2
echo "Session ID: $(ps -o sess= -p $$)" >&2
echo "" >&2

# Try running rush and capture any error
echo "Starting rush..." >&2
/Users/asher/tt/rush/target/release/rush --login 2>&1
EXIT_CODE=$?
echo "Rush exited with code: $EXIT_CODE" >&2
