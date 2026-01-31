# Rush Performance Optimization Strategy

## Current Bottleneck: Startup Time

**Problem**: Rush pays ~5.4ms startup overhead per `-c` invocation
**Impact**: Simple commands appear similar speed to zsh despite 17-427x faster builtins

### Breakdown of 5.4ms Startup

```
Component                  Estimated Time    % of Total
─────────────────────────────────────────────────────────
Binary loading             ~1.5ms            28%
Tokio runtime init         ~1.2ms            22%
Dependencies loading       ~1.0ms            19%
Signal handler setup       ~0.5ms            9%
Environment init           ~0.4ms            7%
Executor creation          ~0.8ms            15%
─────────────────────────────────────────────────────────
TOTAL                      ~5.4ms            100%
```

## Optimization Priorities

### Phase 1: Lazy Initialization (Target: -2ms startup)

**Quick wins without code restructuring:**

1. **Skip unused modules for `-c` mode** (src/main.rs:149)
   ```rust
   fn run_command(command: &str, signal_handler: SignalHandler) -> Result<()> {
       // Current: Loads everything
       // Optimized: Skip reedline, completion, git, etc.
   }
   ```
   **Expected gain**: -1.0ms

2. **Defer git2 initialization**
   - Use `lazy_static` or `OnceCell`
   - Only load when git commands used
   **Expected gain**: -0.5ms

3. **Trim Tokio features** (Cargo.toml:28)
   ```toml
   # Current:
   tokio = { version = "1", features = ["full"] }

   # Optimized:
   tokio = { version = "1", features = ["rt", "process", "io-util"] }
   ```
   **Expected gain**: -0.3ms

4. **Optimize environment init** (src/main.rs:294)
   - Cache `current_exe()` result
   - Skip unnecessary env var checks
   **Expected gain**: -0.2ms

**Total Phase 1 gain**: ~2ms → **3.4ms startup**

### Phase 2: Binary Size Reduction (Target: -1ms startup)

**Smaller binary loads faster:**

1. **Split features** (Cargo.toml)
   ```toml
   [features]
   default = ["basic"]
   basic = []
   git = ["git2"]
   advanced = ["git", "reedline/external"]
   ```
   **Binary size**: 3.5MB → 2.5MB
   **Expected gain**: -0.5ms

2. **Remove redundant regex engines**
   - Keep: grep-regex (fastest)
   - Consider removing: nom regex features
   **Expected gain**: -0.2ms

3. **Optimize dependencies**
   - Use `cargo-bloat` to identify fat deps
   - Replace heavy deps with lighter alternatives
   **Expected gain**: -0.3ms

**Total Phase 2 gain**: ~1ms → **2.4ms startup**

### Phase 3: Advanced Optimizations (Target: -1ms startup)

1. **Profile-Guided Optimization (PGO)**
   ```bash
   # Generate profile
   RUSTFLAGS="-Cprofile-generate=/tmp/pgo-data" cargo build --release
   ./target/release/rush -c "echo test"  # Run workload
   ./target/release/rush -c "ls"

   # Build with profile
   RUSTFLAGS="-Cprofile-use=/tmp/pgo-data/merged.profdata" cargo build --release
   ```
   **Expected gain**: -0.5ms

2. **Custom allocator** (jemalloc or mimalloc)
   ```toml
   [dependencies]
   mimalloc = { version = "0.1", default-features = false }
   ```
   **Expected gain**: -0.3ms

3. **Startup benchmarks** (add to benches/)
   - Track startup regression
   - Optimize hot paths
   **Expected gain**: -0.2ms

**Total Phase 3 gain**: ~1ms → **1.4ms startup**

### Phase 4: Architectural Changes (Target: 0ms amortized)

1. **Script execution** (rush-j1d bead)
   - Run entire scripts in one process
   - Amortize startup across all commands
   **Speedup**: 10-100x for file-heavy scripts

2. **Daemon mode**
   ```bash
   # Start daemon
   rushd --daemon

   # Client connects (no startup)
   rush -c "ls"  # <1ms total
   ```
   **Speedup**: Near-zero startup for subsequent commands

3. **Embedded interpreter**
   - Provide library for embedding Rush
   - Apps can reuse instance
   **Use case**: Build tools, automation

## Benchmark Methodology

### Current (Misleading)

`benches/quick_benchmark.sh` spawns 20 processes per test:
```bash
for i in 1..20; do
  rush -c "echo test"  # 5.4ms each = 108ms total
done
```
**Result**: Startup dominates, builtins look slow

### Proposed (Fair)

1. **Separate benchmarks:**
   - Startup benchmark: `rush -c "exit"`
   - Builtin benchmark: In-process Criterion tests
   - Real-world: Script execution (once implemented)

2. **Report three numbers:**
   - Pure builtin speed (vs zsh process spawn)
   - Single-command speed (with startup)
   - Script speed (startup amortized)

## Implementation Plan

### Week 1: Quick Wins
- [ ] Lazy load git2 and reedline
- [ ] Trim Tokio features
- [ ] Add startup benchmark to track progress
- [ ] Document optimization in benches/README

### Week 2: Feature Flags
- [ ] Split features in Cargo.toml
- [ ] Make git2 optional
- [ ] Add basic/full builds
- [ ] Update CI to test both

### Week 3: PGO & Advanced
- [ ] Set up PGO infrastructure
- [ ] Test custom allocators
- [ ] Profile hot paths
- [ ] Implement findings

### Week 4: Daemon Prototype
- [ ] Design daemon protocol
- [ ] Implement rush server
- [ ] Implement rush client
- [ ] Benchmark against zsh

## Expected Results

| Optimization Phase | Startup Time | `rush -c "echo"` | vs Zsh |
|-------------------|--------------|------------------|--------|
| Current           | 5.4ms        | 21ms             | 0.95x  |
| Phase 1 (lazy)    | 3.4ms        | 19ms             | 1.05x ✓|
| Phase 2 (size)    | 2.4ms        | 18ms             | 1.10x ✓|
| Phase 3 (PGO)     | 1.4ms        | 17ms             | 1.15x ✓|
| Phase 4 (daemon)  | 0.1ms        | 16ms             | 1.20x ✓|

**Interactive mode**: Already faster (startup happens once)
**Script mode**: Will be 10-100x faster (Phase 4)

## Measuring Success

### Key Metrics

1. **Startup time**: `hyperfine 'rush -c exit'`
   - Current: 5.4ms
   - Target: <2ms

2. **Binary size**: `ls -lh target/release/rush`
   - Current: 3.5MB
   - Target: <2MB

3. **Simple command**: `hyperfine 'rush -c "echo test"'`
   - Current: 21ms
   - Target: <18ms

4. **File operation**: `hyperfine 'rush -c "cat 10k-file.txt"'`
   - Current: Should already beat zsh
   - Validate: Rush < Zsh

### Regression Prevention

Add to CI:
```yaml
- name: Benchmark startup
  run: |
    cargo build --release
    hyperfine --warmup 5 './target/release/rush -c exit'
    # Fail if > 3ms after Phase 1
```

## Tools & Commands

```bash
# Install profiling tools
cargo install cargo-bloat cargo-audit

# Analyze binary size
cargo bloat --release -n 20

# Profile startup
cargo build --release
time ./target/release/rush -c "exit"

# Compare features impact
cargo build --release --no-default-features
cargo build --release --features basic

# Run benchmarks
cargo bench --bench startup
cargo bench --bench builtins

# Generate flamegraph (macOS)
cargo install flamegraph
sudo flamegraph -o startup.svg ./target/release/rush -c "exit"
```

## References

- Current benchmark results: `BENCHMARK_RESULTS_UPDATED.md`
- Beans for missing features: `bn list`
- Phase 4 script execution: rush-j1d bead
- Daemon mode concept: rush-7sz bead (Phase 5)

## Next Actions

1. Create `benches/startup.rs` benchmark
2. Profile current startup with flamegraph
3. Implement lazy git2 loading
4. Trim Tokio features
5. Re-run quick_benchmark.sh and document gains

---

*Last updated: January 20, 2026*
*Author: Rush Performance Team*
