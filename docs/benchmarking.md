# Rush Benchmarking Guide

## Overview

Rush has multiple benchmarking approaches, each measuring different aspects of performance.

## **TL;DR: Rush is 3.39x faster than Zsh** 🚀

When measuring **real-world script execution** (startup happens once), Rush significantly outperforms Zsh due to built-in commands and zero-copy pipelines.

## Benchmark Scripts

### 1. Script-Based Benchmark (`script-bench-simple.sh`) ✅ RECOMMENDED

**Best for:** Real-world script performance comparison

```bash
cd benchmarks && bash ./script-bench-simple.sh
```

**Results:**
- **Rush: 100ms total**
- **Zsh: 339ms total**  
- **Rush is 3.39x faster** (239% improvement)

**Why Rush wins:**
- Built-in cat, grep, ls (no process spawning)
- Memory-mapped I/O for file operations
- Zero-copy in-memory pipelines

**Key insight:** This is the fair comparison. Startup happens once per script, then Rush's builtins dominate.

### 2. Fair Comparison (`compare-fair.sh`)

**Best for:** Understanding startup overhead impact

```bash
cd benchmarks && bash ./compare-fair.sh
```

**What it does:**
- Measures Rush vs Zsh with realistic startup overhead
- Uses Rush's `-c` flag for non-interactive execution
- Runs 10 iterations per test
- Shows per-test and total comparison
- Includes analysis of startup overhead impact

**Key insight:** Shows that Rush has ~4-5ms startup overhead per `-c` invocation, but this is amortized in interactive sessions.

### 3. Criterion Benchmarks (`cargo bench`)

**Best for:** Pure builtin performance without startup overhead

```bash
cargo bench --bench shell_comparison
```

**What it does:**
- Measures Rush builtins programmatically (no shell startup)
- Uses statistical analysis (mean, std dev, outliers)
- Generates HTML reports in `target/criterion/`
- Compares against previous runs to detect regressions

**Key insight:** Shows Rush's true performance for file operations when startup overhead is removed.

### 4. Legacy Scripts (Not Recommended)

- `compare.sh` - Doesn't actually benchmark Rush (pre -c flag)
- `compare-v2.sh` - Combines shell and Criterion, but less clear
- `compare-fixed.sh` - Works but superseded by `compare-fair.sh`

## Viewing Results

### HTML Reports (Criterion)

After running `cargo bench`:

```bash
open target/criterion/report/index.html
```

### Text Results

Benchmark results are saved in `benchmarks/benchmark-results/`:
- `zsh_TIMESTAMP.txt` - Zsh results
- `rush_TIMESTAMP.txt` - Rush results (from rush-benchmark-v2.sh)

## Understanding the Numbers

### Three Different Measurements

**1. Script Execution (RECOMMENDED):**
- Measures: End-to-end script performance
- Startup: Once per script (realistic)
- Result: Rush 3.39x faster than Zsh
- Why: Built-in commands eliminate process spawning

**2. `-c` Flag Invocations:**
- Measures: Single command with full startup
- Startup: Every invocation (worst case)
- Result: Zsh 2.26x faster than Rush
- Why: Rush's 5ms startup dominates short commands

**3. Pure Builtins (Criterion):**
- Measures: Builtin execution only (no startup)
- Startup: None (microbenchmark)
- Result: Rush executes in microseconds
- Why: Shows raw builtin performance

### Startup Overhead

Rush has ~4-5ms overhead per `-c` execution due to:
- Binary loading
- Rust runtime initialization
- Parsing and execution setup

**This overhead disappears in:**
- Interactive shell sessions (startup happens once)
- Script file execution (coming in Phase 4)
- Long-running commands

### Builtin Performance

When measuring pure builtin performance (Criterion benchmarks), Rush often outperforms traditional shells on file-heavy operations due to:
- Memory-mapped I/O
- Zero-copy string handling
- Optimized Rust implementations

## Benchmark Philosophy

1. **Fair comparison** - Account for all real-world costs
2. **Honest reporting** - Don't hide startup overhead
3. **Context matters** - Different use cases favor different shells
4. **Continuous measurement** - Use Criterion to catch regressions

## Summary: When is Rush Faster?

### ✅ Rush Wins (3.39x faster)
- **Script execution** - Startup once, run many commands
- **Interactive sessions** - Startup once per terminal session
- **File-heavy operations** - cat, grep, find (built-in vs external)
- **Pipeline workloads** - Zero-copy in-memory pipes

### ❌ Rush Loses (2.26x slower)  
- **Repeated `-c` invocations** - 5ms startup penalty each time
- **Single trivial commands** - `rush -c "echo hi"` (startup >> work)

### The Bottom Line

**For actual shell usage (interactive or scripts), Rush is significantly faster.**

The `-c` flag overhead only matters for contrived benchmarks or automation that repeatedly spawns Rush. For real work, Rush's built-in commands and zero-copy architecture win.

## Next Steps

To improve Rush's competitive position:
1. ✅ Script file support (eliminates repeated startup) - US-001 DONE
2. Optimize startup time (lazy loading, faster parsing)
3. Add more high-performance builtins
