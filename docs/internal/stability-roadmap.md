# Rush Stability Roadmap
**Goal:** Make rush ready to be a default login shell on macOS/Linux

## Critical Blockers (Must Fix)

### 1. Non-TTY Mode Support ⚠️ **BLOCKING**
**Problem:** Reedline requires TTY, crashes on non-interactive use
**Impact:** Login scripts, cron jobs, command substitution all fail

**Solution:**
```rust
// Detect TTY and fallback to simple readline
if atty::is(atty::Stream::Stdin) {
    // Use reedline for interactive mode
    run_interactive_with_reedline()
} else {
    // Use simple stdin reader for non-interactive
    run_non_interactive()
}
```

**Files to modify:**
- `src/main.rs:188` - Split `run_interactive()` into TTY/non-TTY paths
- Add `atty` crate dependency

**Test cases:**
```bash
echo "pwd" | rush              # Should work
rush -c "echo test"            # Already works
rush < script.sh               # Should work
$(rush -c "echo nested")       # Should work
```

---

### 2. Signal Handling
**Problem:** No proper SIGINT, SIGTERM, SIGHUP handling
**Impact:** Orphaned processes, corrupted state on Ctrl-C

**Solution:**
```rust
use signal_hook::{consts::SIGINT, iterator::Signals};

// Set up signal handlers
let mut signals = Signals::new(&[SIGINT, SIGTERM, SIGHUP])?;

// In executor, check for signals
if signal_received {
    cleanup_child_processes();
    exit(130); // Standard shell exit code for SIGINT
}
```

**Files to modify:**
- `src/executor/mod.rs` - Add signal handling to executor
- `src/main.rs` - Register signal handlers on startup

**Test cases:**
- Ctrl-C during long-running command
- Kill rush process during pipeline
- Logout with background jobs (should clean up)

---

### 3. Exit Code Propagation
**Problem:** Exit codes not properly propagated through pipelines
**Impact:** Scripts using `set -e` or `||` operators fail

**Current state:** Basic support exists (`result.exit_code`)
**Needed:**
- Pipeline exit codes (last command or first failure)
- `$?` variable support
- `set -e` equivalent (exit on error)

**Files to modify:**
- `src/executor/pipeline.rs` - Track exit codes through pipeline
- `src/runtime/mod.rs` - Add `$?` special variable

---

### 4. Subshell Support
**Problem:** No support for `( )` subshells
**Impact:** Many scripts use subshells to isolate environment changes

**Solution:**
```rust
// In parser, detect ( ) blocks
Statement::Subshell(statements) => {
    let mut child_runtime = runtime.clone();
    execute_in_isolated_runtime(statements, &mut child_runtime)
}
```

**Files to modify:**
- `src/parser/ast.rs` - Add `Subshell` variant to AST
- `src/lexer/mod.rs` - Tokenize `(` and `)`
- `src/executor/mod.rs` - Execute with isolated runtime

---

### 5. Redirection Support
**Problem:** No file descriptor redirection
**Impact:** Scripts using `>`, `>>`, `<`, `2>&1` fail

**Current state:** Basic pipe support exists
**Needed:**
- `>` stdout redirect
- `>>` append redirect
- `<` stdin redirect
- `2>` stderr redirect
- `2>&1` stderr to stdout
- `&>` both to file

**Files to modify:**
- `src/parser/ast.rs` - Add `Redirect` to command AST
- `src/executor/mod.rs` - Set up file descriptors before exec

---

## Important Features (Should Have)

### 6. Job Control
**Problem:** No background jobs, no `&`, `fg`, `bg`, `jobs`
**Impact:** Can't background long tasks, poor UX

**Solution:**
- Track background processes in runtime
- Implement `&` operator for background execution
- Add `jobs`, `fg`, `bg` builtins
- Handle SIGCHLD for job status

**Complexity:** Medium (2-3 days)

---

### 7. Command Substitution
**Problem:** `$()` and `` `cmd` `` not fully implemented
**Impact:** Scripts using nested commands fail

**Current state:** Parser has `Command` in AST
**Needed:** Execute and capture output

**Files to modify:**
- `src/executor/mod.rs` - Execute command and capture stdout
- `src/runtime/mod.rs` - Substitute into parent command

---

### 8. Proper Variable Expansion
**Problem:** Limited `$VAR` expansion (no `${VAR}`, `${VAR:-default}`, etc.)
**Impact:** POSIX scripts fail

**Needed:**
- `${VAR}` - Braced variables
- `${VAR:-default}` - Default values
- `${VAR:=default}` - Assign default
- `${VAR:?error}` - Error if unset
- `${VAR#pattern}` - Remove prefix
- `${VAR%pattern}` - Remove suffix

---

### 9. Wildcard Expansion
**Problem:** No `*`, `?`, `[...]` glob expansion
**Impact:** `ls *.rs` doesn't work as expected

**Solution:**
- Use `glob` crate
- Expand before command execution
- Handle dotfiles correctly

---

### 10. Error Recovery
**Problem:** Parse errors can crash shell
**Impact:** Bad login shell experience

**Solution:**
- Try/catch around parse/execute
- Return to prompt on error
- Log errors without exiting
- Crash recovery state save

---

## Nice-to-Haves (Polish)

### 11. Login Shell Initialization
- Source `~/.rush_profile` on login
- Source `~/.rushrc` on interactive start
- Set standard env vars (`$SHELL`, `$TERM`, etc.)

### 12. POSIX Compatibility Mode
- `rush --posix` flag
- Fallback to bash/zsh for unsupported features
- Warn on non-POSIX syntax

### 13. Shell Options
- `set -e` - Exit on error
- `set -u` - Error on undefined variables
- `set -x` - Print commands before execution
- `set -o pipefail` - Pipeline fails if any command fails

### 14. Alias Support
- Define command aliases
- Persist in config file
- Expansion before execution

---

## Testing Strategy

### Integration Tests
```bash
# tests/stability/login_shell_test.sh
#!/bin/bash

# Test 1: Non-interactive mode
echo "pwd" | rush

# Test 2: Script execution
rush test_script.sh

# Test 3: Command substitution
rush -c "echo $(rush -c 'echo nested')"

# Test 4: Signal handling
rush -c "sleep 100" &
kill $!

# Test 5: Exit codes
rush -c "false" ; echo $?  # Should be 1

# Test 6: Redirection
rush -c "echo test > /tmp/rush_test.txt"
```

---

## Implementation Priority

**Week 1: Critical Path**
1. Non-TTY mode (1 day)
2. Signal handling (1 day)
3. Exit code propagation (1 day)
4. Redirection support (2 days)

**Week 2: Core Features**
5. Subshells (2 days)
6. Variable expansion (2 days)
7. Wildcard expansion (1 day)

**Week 3: Job Control & Polish**
8. Job control (3 days)
9. Command substitution (1 day)
10. Error recovery (1 day)

**Week 4: Stability & Testing**
11. Integration testing (2 days)
12. Login shell init (1 day)
13. Shell options (2 days)

---

## Success Criteria

Rush is ready as a default shell when:

- ✅ All tests pass (including new stability tests)
- ✅ Can source standard shell configs (`~/.profile`, `~/.bashrc` equivalents)
- ✅ Can run common system scripts without errors
- ✅ Handles signals gracefully (no orphaned processes)
- ✅ Works in both TTY and non-TTY modes
- ✅ Exit codes propagate correctly
- ✅ No crashes during normal operation
- ✅ Can survive and recover from parse errors
- ✅ Job control works (background jobs don't block)
- ✅ Used successfully as login shell for 1 week without issues

---

## Risk Assessment

**High Risk:**
- Non-TTY mode (architectural change to reedline usage)
- Job control (complex signal handling)

**Medium Risk:**
- Redirection (file descriptor juggling)
- Subshells (runtime cloning)

**Low Risk:**
- Variable expansion (parsing only)
- Wildcard expansion (existing crate)

---

## Rollback Plan

If rush fails as login shell:

1. **Keep backup shell configured:**
   ```bash
   # Before switching to rush
   echo $SHELL > ~/.backup_shell
   ```

2. **Emergency recovery:**
   - Boot to recovery mode
   - Mount drive and edit `/etc/passwd`
   - Or: `chsh -s /bin/zsh` from another TTY (Cmd+Opt+F1)

3. **Test in tmux first:**
   ```bash
   # Run rush inside tmux for safety
   tmux
   rush
   ```

---

**Next Step:** Fix the non-TTY blocker first, as it's the most critical for login shell use.
