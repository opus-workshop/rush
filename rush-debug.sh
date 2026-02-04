#!/bin/bash
exec > /tmp/rush-debug.log 2>&1
echo "=== Rush Debug $(date) ==="
echo "PWD: $PWD"
echo "TERM: $TERM"
echo "SHELL: $SHELL"
echo "Args: $@"
echo "TTY: $(tty)"
env | sort
echo "=== Starting rush ==="
exec /usr/local/bin/rush --login
