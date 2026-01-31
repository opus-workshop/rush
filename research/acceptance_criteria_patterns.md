# Rush Project: Acceptance Criteria Patterns (First 20 Beans)

## Executive Summary
Analyzed 20 beans from the Rush shell project to identify acceptance criteria patterns. Found 4 distinct verification patterns that correlate with bean types: **Cargo-based checks**, **File/CLI behavior tests**, **Performance benchmarks**, and **Git/integration tests**.

---

## Pattern 1: Cargo Build & Test Checks (Most Common)

Used in: **15+ beans** (fundamental to all development)

### Acceptance Criteria Template
```
- [ ] cargo build --release succeeds
- [ ] cargo test passes (all tests including [specific tests])
- [ ] cargo clippy -- -D warnings passes
```

### Beans Using This Pattern
| Bean ID | Title | Verify Command |
|---------|-------|---|
| 141 | Custom Allocator (mimalloc) | `cargo test && cargo build --release && hyperfine before/after` |
| 143 | Remove Unused Tokio Dependency | `cargo build --release && cargo test` |
| 21 | US-003: Robust Pipeline Implementation | `cargo test && cargo build --release && cargo clippy -- -D warnings` |
| 22.1 | Basic redirections: >, >>, 2>, < | `cargo test && cargo build --release` |
| 23 | US-001: Script Execution | `cargo test && cargo build --release` |
| 32 | AI-001: Structured git log | `cargo test && cargo build --release && cargo clippy` |
| 34 | AI-002: Structured git diff | `cargo test && cargo build --release && cargo clippy` |
| 35 | AI-003: Extend git_status | `cargo test && cargo build --release && cargo clippy` |
| 43 | STABILITY-001: Non-TTY Mode | `cargo test && cargo build --release && cargo clippy` |
| 49 | STABILITY-002: Signal Handling | `cargo test && cargo build --release && cargo clippy` |
| 50 | STABILITY-003: File Redirection | `cargo test && cargo build --release && cargo clippy` |

**Observation**: Nearly every bean includes `cargo test && cargo build --release` as the base verification. This is the Rust project standard.

---

## Pattern 2: Behavioral/Functional Tests (Critical Feature Beans)

Used in: **8+ beans** (shells require functional verification)

### Acceptance Criteria Template
- [ ] Feature X works: `command --flag` produces expected result
- [ ] Works with pipes/redirects/subshells
- [ ] Error messages are helpful
- [ ] Handles edge cases correctly

### Key Examples

**Bean 21 (Robust Pipeline Implementation)**
```
- [ ] Multi-stage pipes work: cat file | grep pattern | sort | uniq -c
- [ ] Proper error propagation through pipeline
- [ ] Pipeline exit code is last command's exit code
- [ ] SIGPIPE handling (broken pipe errors)
- [ ] Handles large data streams without blocking
- [ ] Works with builtins and external commands
- [ ] Manual testing: complex pipelines with 5+ stages
```

**Verify Command**: `./tests/smoke_test.sh` or manual shell tests

**Bean 23 (Script Execution)**
```
- [ ] Can execute .rush files: rush script.rush
- [ ] Shebang support: #!/usr/bin/env rush
- [ ] Script arguments accessible via $1, $2, etc.
- [ ] Exit codes propagate correctly
- [ ] Error messages show script name and line number
- [ ] Scripts can source other scripts
- [ ] Manual testing: run sample scripts
```

**Verify Command**:
```bash
echo '#!/usr/bin/env rush' > test.rush
chmod +x test.rush
./test.rush arg1 arg2
```

**Bean 18 (Shell Foundation - Make Rush Actually Work)**
```
- [ ] Smoke test passes 120/120 tests (100%)
- [ ] Can run: FOO=bar; echo $FOO
- [ ] Can run: for x in *.rs; do echo $x; done
- [ ] Can run: if [ -f README.md ]; then cat README.md; fi
- [ ] Can use rush as daily driver shell
```

**Verify Command**: `./tests/smoke_test.sh` (must pass all 120 tests)

---

## Pattern 3: File Existence & Structure Checks

Used in: **5+ beans** (documentation, licenses, configs)

### Acceptance Criteria Template
```
- [ ] FILE_A exists with correct CONTENT
- [ ] FILE_B exists with correct CONTENT
- [ ] GitHub/tool recognizes the change
```

### Key Examples

**Bean 58 (Add LICENSE files)**
```
- [ ] LICENSE-MIT exists with correct MIT text
- [ ] LICENSE-APACHE exists with correct Apache 2.0 text
- [ ] GitHub recognizes the license on the repo page
```

**Verify Command**:
```bash
test -f LICENSE-MIT && test -f LICENSE-APACHE && \
  grep -q "MIT License" LICENSE-MIT && \
  grep -q "Apache License" LICENSE-APACHE
```

**Bean 4 (Rush HN Readiness Cleanup)**
Requires file existence checks:
- `.gitignore` properly configured
- `README.md` with badges
- `CONTRIBUTING.md` exists
- `CHANGELOG.md` exists
- `Cargo.toml` metadata complete

---

## Pattern 4: Performance Benchmarking (Optimization Beans)

Used in: **3+ beans** (performance-critical work)

### Acceptance Criteria Template
```
- [ ] Startup time measurably improved (hyperfine before/after)
- [ ] Binary size same or smaller
- [ ] Tests still passing
- [ ] No regression in functionality
```

### Key Examples

**Bean 141 (Custom Allocator - mimalloc)**
```
- [ ] mimalloc added as global allocator
- [ ] panic = "abort" set in release profile
- [ ] Startup time measurably improved (hyperfine before/after)
- [ ] Binary size same or smaller
```

**Verify Command**:
```bash
hyperfine './target/release/rush -c "echo test"' \
           'bash -c "echo test"' --prepare 'sync'
```

**Bean 5 (Daemon Performance Optimization)**
```
- [ ] Daemon execution < 5.5ms (improvement target)
- [ ] All tests passing
- [ ] Stdout/stderr capture still working
- [ ] Measure before/after each change
```

**Verify Command**:
```bash
hyperfine 'rush -c "pwd"' --prepare 'sync'
cargo test  # verify no regressions
```

**Bean 12 (Benchmark Reproducibility Suite)**
```
- [ ] user-friendly benchmark runner: rush --benchmark
- [ ] Quick mode for smoke testing: rush --benchmark quick
- [ ] Comparison mode: rush --benchmark compare (Rush vs bash/zsh)
- [ ] Results saved to benchmark_results.json
- [ ] Can export to markdown
```

**Verify Command**:
```bash
cargo build --release
./target/release/rush --benchmark
./target/release/rush --benchmark compare
```

---

## Pattern 5: JSON Structured Output (AI Agent Beans)

Used in: **3 beans** (git integration for AI agents)

### Acceptance Criteria Template
```
- [ ] COMMAND --json returns valid JSON
- [ ] JSON includes REQUIRED_FIELDS
- [ ] Performance: <Xms for typical operations
- [ ] Error handling with clear messages
- [ ] Handles edge cases (binary files, renames, etc.)
```

### Key Examples

**Bean 32 (AI-001: Structured git log with JSON)**
```
- [ ] git_log builtin command implemented
- [ ] Human-readable output by default
- [ ] git_log --json returns structured JSON array
- [ ] JSON includes: hash, author, date, message, files_changed
- [ ] git_log --json -n N limits to N commits
- [ ] Performance: <5ms for 100 commits on large repos
- [ ] Error handling: clear messages for non-git repos
```

**Verify Command**:
```bash
cargo test  # unit + integration tests
cargo build --release
time ./target/release/rush -c 'git_log --json -n 100' \
  | jq '.[] | {hash, author, date}'
```

**Bean 34 (AI-002: Structured git diff with JSON)**
```
- [ ] git_diff builtin command implemented
- [ ] git_diff --json returns structured JSON
- [ ] JSON includes: files, hunks, line changes, stats
- [ ] git_diff --json --staged for staged changes
- [ ] Performance: <10ms for typical diffs (<1000 lines)
- [ ] Handles binary files gracefully
```

**Verify Command**:
```bash
cargo build --release
./target/release/rush -c 'git_diff --json' | jq '.files[]'
./target/release/rush -c 'git_diff --json --staged' | jq '.summary'
```

**Bean 35 (AI-003: Extend git_status with --json)**
```
- [ ] git_status --json returns structured JSON
- [ ] Include all file categories: staged, unstaged, untracked
- [ ] Include ahead/behind counts and branch info
- [ ] Performance: <5ms on repos with <1000 files
```

**Verify Command**:
```bash
./target/release/rush -c 'git_status --json' | jq '.branch, .staged, .unstaged'
```

---

## Pattern 6: Stability/Signal Handling (System Integration)

Used in: **2 beans** (critical for shell usage)

### Acceptance Criteria Template
```
- [ ] Feature works in isolation
- [ ] No side effects or broken state on signal
- [ ] Cleanup happens properly
- [ ] Exit codes are correct
```

### Key Examples

**Bean 43 (Non-TTY Mode Support)**
```
- [ ] Detects TTY vs non-TTY mode
- [ ] Interactive mode: Uses reedline for line editing
- [ ] Non-interactive mode: Uses simple stdin reader
- [ ] echo "pwd" | rush works
- [ ] rush < script.sh works
- [ ] $(rush -c "echo nested") works
- [ ] Can be used in cron jobs
```

**Verify Command**:
```bash
echo "echo test" | ./target/release/rush  # pipe test
echo "pwd" | ./target/release/rush        # stdin redirect
echo 'echo $1' | ./target/release/rush arg1  # command substitution
```

**Bean 49 (Signal Handling)**
```
- [ ] SIGINT (Ctrl-C) handler implemented
- [ ] SIGTERM handler implemented
- [ ] Child processes cleaned up on signal
- [ ] Exit with correct signal exit code (130 for SIGINT)
- [ ] No orphaned processes after signal
```

**Verify Command**:
```bash
# Manual: start long-running command, press Ctrl-C
./target/release/rush -c 'sleep 10'  # then Ctrl-C
ps aux | grep rush  # verify no orphaned processes

# Or write specific tests
cargo test signal_handling
```

---

## Pattern 7: Documentation & Compliance (Reference Beans)

Used in: **2-3 beans** (epics without deep implementation criteria)

### Acceptance Criteria Template
```
- [ ] Code compiles and tests pass
- [ ] Feature/component documented
- [ ] Examples provided
- [ ] Audit/spec referenced
```

### Key Examples

**Bean 2 (AI Agent Batteries Included - EPIC)**
```
- [ ] All git commands return structured JSON (--json flag)
- [ ] Native JSON parsing and querying
- [ ] All file operations support --json output
- [ ] HTTP fetch builtin for API/docs access
- [ ] Structured error responses
- [ ] 10x faster than bash+jq for agent workflows
- [ ] Documentation with AI agent examples
```

**Verify Command**: Complex - requires multiple sub-beans verified first

**Bean 6 (POSIX Compliance - EPIC)**
```
- [ ] All required POSIX builtins implemented
- [ ] Positional parameters fully wired ($1-$9, $@, $*)
- [ ] Here-documents working (<<EOF)
- [ ] Control flow complete (while, until, case)
- [ ] POSIX test suite passing (target: 90%+)
```

**Verify Command**:
```bash
cargo test posix_  # run all POSIX-related tests
./tests/posix_compliance.sh  # if exists
```

**Bean 19 (Rush Scripting Support - EPIC)**
```
- [ ] All Tier 1 builtins implemented
- [ ] All Tier 2 builtins implemented
- [ ] All Tier 3 builtins implemented
- [ ] Can run basic shell scripts
- [ ] Can execute ~/.rushrc configuration
- [ ] Integration tests for scripting scenarios
- [ ] Documentation for all new builtins
```

**Verify Command**:
```bash
cargo test scripting_
./tests/scripting_tests.sh
```

---

## Verification Command Summary Table

| Pattern | Primary Verify Command | Fallback/Secondary |
|---------|----------------------|-------------------|
| **Cargo-based (15 beans)** | `cargo test && cargo build --release` | `cargo clippy -- -D warnings` |
| **Functional tests (8 beans)** | `./tests/smoke_test.sh` or manual shell tests | `./target/release/rush -c 'test command'` |
| **File checks (5 beans)** | `test -f FILE && grep PATTERN FILE` | File inspection/git webhook |
| **Performance (3 beans)** | `hyperfine 'new' 'old' --prepare sync` | Binary size comparison |
| **JSON output (3 beans)** | `rush -c 'command --json' \| jq .` | Schema validation, performance timing |
| **Stability (2 beans)** | `echo "cmd" \| rush` or manual signal test | Process inspection, exit code check |
| **Documentation (2 beans)** | `cargo test` + manual review | Spec/doc file existence |

---

## Key Insights

### 1. **Universal Base: Cargo Checks**
Every single bean includes `cargo test && cargo build --release`. This is non-negotiable.

### 2. **Shell-Specific: Functional Tests**
For shell features, behavioral tests matter more than just compilation:
- Use `./tests/smoke_test.sh` for coverage tests
- Manual shell tests for complex pipelines
- Exit code verification

### 3. **Performance is Measurable**
Optimization beans require before/after measurements:
- Use `hyperfine` for startup timing
- Use `cargo bloat` for binary size
- Document baseline and target

### 4. **AI-Native = JSON Structured Output**
Git integration beans require:
- JSON schema validation
- Performance targets (<5-10ms)
- Error handling specification

### 5. **Stability = Signal + TTY + Cleanup**
System integration beans need:
- Signal handler verification (no orphans)
- TTY detection tests
- Proper cleanup on errors

### 6. **Epic Beans Reference Sub-Beans**
Complex epics (beans 2, 6, 19) decompose into smaller, verifiable sub-beans with concrete criteria.

---

## Recommended Verify Command Template

For a generic bean verification script:

```bash
#!/bin/bash
# verify.sh - Universal Rush bean verification

set -e

# 1. Cargo checks (universal)
echo "Building..."
cargo build --release

echo "Running tests..."
cargo test

echo "Checking code quality..."
cargo clippy -- -D warnings

# 2. Feature-specific verification (bean type dependent)
case "$BEAN_TYPE" in
  "performance")
    echo "Benchmarking..."
    hyperfine './target/release/rush -c "test"' >/dev/null
    ;;
  "functional")
    echo "Running functional tests..."
    ./tests/smoke_test.sh
    ;;
  "json-output")
    echo "Validating JSON output..."
    ./target/release/rush -c 'git_status --json' | jq . >/dev/null
    ;;
  "stability")
    echo "Testing signal handling..."
    timeout 1 ./target/release/rush -c 'sleep 10' || true
    ;;
esac

echo "✓ Bean verification complete"
```

---

## File Locations

- **Beans**: Use `bn ready` to list
- **Acceptance criteria**: Each bean stored in bean system
- **Verification scripts**: `/tests/` directory (smoke_test.sh is primary)
- **Benchmark scripts**: `/scripts/` directory
- **Documentation**: `/docs/` directory
