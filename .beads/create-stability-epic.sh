#!/bin/bash
# Rush Shell Stability Epic - Make Rush Ready for Login Shell
# Based on docs/stability-roadmap.md

set -e

echo "Creating Rush Shell Stability epic and beads..."

# Quality Gates for Rush (Rust project):
# - cargo test (all tests pass)
# - cargo build --release (builds successfully)
# - cargo clippy -- -D warnings (no clippy warnings)
# - Manual integration tests for login shell scenarios

# Create epic
echo "Creating epic..."
EPIC_ID=$(bd create --type=epic \
  --title="Rush Shell Stability - Login Shell Ready" \
  --description="Make Rush stable enough to be used as a default login shell on macOS/Linux.

## Success Criteria
- ✅ All tests pass (including new stability tests)
- ✅ Can source standard shell configs (~/.profile, ~/.rushrc equivalents)
- ✅ Can run common system scripts without errors
- ✅ Handles signals gracefully (no orphaned processes)
- ✅ Works in both TTY and non-TTY modes
- ✅ Exit codes propagate correctly
- ✅ No crashes during normal operation
- ✅ Can survive and recover from parse errors
- ✅ Job control works (background jobs don't block)
- ✅ Used successfully as login shell for 1 week without issues

## Timeline
3-4 weeks to production-ready:
- Week 1: Critical blockers (non-TTY, signals, redirection, subshells, exit codes)
- Week 2: Core features (variable expansion, wildcards, command substitution)
- Week 3: Job control and polish
- Week 4: Stability testing and integration tests

## Reference
See docs/stability-roadmap.md for complete details." \
  --labels="rush,stability,login-shell,epic" \
  --silent)

echo "Epic created: $EPIC_ID"

# ============================================================================
# WEEK 1: CRITICAL BLOCKERS
# ============================================================================

# Story 1: Non-TTY Mode Support (MOST CRITICAL)
echo "Creating Story 1: Non-TTY Mode Support..."
STORY_1=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-001: Non-TTY Mode Support" \
  --description="As a user running rush in non-interactive contexts, I need rush to work without a TTY so that login scripts, pipes, and command substitution work correctly.

## Priority: P0 - BLOCKING
This is the single most critical blocker for login shell use.

## Problem
- Reedline requires TTY and crashes on non-interactive use
- Breaks: login scripts, \`echo \"pwd\" | rush\`, \`\$(rush -c \"cmd\")\`
- Makes rush unusable for automation or as a login shell

## Acceptance Criteria
- [ ] Detects TTY vs non-TTY mode using atty crate
- [ ] Interactive mode: Uses reedline for rich line editing
- [ ] Non-interactive mode: Uses simple stdin reader
- [ ] \`echo \"pwd\" | rush\` works
- [ ] \`rush -c \"echo test\"\` works (already works)
- [ ] \`rush < script.sh\` works
- [ ] \`\$(rush -c \"echo nested\")\` works (command substitution)
- [ ] Can be used in cron jobs
- [ ] cargo test passes (all tests including non-TTY tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
// In main.rs
use atty;

fn run_interactive() -> Result<()> {
    if atty::is(atty::Stream::Stdin) {
        run_interactive_with_reedline()
    } else {
        run_non_interactive()
    }
}

fn run_non_interactive() -> Result<()> {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        execute_line(&line, &mut executor)?;
    }
    Ok(())
}
\`\`\`

## Files to Modify
- src/main.rs:188 - Split run_interactive() into TTY/non-TTY paths
- Cargo.toml - Add atty dependency

## Testing
- Unit tests for non-TTY execution
- Integration test: echo commands through pipe
- Integration test: command substitution
- Integration test: script redirection

## Estimated Effort: 1-2 days" \
  --priority=1 \
  --labels="rush,stability,p0,critical,non-tty" \
  --silent)

echo "Story 1 created: $STORY_1"

# Story 2: Signal Handling
echo "Creating Story 2: Signal Handling..."
STORY_2=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-002: Signal Handling" \
  --description="As a user, I need rush to handle signals properly so that there are no orphaned processes or corrupted state when I press Ctrl-C.

## Priority: P0 - BLOCKING
Critical for preventing orphaned processes and data corruption.

## Problem
- No SIGINT, SIGTERM, SIGHUP handlers
- Ctrl-C leaves orphaned child processes
- State corruption possible on unexpected termination

## Acceptance Criteria
- [ ] SIGINT (Ctrl-C) handler implemented
- [ ] SIGTERM handler implemented
- [ ] SIGHUP handler implemented
- [ ] Child processes cleaned up on signal
- [ ] Exit with correct signal exit code (130 for SIGINT)
- [ ] State is saved/cleaned before exit
- [ ] No orphaned processes after signal
- [ ] cargo test passes (all tests including signal tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
use signal_hook::{consts::{SIGINT, SIGTERM, SIGHUP}, iterator::Signals};

fn setup_signal_handlers() -> Result<()> {
    let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGHUP])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT => handle_sigint(),
                SIGTERM => handle_sigterm(),
                SIGHUP => handle_sighup(),
                _ => {}
            }
        }
    });

    Ok(())
}
\`\`\`

## Files to Modify
- src/main.rs - Register signal handlers on startup
- src/executor/mod.rs - Add signal handling to executor
- Cargo.toml - Add signal_hook dependency

## Testing
- Test Ctrl-C during long-running command
- Test killing rush process during pipeline
- Test no orphaned processes after signal
- Test state cleanup on signal

## Estimated Effort: 1 day" \
  --priority=1 \
  --labels="rush,stability,p0,critical,signals" \
  --silent)

echo "Story 2 created: $STORY_2"

# Story 3: File Redirection
echo "Creating Story 3: File Redirection..."
STORY_3=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-003: File Redirection Support" \
  --description="As a shell user, I need standard Unix redirections so that I can redirect output to files and chain commands properly.

## Priority: P0 - BLOCKING
Essential for basic shell scripting.

## Problem
- No \`>\`, \`>>\`, \`<\`, \`2>\`, \`2>&1\`, \`&>\` support
- Cannot save command output to files
- Cannot redirect error streams

## Acceptance Criteria
- [ ] \`>\` stdout redirect: \`echo \"test\" > file.txt\`
- [ ] \`>>\` append redirect: \`echo \"more\" >> file.txt\`
- [ ] \`<\` stdin redirect: \`command < input.txt\`
- [ ] \`2>\` stderr redirect: \`command 2> errors.log\`
- [ ] \`2>&1\` stderr to stdout: \`command 2>&1 | grep error\`
- [ ] \`&>\` both to file: \`command &> output.log\`
- [ ] Works with pipes: \`cmd | grep foo > results.txt\`
- [ ] File permissions handled correctly
- [ ] Handles missing parent directories gracefully
- [ ] cargo test passes (all tests including redirection tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
// In parser/ast.rs
pub enum Redirect {
    Stdout(PathBuf),       // >
    StdoutAppend(PathBuf), // >>
    Stdin(PathBuf),        // <
    Stderr(PathBuf),       // 2>
    StderrToStdout,        // 2>&1
    Both(PathBuf),         // &>
}

pub struct Command {
    // ... existing fields
    pub redirects: Vec<Redirect>,
}
\`\`\`

## Files to Modify
- src/parser/ast.rs - Add Redirect enum to AST
- src/lexer/mod.rs - Tokenize redirect operators
- src/parser/mod.rs - Parse redirections
- src/executor/mod.rs - Set up file descriptors before exec

## Testing
- Test all redirect operators individually
- Test redirect with pipes
- Test redirect with builtins
- Test redirect file permissions
- Test redirect error handling (read-only filesystem, etc.)

## Estimated Effort: 2 days" \
  --priority=1 \
  --labels="rush,stability,p0,critical,redirection" \
  --silent)

echo "Story 3 created: $STORY_3"

# Story 4: Subshell Support
echo "Creating Story 4: Subshell Support..."
STORY_4=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-004: Subshell Support" \
  --description="As a shell scripter, I need subshells \`( )\` so that I can isolate environment changes and run commands in isolated contexts.

## Priority: P0 - BLOCKING
Many scripts rely on subshells for environment isolation.

## Problem
- No support for \`( )\` subshells
- Cannot isolate environment changes
- Scripts using subshells fail

## Acceptance Criteria
- [ ] Parse \`( )\` syntax correctly
- [ ] Subshell gets isolated runtime/environment
- [ ] Variables set in subshell don't affect parent
- [ ] \`cd\` in subshell doesn't affect parent
- [ ] Subshell exit code propagates correctly
- [ ] Works with pipes: \`(cd /tmp && ls) | grep foo\`
- [ ] Nested subshells work: \`((echo nested))\`
- [ ] cargo test passes (all tests including subshell tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
// In parser/ast.rs
pub enum Statement {
    // ... existing variants
    Subshell(Vec<Statement>),
}

// In executor/mod.rs
Statement::Subshell(statements) => {
    let mut child_runtime = self.runtime.clone();
    let mut child_executor = Executor::with_runtime(child_runtime);
    child_executor.execute(statements)
}
\`\`\`

## Files to Modify
- src/parser/ast.rs - Add Subshell variant
- src/lexer/mod.rs - Tokenize \`(\` and \`)\`
- src/parser/mod.rs - Parse subshell blocks
- src/executor/mod.rs - Execute with cloned runtime
- src/runtime/mod.rs - Ensure runtime is Clone

## Testing
- Test variable isolation
- Test directory isolation (cd)
- Test exit code propagation
- Test nested subshells
- Test subshells in pipes

## Estimated Effort: 1-2 days" \
  --priority=1 \
  --labels="rush,stability,p0,critical,subshells" \
  --silent)

echo "Story 4 created: $STORY_4"

# Story 5: Exit Code Propagation
echo "Creating Story 5: Exit Code Propagation..."
STORY_5=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-005: Exit Code Propagation" \
  --description="As a shell scripter, I need proper exit code handling so that error detection and conditional execution work correctly.

## Priority: P0 - BLOCKING
Essential for error handling in scripts.

## Problem
- Exit codes not properly propagated through pipelines
- No \$? variable support
- Scripts using \`set -e\` or \`||\` fail

## Acceptance Criteria
- [ ] \$? variable contains last exit code
- [ ] Pipeline exit code is last command (default)
- [ ] Pipeline exit code is first failure (with set -o pipefail)
- [ ] \`command && next\` only runs if success
- [ ] \`command || fallback\` only runs if failure
- [ ] Exit codes work with builtins
- [ ] Exit codes work with external commands
- [ ] cargo test passes (all tests including exit code tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
// In runtime/mod.rs
pub fn set_last_exit_code(&mut self, code: i32) {
    self.set_variable(\"?\".to_string(), code.to_string());
}

pub fn get_last_exit_code(&self) -> i32 {
    self.get_variable(\"?\")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
\`\`\`

## Files to Modify
- src/runtime/mod.rs - Add \$? special variable
- src/executor/pipeline.rs - Track exit codes through pipeline
- src/executor/mod.rs - Update last exit code after each command
- src/parser/mod.rs - Parse && and || operators

## Testing
- Test \$? variable after success and failure
- Test pipeline exit codes
- Test && conditional execution
- Test || conditional execution
- Test mixed conditions

## Estimated Effort: 1 day" \
  --priority=1 \
  --labels="rush,stability,p0,critical,exit-codes" \
  --silent)

echo "Story 5 created: $STORY_5"

# ============================================================================
# WEEK 2: CORE FEATURES
# ============================================================================

# Story 6: Variable Expansion
echo "Creating Story 6: Variable Expansion..."
STORY_6=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-006: Proper Variable Expansion" \
  --description="As a shell scripter, I need POSIX variable expansion so that my scripts can handle defaults, errors, and string manipulation.

## Priority: P1 - HIGH
Critical for POSIX compatibility.

## Acceptance Criteria
- [ ] \${VAR} - Braced variables
- [ ] \${VAR:-default} - Use default if unset
- [ ] \${VAR:=default} - Assign default if unset
- [ ] \${VAR:?error} - Error if unset
- [ ] \${VAR#pattern} - Remove shortest prefix match
- [ ] \${VAR##pattern} - Remove longest prefix match
- [ ] \${VAR%pattern} - Remove shortest suffix match
- [ ] \${VAR%%pattern} - Remove longest suffix match
- [ ] Works with arrays/lists (future)
- [ ] cargo test passes (all tests including expansion tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
pub fn expand_variable(&self, expr: &str) -> Result<String> {
    // Parse \${VAR:-default} syntax
    // Apply transformation based on operator
    // Return expanded value
}
\`\`\`

## Files to Modify
- src/runtime/mod.rs - Implement variable expansion
- src/lexer/mod.rs - Tokenize braced variables
- src/parser/mod.rs - Parse variable expansions

## Testing
- Test all expansion operators
- Test nested expansions
- Test edge cases (empty, unset, special chars)

## Estimated Effort: 2 days" \
  --priority=2 \
  --labels="rush,stability,p1,variables,posix" \
  --silent)

echo "Story 6 created: $STORY_6"

# Story 7: Wildcard Expansion
echo "Creating Story 7: Wildcard Expansion..."
STORY_7=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-007: Wildcard Expansion (Globbing)" \
  --description="As a shell user, I need glob patterns to work so that I can match multiple files with wildcards.

## Priority: P1 - HIGH
Basic shell functionality.

## Acceptance Criteria
- [ ] \`*\` matches any characters
- [ ] \`?\` matches single character
- [ ] \`[...]\` character classes work
- [ ] \`**\` recursive glob (bonus)
- [ ] Dotfiles not matched by default
- [ ] Empty glob returns error (not literal)
- [ ] Works with multiple patterns
- [ ] cargo test passes (all tests including glob tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
use glob::glob;

fn expand_globs(pattern: &str) -> Result<Vec<PathBuf>> {
    let matches = glob(pattern)?
        .filter_map(Result::ok)
        .collect();
    Ok(matches)
}
\`\`\`

## Files to Modify
- src/executor/mod.rs - Expand globs before execution
- Cargo.toml - Add glob crate

## Testing
- Test * expansion
- Test ? expansion
- Test [...] expansion
- Test dotfile handling
- Test no matches error

## Estimated Effort: 1 day" \
  --priority=2 \
  --labels="rush,stability,p1,globbing" \
  --silent)

echo "Story 7 created: $STORY_7"

# Story 8: Command Substitution
echo "Creating Story 8: Command Substitution..."
STORY_8=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-008: Command Substitution" \
  --description="As a shell scripter, I need command substitution so that I can use command output in other commands.

## Priority: P1 - HIGH
Essential for shell scripting.

## Acceptance Criteria
- [ ] \$(command) syntax works
- [ ] \\\`command\\\` backticks work (bonus)
- [ ] Nested substitution works: \$(echo \$(pwd))
- [ ] Substitution in strings: \"path: \$(pwd)\"
- [ ] Exit code of substituted command available
- [ ] Whitespace handling correct (trimming)
- [ ] Works in pipes and redirects
- [ ] cargo test passes (all tests including substitution tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
pub fn execute_command_substitution(&mut self, command: &str) -> Result<String> {
    let result = self.execute_line(command)?;
    Ok(result.stdout.trim_end().to_string())
}
\`\`\`

## Files to Modify
- src/parser/mod.rs - Parse \$() syntax
- src/executor/mod.rs - Execute and capture output
- src/runtime/mod.rs - Substitute into parent command

## Testing
- Test simple substitution
- Test nested substitution
- Test substitution in strings
- Test whitespace handling

## Estimated Effort: 1 day" \
  --priority=2 \
  --labels="rush,stability,p1,substitution" \
  --silent)

echo "Story 8 created: $STORY_8"

# ============================================================================
# WEEK 3: JOB CONTROL & POLISH
# ============================================================================

# Story 9: Job Control
echo "Creating Story 9: Job Control..."
STORY_9=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-009: Job Control (Background Jobs)" \
  --description="As a power user, I need job control so that I can run long tasks in the background and manage multiple jobs.

## Priority: P2 - MEDIUM
Important for UX, not critical for basic shell use.

## Acceptance Criteria
- [ ] \`command &\` runs in background
- [ ] \`jobs\` builtin lists background jobs
- [ ] \`fg [job]\` brings job to foreground
- [ ] \`bg [job]\` continues stopped job in background
- [ ] \`kill %job\` kills job by number
- [ ] SIGCHLD handler for job status updates
- [ ] Job status shown in prompt (optional)
- [ ] cargo test passes (all tests including job tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
pub struct JobManager {
    jobs: Vec<Job>,
}

pub struct Job {
    id: usize,
    pid: u32,
    command: String,
    status: JobStatus,
}

enum JobStatus {
    Running,
    Stopped,
    Done,
}
\`\`\`

## Files to Modify
- src/executor/mod.rs - Handle & operator
- src/runtime/mod.rs - Add JobManager
- src/builtins/mod.rs - Add jobs, fg, bg builtins
- src/main.rs - Handle SIGCHLD

## Testing
- Test background execution
- Test jobs listing
- Test fg/bg commands
- Test job completion detection

## Estimated Effort: 3 days" \
  --priority=3 \
  --labels="rush,stability,p2,job-control" \
  --silent)

echo "Story 9 created: $STORY_9"

# Story 10: Error Recovery
echo "Creating Story 10: Error Recovery..."
STORY_10=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-010: Error Recovery" \
  --description="As a user, I need rush to recover from errors gracefully so that it doesn't crash and I don't lose my session.

## Priority: P1 - HIGH
Critical for login shell stability.

## Acceptance Criteria
- [ ] Parse errors don't crash shell
- [ ] Execution errors don't crash shell
- [ ] Returns to prompt after error
- [ ] Logs errors without exiting
- [ ] Panic recovery handler (last resort)
- [ ] State corruption recovery
- [ ] History saved on crash
- [ ] cargo test passes (all tests including recovery tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
fn run_interactive() -> Result<()> {
    loop {
        match std::panic::catch_unwind(|| {
            // Execute line with panic protection
        }) {
            Ok(Ok(result)) => handle_success(result),
            Ok(Err(e)) => handle_error(e),
            Err(panic) => handle_panic(panic),
        }
    }
}
\`\`\`

## Files to Modify
- src/main.rs - Add panic recovery
- src/executor/mod.rs - Better error handling
- src/parser/mod.rs - Graceful parse error handling

## Testing
- Test parse errors don't crash
- Test execution errors don't crash
- Test panic recovery (if possible)

## Estimated Effort: 1 day" \
  --priority=2 \
  --labels="rush,stability,p1,error-handling" \
  --silent)

echo "Story 10 created: $STORY_10"

# Story 11: Login Shell Initialization
echo "Creating Story 11: Login Shell Initialization..."
STORY_11=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-011: Login Shell Initialization" \
  --description="As a rush user, I need proper login shell initialization so that my environment is set up correctly on login.

## Priority: P2 - MEDIUM
Important for login shell use.

## Acceptance Criteria
- [ ] Sources ~/.rush_profile on login shell
- [ ] Sources ~/.rushrc on interactive shell
- [ ] Sets \$SHELL environment variable
- [ ] Sets \$TERM if not set
- [ ] Sets \$USER, \$HOME correctly
- [ ] --login flag forces login shell behavior
- [ ] --no-rc skips config files
- [ ] cargo test passes (all tests including init tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
fn run_login_shell() -> Result<()> {
    // Set environment variables
    env::set_var(\"SHELL\", env::current_exe()?);

    // Source profile
    if let Some(profile) = find_profile() {
        source_file(&profile)?;
    }

    // Run interactive
    run_interactive()
}
\`\`\`

## Files to Modify
- src/main.rs - Add login shell detection and init
- src/executor/mod.rs - Add source_file function

## Testing
- Test profile sourcing
- Test environment variable setup
- Test --login and --no-rc flags

## Estimated Effort: 1 day" \
  --priority=3 \
  --labels="rush,stability,p2,initialization" \
  --silent)

echo "Story 11 created: $STORY_11"

# Story 12: Shell Options
echo "Creating Story 12: Shell Options (set -e, -u, -x, etc.)..."
STORY_12=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-012: Shell Options (set command)" \
  --description="As a shell scripter, I need shell options so that I can control error handling and debugging behavior.

## Priority: P2 - MEDIUM
Improves script reliability.

## Acceptance Criteria
- [ ] \`set -e\` - Exit on error
- [ ] \`set -u\` - Error on undefined variables
- [ ] \`set -x\` - Print commands before execution
- [ ] \`set -o pipefail\` - Pipeline fails if any command fails
- [ ] \`set +e\`, \`set +u\`, etc. to unset options
- [ ] \`set\` with no args shows current options
- [ ] Options affect current shell and subshells
- [ ] cargo test passes (all tests including set tests)
- [ ] cargo build --release succeeds
- [ ] cargo clippy -- -D warnings passes

## Technical Implementation
\`\`\`rust
pub struct ShellOptions {
    errexit: bool,     // -e
    nounset: bool,     // -u
    xtrace: bool,      // -x
    pipefail: bool,    // -o pipefail
}

impl Runtime {
    pub fn set_option(&mut self, option: &str, value: bool) {
        // Update options
    }
}
\`\`\`

## Files to Modify
- src/runtime/mod.rs - Add ShellOptions struct
- src/builtins/mod.rs - Add set builtin
- src/executor/mod.rs - Check options during execution

## Testing
- Test each option individually
- Test option combinations
- Test set/unset toggle

## Estimated Effort: 2 days" \
  --priority=3 \
  --labels="rush,stability,p2,shell-options" \
  --silent)

echo "Story 12 created: $STORY_12"

# ============================================================================
# WEEK 4: INTEGRATION TESTING
# ============================================================================

# Story 13: Integration Tests
echo "Creating Story 13: Integration Tests..."
STORY_13=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-013: Integration Tests for Login Shell" \
  --description="As a rush developer, I need comprehensive integration tests so that I can verify rush works correctly as a login shell.

## Priority: P1 - HIGH
Essential for verification.

## Acceptance Criteria
- [ ] Non-interactive mode tests (echo | rush)
- [ ] Script execution tests
- [ ] Command substitution tests
- [ ] Signal handling tests
- [ ] Redirection tests
- [ ] Pipeline tests
- [ ] Exit code tests
- [ ] Job control tests (if implemented)
- [ ] All integration tests pass
- [ ] Tests run in CI

## Test Suite
\`\`\`bash
# tests/integration/login_shell_test.sh
#!/bin/bash

# Test 1: Non-interactive mode
echo \"pwd\" | ./target/release/rush

# Test 2: Script execution
./target/release/rush tests/fixtures/test_script.sh

# Test 3: Command substitution
./target/release/rush -c \"echo \$(./target/release/rush -c 'echo nested')\"

# Test 4: Signal handling
./target/release/rush -c \"sleep 100\" &
kill \$!

# Test 5: Exit codes
./target/release/rush -c \"false\" ; echo \$?  # Should be 1

# Test 6: Redirection
./target/release/rush -c \"echo test > /tmp/rush_test.txt\"
cat /tmp/rush_test.txt
\`\`\`

## Files to Create
- tests/integration/login_shell_test.sh
- tests/integration/non_tty_test.sh
- tests/integration/signal_test.sh
- tests/fixtures/*.sh (test scripts)

## Testing
- Run full integration test suite
- Verify all tests pass
- Add to CI pipeline

## Estimated Effort: 2 days" \
  --priority=2 \
  --labels="rush,stability,p1,testing,integration" \
  --silent)

echo "Story 13 created: $STORY_13"

# Story 14: Real-World Stability Testing
echo "Creating Story 14: Real-World Stability Testing..."
STORY_14=$(bd create \
  --parent="$EPIC_ID" \
  --title="STABILITY-014: Real-World Stability Testing (1 Week Trial)" \
  --description="As the rush developer, I need to use rush as my login shell for 1 week so that I can identify and fix real-world issues.

## Priority: P1 - HIGH
Final verification step.

## Acceptance Criteria
- [ ] Set rush as login shell on development machine
- [ ] Use for all daily work for 1 week
- [ ] Document all issues encountered
- [ ] Fix critical issues immediately
- [ ] Create beads for nice-to-have improvements
- [ ] No crashes during 1 week trial
- [ ] No data loss during 1 week trial
- [ ] Performance acceptable for daily use

## Testing Checklist
- [ ] Day 1: Basic usage (cd, ls, cat, git)
- [ ] Day 2: Complex pipelines and scripts
- [ ] Day 3: Background jobs and job control
- [ ] Day 4: Heavy I/O operations
- [ ] Day 5: Integration with other tools (git, cargo, npm)
- [ ] Day 6: Stress testing (many commands, long sessions)
- [ ] Day 7: Final verification and documentation

## Rollback Plan
Keep backup shell configured:
\`\`\`bash
echo \$SHELL > ~/.backup_shell
# Emergency: chsh -s /bin/zsh
\`\`\`

## Issues to Watch For
- Crashes or panics
- Orphaned processes
- Memory leaks
- Slow performance
- Terminal corruption
- Signal handling issues
- History corruption

## Estimated Effort: 5 days (1 week of use + fixes)" \
  --priority=2 \
  --labels="rush,stability,p1,testing,real-world" \
  --silent)

echo "Story 14 created: $STORY_14"

echo ""
echo "✅ Rush Shell Stability epic and all beads created successfully!"
echo ""
echo "Epic: $EPIC_ID (Rush Shell Stability - Login Shell Ready)"
echo ""
echo "Week 1 - Critical Blockers:"
echo "  - $STORY_1 (STABILITY-001: Non-TTY Mode Support) [P0]"
echo "  - $STORY_2 (STABILITY-002: Signal Handling) [P0]"
echo "  - $STORY_3 (STABILITY-003: File Redirection) [P0]"
echo "  - $STORY_4 (STABILITY-004: Subshell Support) [P0]"
echo "  - $STORY_5 (STABILITY-005: Exit Code Propagation) [P0]"
echo ""
echo "Week 2 - Core Features:"
echo "  - $STORY_6 (STABILITY-006: Variable Expansion) [P1]"
echo "  - $STORY_7 (STABILITY-007: Wildcard Expansion) [P1]"
echo "  - $STORY_8 (STABILITY-008: Command Substitution) [P1]"
echo ""
echo "Week 3 - Job Control & Polish:"
echo "  - $STORY_9 (STABILITY-009: Job Control) [P2]"
echo "  - $STORY_10 (STABILITY-010: Error Recovery) [P1]"
echo "  - $STORY_11 (STABILITY-011: Login Shell Init) [P2]"
echo "  - $STORY_12 (STABILITY-012: Shell Options) [P2]"
echo ""
echo "Week 4 - Testing:"
echo "  - $STORY_13 (STABILITY-013: Integration Tests) [P1]"
echo "  - $STORY_14 (STABILITY-014: Real-World Testing) [P1]"
echo ""
echo "Total: 14 stories across 4 weeks"
echo ""
echo "To start working on this epic, run:"
echo "  bd list --parent $EPIC_ID"
echo ""
echo "To start with the most critical issue:"
echo "  bd show $STORY_1"
echo ""
echo "Reference documentation:"
echo "  docs/stability-roadmap.md"
echo ""
