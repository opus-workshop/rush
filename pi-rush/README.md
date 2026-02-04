# pi-rush

Pi extension for [Rush shell](https://github.com/paiml/rush) - high-performance shell commands with JSON output and bidirectional IPC.

## Features

- **Fast builtins** - Rush's native commands are 17-427x faster than GNU equivalents
- **JSON output** - Structured data for better AI parsing
- **Native git** - git2 bindings (5-10x faster than git CLI)
- **Daemon mode** - 0.4ms latency for high-frequency operations
- **Rush ↔ Pi IPC** - Unix socket daemon for `|?` operator integration
- **Skill** - Teaches the model to use Rush effectively

## Installation

```bash
# Install Rush first
brew install opus-workshop/rush/rush
# or: cargo install --git https://github.com/paiml/rush

# Install pi-rush package
pi install git:github.com/paiml/rush --path pi-rush
```

## Tools

### `rush` - General command execution
```
rush { command: "ls -la src/", json: true }
```

### `rush_git` - Fast git operations
```
rush_git { operation: "status" }
rush_git { operation: "log", args: "-n 5" }
```

### `rush_find` - Parallel file search
```
rush_find { path: "src", name: "*.rs", type: "f" }
```

### `rush_grep` - Fast text search
```
rush_grep { pattern: "TODO", path: "src/" }
```

## Commands

| Command | Description |
|---------|-------------|
| `/rush-daemon start` | Start Rush daemon (0.4ms latency) |
| `/rush-daemon stop` | Stop Rush daemon |
| `/rush-status` | Show Rush version and mode |
| `/pi-daemon start` | Start Pi↔Rush IPC socket server |
| `/pi-daemon stop` | Stop Pi↔Rush IPC socket server |
| `/pi-daemon status` | Show IPC daemon status |

## Rush ↔ Pi IPC Protocol

The daemon extension creates a Unix socket server at `~/.pi/rush.sock` that enables bidirectional communication between Rush shell and Pi agent.

### Architecture

```
┌─────────────────┐     Unix Socket      ┌─────────────────┐
│   Rush Shell    │◄───────────────────►│    Pi Agent     │
│                 │  ~/.pi/rush.sock     │                 │
│  • Shell exec   │                      │  • LLM provider │
│  • Job control  │◄────── context ──────│  • Tool use     │
│  • History      │                      │  • Memory       │
│  • |? operator  │────── queries ──────►│  • Streaming    │
└─────────────────┘                      └─────────────────┘
```

### Message Protocol (JSONL)

Messages are newline-delimited JSON. Each message has a `type` field.

#### Rush → Pi Messages

**Query** - Send a prompt to the LLM:
```json
{
  "type": "query",
  "id": "req-123",
  "prompt": "explain this error",
  "stdin": "error: cannot find crate...",
  "context": {
    "cwd": "/home/user/project",
    "last_command": "cargo build",
    "last_exit_code": 1,
    "history": ["cd project", "cargo build"],
    "env": {"SHELL": "/bin/zsh", "USER": "user"}
  }
}
```

**Tool Result** - Return tool execution result:
```json
{
  "type": "tool_result",
  "id": "tool-456",
  "output": "total 32\ndrwxr-xr-x 5 user staff 160 Jan 1 12:00 .",
  "exit_code": 0
}
```

**Intent** - Convert natural language to shell command:
```json
{
  "type": "intent",
  "id": "intent-123",
  "intent": "find all rust files modified today",
  "context": {
    "cwd": "/home/user/project",
    "last_command": "cargo build",
    "last_exit_code": 0,
    "history": ["cd project", "cargo build"],
    "env": {"SHELL": "/bin/zsh"}
  },
  "project_type": "rust"
}
```

#### Pi → Rush Messages

**Chunk** - Streaming content fragment:
```json
{"type": "chunk", "id": "req-123", "content": "The error "}
{"type": "chunk", "id": "req-123", "content": "indicates..."}
```

**Done** - Stream complete:
```json
{"type": "done", "id": "req-123"}
```

**Error** - Error occurred:
```json
{"type": "error", "id": "req-123", "message": "Rate limit exceeded"}
```

**Tool Call** - Pi wants to execute a command:
```json
{
  "type": "tool_call",
  "id": "tool-789",
  "tool": "bash",
  "args": {"command": "ls -la", "timeout": 30}
}
```

**Suggested Command** - Response to intent query:
```json
{
  "type": "suggested_command",
  "id": "intent-123",
  "command": "find . -name \"*.rs\" -mtime 0",
  "explanation": "Finds all Rust files modified today",
  "confidence": 0.95
}
```

### Usage in Rush

```bash
# Pipe command output to LLM
git diff |? "write a commit message for these changes"

# Query with context
cargo build 2>&1 |? "fix this error"

# Natural language to command (? prefix)
? find all rust files modified today
# Pi suggests: find . -name "*.rs" -mtime 0
# [Enter] Execute  [Tab] Edit  [Ctrl-C] Cancel

? deploy to staging
# Pi suggests: git push origin main && ssh staging "./deploy.sh"

? list all TODO comments in src
# Pi suggests: grep -rn "TODO" src/
```

The `?` prefix detects project type automatically (rust, node, python, etc.) to provide more relevant commands.

### Session Context

Pi uses the shell context to:
- Set working directory for file operations
- Include command history for context
- Understand if previous command failed (exit code)
- Access relevant environment variables

## Performance

| Operation | bash | Rush | Speedup |
|-----------|------|------|---------|
| ls (1000 files) | 12ms | 0.1ms | 120x |
| grep pattern | 45ms | 0.2ms | 212x |
| cat small file | 8ms | 0.02ms | 427x |
| Startup (daemon) | N/A | 0.4ms | - |

## Why?

AI coding agents make hundreds of shell calls per task. Rush's combination of:
- Fast native builtins (no fork/exec overhead)
- JSON output (structured data for parsing)
- Daemon mode (minimal latency)
- Bidirectional IPC (Pi can call back to Rush)

...makes it ideal for AI agent workflows. This extension exposes those capabilities to Pi.

## Development

```bash
# Test the extensions locally
cd rush

# Load rush tools only
pi -e ./pi-rush/extensions/rush.ts

# Load daemon only
pi -e ./pi-rush/extensions/daemon.ts

# Load both
pi -e ./pi-rush/extensions/rush.ts -e ./pi-rush/extensions/daemon.ts

# Or load the skill
pi --skill ./pi-rush/skills/rush/SKILL.md
```

### Testing the Daemon

```bash
# Start Pi with daemon extension
pi -e ./pi-rush/extensions/daemon.ts

# In another terminal, test with socat
echo '{"type":"query","id":"test-1","prompt":"hello","stdin":null,"context":{"cwd":"/tmp","last_command":null,"last_exit_code":null,"history":[],"env":{}}}' | socat - UNIX-CONNECT:~/.pi/rush.sock

# Or use netcat
echo '{"type":"query","id":"test-1","prompt":"hello","stdin":null,"context":{"cwd":"/tmp","last_command":null,"last_exit_code":null,"history":[],"env":{}}}' | nc -U ~/.pi/rush.sock
```

## License

MIT - same as Rush
