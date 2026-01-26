# Benchmarking Claude Code in Rush vs Zsh

This guide explains how to benchmark Claude Code and other tools running in Rush vs Zsh to measure performance differences.

## Quick Start

### Option 1: Quick Benchmark (Bash)
```bash
# Fast comparison of basic commands
./benches/quick_benchmark.sh
```

**Output:**
- Shell startup time comparison
- Simple command execution times
- Pipes and redirects performance
- Command substitution speed
- Overall speedup comparison

### Option 2: Comprehensive Benchmark (Python)
```bash
# Detailed statistics with JSON output
python3 ./benches/claude_code_benchmark.py
```

**Output:**
- Detailed statistics (mean, median, stddev)
- Multiple categories:
  - Shell startup
  - Command execution
  - File operations
  - Git operations
  - Environment variables
- JSON results file: `benchmark_results_claude_code.json`
- Performance comparison report

## What Gets Benchmarked

### 1. Shell Startup Time
- Time to start shell and exit
- Time to run simple echo command
- **Why it matters:** Affects every command you run, including claude-code startup

### 2. Command Execution
- `pwd` - Current directory
- `ls -la` - List files
- `echo $HOME` - Variable expansion
- `echo 'test' | cat` - Pipes
- `echo $(pwd)` - Command substitution
- **Why it matters:** Your daily workflow commands

### 3. File Operations
- Creating files with `>`
- Appending with `>>`
- Reading files with `cat`
- **Why it matters:** Claude-code often reads/writes files

### 4. Git Operations
- `git status`
- `git log --oneline -5`
- `git branch`
- **Why it matters:** Essential for development work with Claude

### 5. Environment Variables
- Reading `$HOME`, `$USER`, `$PATH`
- Setting and reading variables
- **Why it matters:** Claude-code and tools need env vars

## Understanding Results

### Speedup Factor
- **> 1.0x**: Rush is faster
- **< 1.0x**: Zsh is faster
- **Example**: 2.5x means Rush is 2.5 times faster than Zsh

### What to Look For

**Good signs:**
- ✅ Most benchmarks show Rush ≥ 0.8x (within 20% of Zsh)
- ✅ Shell startup similar or faster
- ✅ No tests with Rush > 2x slower

**Red flags:**
- ⚠️ Any test where Rush is > 2x slower than Zsh
- ⚠️ Shell startup significantly slower (affects every command)
- ⚠️ Basic commands (pwd, echo) much slower

## Running Benchmarks

### Prerequisites
```bash
# Build Rush in release mode (required for fair comparison)
cargo build --release

# Verify Rush works
./target/release/rush -c "echo 'test'"
```

### Quick Benchmark
```bash
# 20 runs of each test (fast)
./benches/quick_benchmark.sh

# Customize number of runs
# Edit quick_benchmark.sh and change: RUNS=50
```

### Detailed Benchmark
```bash
# Default: 10 runs per test
python3 ./benches/claude_code_benchmark.py

# Custom number of runs (more runs = more accurate)
python3 ./benches/claude_code_benchmark.py ./target/release/rush 20

# Custom Rush path
python3 ./benches/claude_code_benchmark.py /usr/local/bin/rush 10
```

### Viewing Results

**Terminal output:**
- Summary statistics
- Detailed comparison table
- Biggest improvements
- Areas to improve

**JSON file:**
```bash
# View results
cat benchmark_results_claude_code.json | python3 -m json.tool

# Compare with previous run
diff benchmark_results_claude_code.json benchmark_results_old.json
```

## Interpreting Results for Claude Code

### What Matters Most for Claude Code

1. **Shell Startup** (High Priority)
   - Claude-code spawns subshells for commands
   - Faster startup = faster command execution in Claude sessions
   - Target: Within 10% of Zsh

2. **Command Substitution** (High Priority)
   - Claude-code uses `$(command)` to capture output
   - Critical for tool use and command execution
   - Target: Within 20% of Zsh

3. **File Operations** (Medium Priority)
   - Claude reads/writes files frequently
   - Redirection performance matters
   - Target: Within 30% of Zsh

4. **Git Operations** (Medium Priority)
   - Common in development workflows
   - Should be as fast as Zsh (same git binary)
   - Target: Within 10% of Zsh

5. **Pipes** (Medium Priority)
   - Used in complex commands
   - Target: Within 20% of Zsh

### Decision Criteria

**Rush is ready if:**
- ✅ Shell startup ≤ 1.2x Zsh time (within 20%)
- ✅ No single benchmark > 3x slower
- ✅ Average speedup ≥ 0.7x (Rush is at most 30% slower overall)
- ✅ Claude-code feels responsive in practice

**Need optimization if:**
- ⚠️ Shell startup > 1.5x Zsh time
- ⚠️ Any core command > 3x slower
- ⚠️ Average speedup < 0.5x

**Not ready if:**
- ❌ Shell startup > 2x Zsh time
- ❌ Claude-code noticeably laggy in practice
- ❌ Basic commands (pwd, echo) > 2x slower

## Real-World Testing

After benchmarks, test Claude-code interactively:

```bash
# Start Rush
./target/release/rush

# Start Claude
claude

# Try these in Claude:
# 1. "list files in this directory"
# 2. "read src/main.rs"
# 3. "run cargo test --lib"
# 4. "what's the git status?"
# 5. "create a new file called test.md with hello world"

# Does it feel responsive?
# Any noticeable lag compared to Zsh?
```

## Continuous Benchmarking

### Track Performance Over Time

```bash
# Run benchmark and save with date
python3 ./benches/claude_code_benchmark.py > benchmark_$(date +%Y%m%d).txt

# Compare with previous
diff benchmark_20250120.txt benchmark_20250121.txt
```

### Before/After Optimization

```bash
# Baseline
python3 ./benches/claude_code_benchmark.py
mv benchmark_results_claude_code.json benchmark_before.json

# Make changes to Rush...
cargo build --release

# New benchmark
python3 ./benches/claude_code_benchmark.py
mv benchmark_results_claude_code.json benchmark_after.json

# Compare
diff <(cat benchmark_before.json | jq '.comparison') \
     <(cat benchmark_after.json | jq '.comparison')
```

## Known Performance Characteristics

### Expected Rush Performance

Based on the implementation:

**Faster than Zsh:**
- ✅ Built-in commands (cat, grep, ls, git status)
  - Rush has optimized Rust implementations
  - Should be significantly faster (see previous benchmarks)

**Similar to Zsh:**
- ≈ External command execution
  - Same process spawning overhead
  - Same binary execution

**Potentially slower:**
- ⚠️ Shell startup (first-time)
  - Rust binary loading
  - Should be minimal (<10ms difference)

**Optimization opportunities:**
- Caching compiled scripts
- Pre-warming reedline
- Lazy module initialization

## Troubleshooting

### Benchmark Hangs
```bash
# Reduce timeout in claude_code_benchmark.py
# Edit line ~40: timeout=30  ->  timeout=5
```

### Inconsistent Results
```bash
# Increase runs for better statistics
python3 ./benches/claude_code_benchmark.py ./target/release/rush 50

# Run on idle system (close other apps)
# Disable background processes
```

### Command Failures
```bash
# Check which commands fail
python3 ./benches/claude_code_benchmark.py 2>&1 | grep "Warning"

# Test failed command manually
./target/release/rush -c "failing_command"
```

## Next Steps

After benchmarking:

1. **Review Results**
   - Check if performance is acceptable
   - Identify any major slowdowns

2. **Real-World Testing**
   - Use Rush with Claude-code for actual work
   - Does it *feel* fast enough?

3. **Optimize if Needed**
   - Profile slow operations
   - Implement optimizations
   - Re-benchmark to verify improvements

4. **Set as Default Shell**
   - If performance is good and Claude-code works well
   - Follow CHECKLIST_BEFORE_DEFAULT_SHELL.md

## Benchmarking Tips

- **Warm up**: Run benchmarks twice, use second run results
- **Consistency**: Close other apps during benchmarking
- **Multiple runs**: More runs = more accurate statistics
- **Real workload**: Benchmark matches your actual usage
- **Subjective feel**: Numbers matter, but responsiveness matters more!

Remember: A shell that's 10% slower but feels responsive is better than one that's 10% faster but has UI lag!
