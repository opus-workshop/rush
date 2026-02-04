# Rush Shell Skill

Use this skill when working with Rush shell commands for faster execution and structured output.

## When to Use

- File operations (ls, cat, find, grep) - Rush builtins are 17-427x faster
- Git operations - Rush has native git2 bindings (5-10x faster)
- When you need JSON output for easier parsing
- High-frequency command execution (Rush daemon mode)

## Available Tools

### `rush` - General command execution
```
rush { command: "ls -la src/", json: true }
```
- Executes any shell command through Rush
- `json: true` (default) requests structured output
- Much faster for built-in commands (ls, cat, grep, find, git)

### `rush_git` - Fast git operations
```
rush_git { operation: "status" }
rush_git { operation: "log", args: "-n 5" }
rush_git { operation: "diff", args: "HEAD~1" }
```
- Native git2 bindings, no git CLI overhead
- Always returns JSON
- Operations: status, log, diff, branch

### `rush_find` - Parallel file search
```
rush_find { path: "src", name: "*.rs", type: "f" }
rush_find { pattern: "**/test*", maxDepth: 3 }
```
- Respects .gitignore automatically
- Parallel traversal for speed
- type: "f" (files), "d" (directories), "all"

### `rush_grep` - Fast text search
```
rush_grep { pattern: "TODO|FIXME", path: "src/" }
rush_grep { pattern: "error", ignoreCase: true, context: 2 }
```
- Ripgrep-powered (10-50x faster than grep)
- Respects .gitignore
- Supports regex patterns

## Commands

- `/rush-daemon start` - Start daemon for 0.4ms latency
- `/rush-daemon stop` - Stop daemon
- `/rush-status` - Show Rush version and mode

## Performance Tips

1. **Use Rush builtins** for file operations:
   - `rush { command: "ls --json" }` instead of `bash ls`
   - `rush { command: "cat file.txt" }` for fast file reads
   - `rush_find` instead of `find` for directory traversal

2. **Enable daemon mode** for high-frequency tasks:
   - `/rush-daemon start` reduces latency from 4.9ms to 0.4ms
   - Ideal for test suites, build systems, CI/CD

3. **Use JSON output** for structured data:
   - Git operations return parseable JSON
   - File listings include metadata
   - Grep results are structured

## Example Workflows

### Fast codebase exploration
```
rush_find { path: ".", name: "*.rs", type: "f" }
rush_grep { pattern: "pub fn", path: "src/" }
rush_git { operation: "status" }
```

### Efficient git workflow
```
rush_git { operation: "status" }
rush_git { operation: "diff" }
rush_git { operation: "log", args: "-n 10 --oneline" }
```

### Large project navigation
```
rush { command: "find . -name '*.ts' | head -100", json: true }
rush { command: "ls -la --json src/" }
```
