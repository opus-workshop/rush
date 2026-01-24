#!/bin/bash
# Rush performance monitoring hook
# Called after command execution to capture performance data

# Read JSON input from stdin
input=$(cat)

# Extract command and result from JSON
command=$(echo "$input" | jq -r '.tool_input.command // empty')
exit_code=$(echo "$input" | jq -r '.tool_result.exit_code // 0')

# Log to performance file (for future analysis)
perf_log="/Users/asher/knowledge/rush/.claude/rush-perf.jsonl"

# Create log entry
timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
echo "{\"timestamp\":\"$timestamp\",\"command\":\"$command\",\"exit_code\":$exit_code}" >> "$perf_log"

exit 0
