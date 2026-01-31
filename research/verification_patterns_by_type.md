# Rush Beans: Verification Patterns by Type

## Quick Reference: What Verify Command to Run by Bean Type

### Pattern 1: Cargo-Based Checks (Most Common)
**Used by**: All 20 beans (baseline)

```bash
# Universal base verification
cargo build --release
cargo test
cargo clippy -- -D warnings
```

**Why**: Rust projects require compilation, tests, and linting to be correct.

**Beans**: 141, 143, 21, 22.1, 23, 32, 34, 35, 43, 49, 50, and components of all others.

---

### Pattern 2: Functional/Behavioral Tests
**Used by**: Beans 18, 19, 21, 22.1, 23, 43, 50 (7 beans)

**Test Categories**:
1. **Shell syntax verification**
   - Variable assignment: `FOO=bar; echo $FOO`
   - Globs: `for x in *.rs; do echo $x; done`
   - Control flow: `if [ -f file ]; then echo yes; fi`
   - Arithmetic: `echo $((1+2))`

2. **Pipeline/redirection verification**
   - Multi-stage pipes: `cat file | grep pattern | sort | uniq`
   - Input redirect: `command < input.txt`
   - Output redirect: `command > output.txt`
   - Append: `command >> output.txt`
   - Stderr: `command 2> errors.log`
   - Combined: `command 2>&1 | grep error`

3. **Script execution**
   - File execution: `rush script.rush`
   - Shebang support: `#!/usr/bin/env rush`
   - Arguments: `$1`, `$2`, `$@`
   - Exit codes: `echo $?`

```bash
# Verification approach: Manual shell tests or smoke test suite
./tests/smoke_test.sh                          # Run all tests
echo "test" | ./target/release/rush            # Pipe test
echo 'echo $1' | ./target/release/rush arg1    # Args test
./target/release/rush < script.sh              # Redirect test
./target/release/rush -c 'FOO=bar; echo $FOO'  # Inline test
```

**Expected Result**: Exit code 0, correct output, no orphaned processes.

---

### Pattern 3: File Existence & Structure Checks
**Used by**: Beans 58, 4 (2 beans, plus documentation in others)

**Check Types**:
1. **License files**
   ```bash
   test -f LICENSE-MIT
   test -f LICENSE-APACHE
   grep -q "MIT License" LICENSE-MIT
   grep -q "Apache License" LICENSE-APACHE
   ```

2. **Documentation files**
   ```bash
   test -f README.md
   test -f CONTRIBUTING.md
   test -f CHANGELOG.md
   grep -q "badge" README.md  # Verify badges added
   ```

3. **Configuration files**
   ```bash
   test -f .gitignore
   test -f Cargo.toml
   grep -q "license =" Cargo.toml
   ```

4. **GitHub recognition** (manual or webhook)
   - Visit repo page → see license displayed
   - Or use GitHub API: `curl https://api.github.com/repos/user/rush`

```bash
# Verification command
test -f LICENSE-MIT && \
test -f LICENSE-APACHE && \
test -f README.md && \
test -f CONTRIBUTING.md && \
grep -q "MIT OR Apache-2.0" Cargo.toml
```

---

### Pattern 4: Performance Benchmarking
**Used by**: Beans 141, 5, 12, 13 (4 beans)

**Measurements**:
1. **Startup time** (using hyperfine)
   ```bash
   hyperfine './target/release/rush -c "echo test"' \
             'bash -c "echo test"' \
             --prepare 'sync'  # Clear caches
   ```
   - Expected improvement: 1-3ms faster

2. **Binary size**
   ```bash
   ls -lh target/release/rush
   du -h target/release/rush
   # Should be same or smaller after changes
   ```

3. **Memory usage**
   ```bash
   /usr/bin/time -v ./target/release/rush -c "ls"
   # Look for "Maximum resident set size"
   ```

4. **Pipeline performance** (large data)
   ```bash
   seq 1 1000000 | ./target/release/rush -c 'sort | uniq -c | wc -l'
   # Measure time with: time command
   ```

```bash
# Complete performance verification
echo "Before changes:"
hyperfine './target/release/rush-old -c "pwd"' --prepare 'sync'

# After making changes:
cargo build --release
echo "After changes:"
hyperfine './target/release/rush -c "pwd"' --prepare 'sync'

echo "Binary size:"
ls -lh target/release/rush
```

**Expected Result**: Show measurable improvement in timing or size.

---

### Pattern 5: JSON Structured Output (AI-Agent Focused)
**Used by**: Beans 32, 34, 35 (3 beans)

**Verification Steps**:
1. **JSON validity**
   ```bash
   ./target/release/rush -c 'git_log --json' | jq . >/dev/null
   # Success = valid JSON, failure = jq error
   ```

2. **Required fields present**
   ```bash
   ./target/release/rush -c 'git_log --json -n 1' | \
     jq -e '.[] | select(.hash and .author and .date and .message)' >/dev/null
   ```

3. **Performance check** (use `time`)
   ```bash
   time ./target/release/rush -c 'git_log --json -n 100'
   # Should complete in <5ms
   ```

4. **Error handling**
   ```bash
   cd /tmp/not-a-repo
   ./target/release/rush -c 'git_log --json' 2>&1 | grep -q "not a git repository"
   ```

5. **Edge cases**
   ```bash
   # Binary files
   echo -e '\x00\x01\x02' > /tmp/binary.bin
   ./target/release/rush -c 'git_diff --json' | jq '.files[] | select(.path == "binary.bin")'

   # Renames
   git mv oldname.rs newname.rs
   ./target/release/rush -c 'git_diff --json --staged' | jq '.files[].status'
   ```

```bash
# Complete JSON verification
echo "Testing JSON output..."
./target/release/rush -c 'git_status --json' | jq . >/dev/null

echo "Testing required fields..."
./target/release/rush -c 'git_log --json -n 5' | \
  jq '.[] | {hash: .hash, author: .author, date: .date}'

echo "Performance check (should be <5ms)..."
time ./target/release/rush -c 'git_log --json -n 100' >/dev/null

echo "Testing error handling..."
cd /tmp
./target/release/rush -c 'git_status --json' 2>&1 | head -1

echo "✓ JSON verification complete"
```

---

### Pattern 6: Stability & Signal Handling
**Used by**: Beans 43, 49, 50 (3 beans)

**Verification Steps**:
1. **Non-TTY mode**
   ```bash
   # Test 1: Pipe input
   echo "echo test" | ./target/release/rush
   # Expected: prints "test"

   # Test 2: File redirect
   echo "pwd" | ./target/release/rush
   # Expected: prints current directory

   # Test 3: Command substitution
   result=$(./target/release/rush -c "echo hello")
   # Expected: result="hello"
   ```

2. **Signal handling**
   ```bash
   # Test Ctrl-C (SIGINT)
   timeout 1 ./target/release/rush -c 'sleep 10' || \
     test $? -eq 130  # 130 = 128 + 2 (SIGINT)

   # Verify no orphaned processes
   ps aux | grep -v grep | grep rush || echo "✓ No orphaned processes"
   ```

3. **Exit codes**
   ```bash
   ./target/release/rush -c 'exit 42'
   test $? -eq 42  # Should exit with correct code
   ```

4. **File redirection**
   ```bash
   echo 'echo "stdout test" > /tmp/out.txt' | ./target/release/rush
   test -f /tmp/out.txt && grep -q "stdout test" /tmp/out.txt

   echo 'echo "error" >&2' | ./target/release/rush 2>/tmp/err.txt
   grep -q "error" /tmp/err.txt
   ```

```bash
# Complete stability verification
echo "Testing non-TTY mode..."
echo "echo test" | ./target/release/rush

echo "Testing pipe input..."
echo "pwd" | ./target/release/rush

echo "Testing signal handling..."
timeout 1 ./target/release/rush -c 'sleep 10' || true
sleep 1 && ps aux | grep rush | grep -v grep || echo "✓ No orphaned processes"

echo "Testing exit codes..."
./target/release/rush -c 'exit 0' && echo "✓ Exit 0 works"
./target/release/rush -c 'exit 42'; test $? -eq 42 && echo "✓ Exit 42 works"

echo "Testing file redirections..."
echo 'echo "test" > /tmp/out.txt' | ./target/release/rush
test -f /tmp/out.txt && echo "✓ Output redirection works"

echo "✓ Stability verification complete"
```

---

### Pattern 7: Epic/Reference Beans (Large Scope)
**Used by**: Beans 2, 6, 19 (3 beans - not directly verifiable, but composed of sub-beans)

These are **epics** that decompose into multiple smaller beans with their own acceptance criteria.

```bash
# Verification approach: Verify all child beans first
bn show 2  # Check what sub-beans exist
# Then verify each child bean using appropriate patterns above

# Example for Epic #2 (AI Agent Batteries Included):
# - Verify AI-001 (bean 32): git_log --json works
# - Verify AI-002 (bean 34): git_diff --json works
# - Verify AI-003 (bean 35): git_status --json works
# - When all children pass, epic passes
```

---

## Universal Verification Checklist

Use this for ANY bean:

```bash
#!/bin/bash
set -e

echo "=== Bean Verification Checklist ==="

# 1. Compile
echo "1. Building release binary..."
cargo build --release
echo "   ✓ Build succeeded"

# 2. Test
echo "2. Running tests..."
cargo test
echo "   ✓ Tests passed"

# 3. Code quality
echo "3. Checking code quality..."
cargo clippy -- -D warnings
echo "   ✓ No clippy warnings"

# 4. Feature-specific verification (from bean description)
echo "4. Feature-specific verification..."
case "$BEAN_ID" in
  18|21|23|43|50)
    echo "   Running functional tests..."
    ./tests/smoke_test.sh
    ;;
  32|34|35)
    echo "   Testing JSON output..."
    ./target/release/rush -c 'git_status --json' | jq .
    ;;
  141)
    echo "   Checking allocator..."
    ./target/release/rush -c 'echo test'
    ;;
  *)
    echo "   (No specific tests for this bean type)"
    ;;
esac
echo "   ✓ Feature tests passed"

echo ""
echo "✓✓✓ Bean verification complete ✓✓✓"
```

---

## Performance Baseline (for reference)

Current Rush performance characteristics (as of latest commits):
- Startup: ~4.5ms (target: <2ms with optimizations)
- Direct execution vs daemon: 4.5ms vs 9.3ms
- Binary size: ~5.5MB
- Smoke test: 71/120 passing (59%) → target 120/120 (100%)

Use these as benchmarks when verifying performance beans.
