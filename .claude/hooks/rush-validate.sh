#!/bin/bash
# Rush syntax validation hook
# Called before command execution to validate syntax

# Read JSON input from stdin
input=$(cat)

# Extract command from JSON
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# If no command, allow (not a Bash tool call)
if [ -z "$command" ]; then
  exit 0
fi

# Skip validation for certain safe commands
case "$command" in
  pwd|ls|echo*|cat*|which*)
    exit 0
    ;;
esac

# For now, just pass through - we'll add actual validation later
# Once Rush has a --check-syntax flag, we can use:
# echo "$command" | /Users/asher/knowledge/rush/target/release/rush --check-syntax

exit 0
