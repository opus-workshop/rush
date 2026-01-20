# Rush Benchmarking Guide

## Overview

Rush has multiple benchmarking approaches, each measuring different aspects of performance.

## Benchmark Scripts

### 1. Fair Comparison (`compare-fair.sh`) ✅ RECOMMENDED

**Best for:** Real-world comparison with shell startup overhead

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

### 2. Criterion Benchmarks (`cargo bench`)

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

### 3. Legacy Scripts (Not Recommended)

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

## Next Steps

To improve Rush's competitive position:
1. ✅ Script file support (eliminates repeated startup) - US-001 DONE
2. Optimize startup time (lazy loading, faster parsing)
3. Add more high-performance builtins
