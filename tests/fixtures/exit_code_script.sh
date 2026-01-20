#!/usr/bin/env bash
# Test script that returns specific exit codes
echo "Running commands with different exit codes"
true
echo "True exit code: $?"
false
echo "False exit code: $?"
exit 42
