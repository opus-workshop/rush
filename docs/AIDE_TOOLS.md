# AIDE Tool Suite

**Design Document v0.1**
**Date:** 2026-01-20

## Overview

AIDE's tools provide Rush-native capabilities that Claude Code cannot offer. Each tool is designed to leverage Rush's unique features: persistent sessions, performance monitoring, job control, and daemon architecture.

## Design Principles

1. **Leverage Rush internals** - Direct API access vs subprocess calls
2. **Stateful operations** - Tools work with persistent sessions
3. **Structured output** - Return JSON for machine processing, not just text
4. **Performance aware** - Tools report timing and can query perf stats
5. **Job-centric** - Parallel execution as first-class feature

## Tool Categories

### 1. Session Management
### 2. Command Execution
### 3. Job Control
### 4. Performance Analysis
### 5. File Operations (Rush-aware)
### 6. Introspection

---

## 1. Session Management Tools

### `query_session_state`

**Purpose:** Get current Rush session state without executing commands.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {},
  "required": []
}
```

**Output:**
```json
{
  "session_id": 12345,
  "working_dir": "/Users/asher/knowledge/rush",
  "environment": {
    "PATH": "/usr/bin:/bin",
    "RUST_LOG": "debug",
    "CARGO_TARGET_DIR": "target"
  },
  "active_jobs": [
    {
      "id": 1,
      "pid": 54321,
      "command": "cargo test --release",
      "status": "Running",
      "started_at": "2026-01-20T18:30:00Z"
    }
  ],
  "last_exit_code": 0,
  "uptime_seconds": 3600
}
```

**Implementation:**
```rust
async fn query_session_state(
    &self,
    _input: serde_json::Value
) -> Result<String> {
    let state = self.rush_client
        .query_session(self.session_id)
        .await?;

    Ok(serde_json::to_string_pretty(&state)?)
}
```

**Why Rush-specific:** Direct access to daemon session state - no subprocess needed.

---

### `set_environment`

**Purpose:** Modify session environment variables (persists across commands).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "variables": {
      "type": "object",
      "description": "Key-value pairs of environment variables to set",
      "additionalProperties": { "type": "string" }
    }
  },
  "required": ["variables"]
}
```

**Example:**
```json
{
  "variables": {
    "RUST_LOG": "trace",
    "CARGO_INCREMENTAL": "0"
  }
}
```

**Output:**
```json
{
  "updated": ["RUST_LOG", "CARGO_INCREMENTAL"],
  "session_id": 12345
}
```

**Why Rush-specific:** Persistent environment in daemon session. Claude Code would need `export` in every command.

---

## 2. Command Execution Tools

### `execute_in_session`

**Purpose:** Execute command in persistent Rush session with performance tracking.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Rush command to execute"
    },
    "capture_output": {
      "type": "boolean",
      "description": "Whether to capture stdout/stderr",
      "default": true
    },
    "timeout_seconds": {
      "type": "integer",
      "description": "Execution timeout",
      "default": 300
    }
  },
  "required": ["command"]
}
```

**Output:**
```json
{
  "exit_code": 0,
  "stdout": "test result: ok. 42 passed; 0 failed",
  "stderr": "",
  "execution_time_ms": 1234,
  "performance": {
    "lex_time_us": 12.5,
    "parse_time_us": 45.2,
    "expand_time_us": 8.1,
    "execute_time_us": 1168.2
  }
}
```

**Implementation:**
```rust
async fn execute_in_session(
    &self,
    input: serde_json::Value
) -> Result<String> {
    let command = input["command"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing command"))?;

    // Execute via daemon protocol
    let result = self.rush_client
        .execute(self.session_id, command)
        .await?;

    // Query performance stats
    let perf = self.rush_client
        .query_perf_stats(self.session_id)
        .await?;

    let response = json!({
        "exit_code": result.exit_code,
        "stdout": result.stdout,
        "stderr": result.stderr,
        "execution_time_ms": result.duration_ms,
        "performance": perf.last_command,
    });

    Ok(serde_json::to_string_pretty(&response)?)
}
```

**Why Rush-specific:**
- Executes in persistent session (environment carries forward)
- Returns Rush's internal perf stats (not available externally)
- No process startup overhead

---

### `validate_syntax`

**Purpose:** Validate Rush command syntax WITHOUT executing (using parser API).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Command to validate"
    }
  },
  "required": ["command"]
}
```

**Output:**
```json
{
  "valid": true,
  "ast": {
    "type": "Pipeline",
    "commands": [
      {
        "type": "SimpleCommand",
        "program": "cargo",
        "args": ["test", "--release"]
      }
    ]
  }
}
```

**Or on error:**
```json
{
  "valid": false,
  "error": "Unexpected token '|' at position 12",
  "suggestion": "Did you mean '||'?"
}
```

**Implementation:**
```rust
async fn validate_syntax(
    &self,
    input: serde_json::Value
) -> Result<String> {
    let command = input["command"]
        .as_str()
        .ok_or_else(|| anyhow!("Missing command"))?;

    // Use Rush's parser directly (no execution)
    match rush::parser::parse(command) {
        Ok(ast) => {
            Ok(json!({
                "valid": true,
                "ast": ast.to_json(),
            }).to_string())
        }
        Err(e) => {
            Ok(json!({
                "valid": false,
                "error": e.to_string(),
                "suggestion": e.suggestion(),
            }).to_string())
        }
    }
}
```

**Why Rush-specific:** Direct access to Rush parser. Instant validation without subprocess.

---

## 3. Job Control Tools

### `spawn_job`

**Purpose:** Start command as background job (non-blocking).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Command to run in background"
    },
    "name": {
      "type": "string",
      "description": "Human-readable job name"
    }
  },
  "required": ["command"]
}
```

**Example:**
```json
{
  "command": "cargo test --release",
  "name": "release-tests"
}
```

**Output:**
```json
{
  "job_id": 3,
  "pid": 54321,
  "command": "cargo test --release",
  "status": "Running",
  "started_at": "2026-01-20T18:45:00Z"
}
```

---

### `query_jobs`

**Purpose:** Get status of all background jobs.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {},
  "required": []
}
```

**Output:**
```json
{
  "jobs": [
    {
      "id": 1,
      "pid": 54320,
      "command": "cargo build --release",
      "status": "Done",
      "exit_code": 0,
      "started_at": "2026-01-20T18:30:00Z",
      "finished_at": "2026-01-20T18:32:15Z",
      "duration_seconds": 135
    },
    {
      "id": 2,
      "pid": 54321,
      "command": "cargo test --release",
      "status": "Running",
      "started_at": "2026-01-20T18:35:00Z"
    },
    {
      "id": 3,
      "pid": 54322,
      "command": "cargo clippy --all-targets",
      "status": "Done",
      "exit_code": 1,
      "started_at": "2026-01-20T18:40:00Z",
      "finished_at": "2026-01-20T18:41:30Z"
    }
  ],
  "active_count": 1,
  "total_count": 3
}
```

**Implementation:**
```rust
async fn query_jobs(&self, _input: serde_json::Value) -> Result<String> {
    let jobs = self.rush_client
        .list_jobs(self.session_id)
        .await?;

    let active_count = jobs.iter()
        .filter(|j| j.status == JobStatus::Running)
        .count();

    Ok(json!({
        "jobs": jobs,
        "active_count": active_count,
        "total_count": jobs.len(),
    }).to_string())
}
```

**Why Rush-specific:** Access to Rush's job control system. Can manage multiple parallel tasks.

---

### `wait_for_job`

**Purpose:** Block until specific job completes.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "job_id": {
      "type": "integer",
      "description": "Job ID to wait for"
    },
    "timeout_seconds": {
      "type": "integer",
      "description": "Max wait time",
      "default": 300
    }
  },
  "required": ["job_id"]
}
```

**Output:**
```json
{
  "job_id": 2,
  "status": "Done",
  "exit_code": 0,
  "duration_seconds": 145,
  "stdout": "test result: ok. 42 passed; 0 failed",
  "stderr": ""
}
```

---

## 4. Performance Analysis Tools

### `query_perf_stats`

**Purpose:** Get Rush's internal performance statistics.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "last_n_commands": {
      "type": "integer",
      "description": "Number of recent commands to include",
      "default": 10
    }
  },
  "required": []
}
```

**Output:**
```json
{
  "aggregate": {
    "total_commands": 142,
    "avg_lex_time_us": 12.5,
    "avg_parse_time_us": 45.2,
    "avg_expand_time_us": 8.1,
    "avg_execute_time_us": 1168.2
  },
  "recent": [
    {
      "command": "cargo test",
      "lex_time_us": 13.1,
      "parse_time_us": 48.5,
      "expand_time_us": 9.2,
      "execute_time_us": 3245.1,
      "total_time_us": 3315.9
    }
  ],
  "anomalies": [
    {
      "command": "cargo test",
      "phase": "execute",
      "actual_us": 3245.1,
      "baseline_us": 1168.2,
      "deviation_percent": 177.8,
      "severity": "high"
    }
  ]
}
```

**Implementation:**
```rust
async fn query_perf_stats(
    &self,
    input: serde_json::Value
) -> Result<String> {
    let last_n = input["last_n_commands"]
        .as_i64()
        .unwrap_or(10) as usize;

    // Query Rush's performance stats
    let perf = self.rush_client
        .get_perf_stats(self.session_id, last_n)
        .await?;

    // Detect anomalies vs baseline
    let anomalies = self.perf_baseline
        .detect_anomalies(&perf.recent);

    Ok(json!({
        "aggregate": perf.aggregate,
        "recent": perf.recent,
        "anomalies": anomalies,
    }).to_string())
}
```

**Why Rush-specific:** Direct access to Rush's `perf.rs` stats. Invisible to external tools.

---

### `compare_performance`

**Purpose:** Compare performance before/after code changes.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "baseline_checkpoint": {
      "type": "string",
      "description": "Git commit or tag for baseline"
    },
    "test_command": {
      "type": "string",
      "description": "Command to benchmark",
      "default": "cargo test"
    },
    "iterations": {
      "type": "integer",
      "description": "Number of runs",
      "default": 5
    }
  },
  "required": ["baseline_checkpoint"]
}
```

**Output:**
```json
{
  "baseline": {
    "commit": "abc123",
    "avg_time_ms": 1234,
    "std_dev_ms": 45
  },
  "current": {
    "commit": "def456",
    "avg_time_ms": 987,
    "std_dev_ms": 32
  },
  "improvement_percent": 20.0,
  "significant": true
}
```

---

## 5. File Operations (Rush-aware)

### `read_file`

**Purpose:** Read file with Rush context awareness.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "File path (absolute or relative to session working dir)"
    }
  },
  "required": ["path"]
}
```

**Output:**
```json
{
  "path": "/Users/asher/knowledge/rush/src/parser/mod.rs",
  "content": "...",
  "line_count": 456,
  "size_bytes": 12345
}
```

**Why Rush-specific:** Resolves paths relative to session working directory.

---

### `edit_file`

**Purpose:** Edit file with validation and rollback support.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "path": {
      "type": "string",
      "description": "File path"
    },
    "edits": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "old_text": { "type": "string" },
          "new_text": { "type": "string" }
        }
      }
    }
  },
  "required": ["path", "edits"]
}
```

**Output:**
```json
{
  "path": "src/parser/mod.rs",
  "edits_applied": 2,
  "backup_path": ".aide/session-42/backups/src_parser_mod.rs.2026-01-20-184500"
}
```

**Why Rush-specific:** Automatic backups in session directory. Can rollback on test failure.

---

## 6. Introspection Tools

### `explain_command`

**Purpose:** Explain what a Rush command will do (using parser + AST analysis).

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "Command to explain"
    }
  },
  "required": ["command"]
}
```

**Output:**
```json
{
  "command": "cargo test --release && cargo build --profile bench",
  "explanation": "This is a pipeline with two commands connected by AND (&&):",
  "steps": [
    {
      "step": 1,
      "command": "cargo test --release",
      "description": "Run tests in release mode",
      "will_execute_if": "always (first command)",
      "estimated_risk": "low"
    },
    {
      "step": 2,
      "command": "cargo build --profile bench",
      "description": "Build with benchmark profile",
      "will_execute_if": "previous command succeeds (exit code 0)",
      "estimated_risk": "low"
    }
  ],
  "overall_risk": "low",
  "estimated_time_seconds": 45
}
```

**Why Rush-specific:** Uses Rush's parser to understand command structure.

---

## Tool Suite Summary

| Category | Tools | Key Benefit |
|----------|-------|-------------|
| **Session** | query_session_state, set_environment | Persistent state management |
| **Execution** | execute_in_session, validate_syntax | Perf tracking + pre-validation |
| **Jobs** | spawn_job, query_jobs, wait_for_job | Parallel task orchestration |
| **Performance** | query_perf_stats, compare_performance | Anomaly detection + regression testing |
| **Files** | read_file, edit_file | Session-aware paths + auto-backup |
| **Introspection** | explain_command | Risk analysis before execution |

**Total:** 12 core tools (vs Claude Code's generic Bash tool)

## What Claude Code Can't Do

1. **Persistent sessions** - Environment/state carries forward
2. **Performance monitoring** - Access to Rush's internal timing
3. **Job orchestration** - Parallel tasks with status tracking
4. **Syntax pre-validation** - Parse before execute
5. **Structured output** - JSON vs text parsing
6. **Risk analysis** - AST-based safety scoring

---

**Next:** Implement Rush daemon client and wire up tools to agent loop.
