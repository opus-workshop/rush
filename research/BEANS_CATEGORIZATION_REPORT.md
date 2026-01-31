# Rush Bean Categorization Report

## Overview
Extracted and categorized **203 beans** from the Rush project, organized by verification pattern type.

**Source:** `bn ready` output + `bn list --all` child beans extraction

## Priority Distribution
- **P0 (Critical):** 3 beans
- **P1 (High):** 74 beans  
- **P2 (Medium):** 86 beans
- **P3 (Low):** 35 beans
- **P4 (Backlog):** 5 beans

**Total:** 203 beans

## Pattern Distribution

### Pattern 1: Cargo Checks (47 beans)
**Universal verification for Rust projects**
```bash
cargo build --release && cargo test && cargo clippy -- -D warnings
```
**Keywords:** cargo, build, test, clippy, lint checks, compilation  
**Use Case:** General Rust code changes, refactoring, performance tuning  
**Examples:** `Rush HN Readiness Cleanup`, `Reduce baseline command execution overhead`

### Pattern 2: Functional Tests (103 beans)
**For feature implementation, scripting, builtins, and shell functionality**
```bash
cargo build --release && ./tests/functional_tests.sh && cargo test
# Variants:
echo "test" | ./target/release/rush && cargo test  # Script execution
echo "echo test > /tmp/out" | ./target/release/rush && cargo test  # Redirections
echo "cat | grep | sort" | ./target/release/rush && cargo test  # Pipelines
```
**Keywords:** script, execution, shell, command, feature, builtin, pipes, redirects, loops, flow, here-documents, case, while, until, for, function  
**Use Case:** Builtin implementation, scripting features, control flow  
**Examples:** `SCRIPT-001: read builtin`, `POSIX-017: Implement while loop`, `FOUNDATION-005a: if/then/elif/else/fi syntax`

### Pattern 3: File Checks (14 beans)
**For documentation, license, and project structure**
```bash
# License files
test -f LICENSE-MIT && test -f LICENSE-APACHE

# Documentation
test -f README.md && test -f CONTRIBUTING.md && test -f CHANGELOG.md

# .gitignore
test -f .gitignore && grep -q "target" .gitignore

# CI/CD & Distribution
test -f .github/workflows/release.yml && ./target/release/rush --version
```
**Keywords:** file, license, create, document, README, CONTRIBUTING, changelog, gitignore, metadata, badges, formula, binary  
**Use Case:** Project setup, file creation, documentation generation  
**Examples:** `Add LICENSE files`, `Add CONTRIBUTING.md and CHANGELOG.md`, `Homebrew formula and installation testing`

### Pattern 4: Performance (12 beans)
**For benchmarking, profiling, and startup optimization**
```bash
# Startup comparison
hyperfine './target/release/rush -c echo' 'bash -c echo'

# Benchmark suite
cargo build --release && ./target/release/rush --benchmark

# Timing tests
cargo build --release && ./target/release/rush --time -c "sleep 0.1"
```
**Keywords:** performance, optimization, fast, startup, overhead, benchmark, timing, profil  
**Use Case:** Performance improvements, benchmark validation  
**Examples:** `Custom Allocator (mimalloc) for Faster Startup`, `Benchmark Reproducibility Suite`, `Profiling timing infrastructure`

### Pattern 5: JSON Output (8 beans)
**For AI integration, structured output, and tooling**
```bash
cargo build --release && ./target/release/rush -c 'git_log --json -n 10' | jq '.'
cargo build --release && ./target/release/rush -c 'git_diff --json' | jq '.'
cargo build --release && ./target/release/rush -c 'git_status --json' | jq '.'
cargo build --release && ./target/release/rush -c 'http_fetch --json https://...' | jq '.'
```
**Keywords:** json, git_log, git_diff, git_status, structured, ai, jq, http, fetch  
**Use Case:** AI agent integration, JSON-based git commands, API responses  
**Examples:** `AI-001: Structured git log with JSON output`, `AI-002: Structured git diff with JSON output`

### Pattern 6: Stability (14 beans)
**For signal handling, TTY modes, error recovery**
```bash
# Signal handling
timeout 1 ./target/release/rush -c "sleep 10" || test $? -eq 130 && cargo test

# TTY/Non-TTY mode
echo "test" | ./target/release/rush && cargo test

# Integration tests
cargo test stability_ && ./tests/integration_tests.sh
```
**Keywords:** stability, signal, tty, non-tty, recovery, error, integration, signal handling, exit code, panic  
**Use Case:** Error handling, signal processing, shell stability  
**Examples:** `STABILITY-002: Signal Handling`, `STABILITY-001: Non-TTY Mode Support`, `Audit and fix critical production panics`

### Pattern 7: Epic Reference (5 beans)
**For epic-level features requiring full test suite**
```bash
cargo test
```
**Keywords:** Critical features, full system, comprehensive  
**Use Case:** Epic tracking, master features  
**Examples:** `Rush 1.0 Critical Features`, `AI Agent Batteries Included`, `POSIX Compliance - Full Unix Shell Support`

## Key Statistics

| Pattern | Count | % of Total |
|---------|-------|-----------|
| Pattern_2 (Functional) | 103 | 50.7% |
| Pattern_1 (Cargo) | 47 | 23.2% |
| Pattern_3 (Files) | 14 | 6.9% |
| Pattern_6 (Stability) | 14 | 6.9% |
| Pattern_4 (Performance) | 12 | 5.9% |
| Pattern_5 (JSON) | 8 | 3.9% |
| Pattern_7 (Epic) | 5 | 2.5% |

## Usage

The categorization is saved in CSV format at:
```
/Users/asher/tt/rush/research/all_beans_verification_map.csv
```

### CSV Columns
- **Bean_ID:** Unique identifier (numeric or with decimal for child beans)
- **Title:** Bean description
- **Priority:** P0-P4 priority level
- **Pattern_Number:** Verification pattern (Pattern_1 through Pattern_7)
- **Suggested_Verify_Command:** Recommended verification command for the pattern

### Integration with Verification Pipeline
Each pattern maps to a standard verification approach:
1. **Pattern_1:** Always uses `cargo build --release && cargo test && cargo clippy`
2. **Pattern_2:** Uses functional test variants based on keyword matching
3. **Pattern_3:** Uses file existence and content checks
4. **Pattern_4:** Uses hyperfine/cargo bench for performance comparison
5. **Pattern_5:** Uses JSON parsing with `jq` validation
6. **Pattern_6:** Uses signal, TTY, and integration test variants
7. **Pattern_7:** Uses full `cargo test` suite

## Notes
- Child beans (e.g., 15.1, 22.2, 28.4) extracted from `bn list --all` output
- Pattern classification based on keyword analysis of bean titles
- Verify commands are templated and may need adaptation based on specific test infrastructure
- P0 beans are critical path for release
- P1 beans should be completed before P2/P3 for feature completeness
