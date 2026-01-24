# I/O and External Process Optimization Opportunities

## Current Analysis

After optimizing the core execution loop (lex/parse/execute), the remaining bottlenecks are:
- **External command spawning**: Process creation overhead
- **I/O operations**: File reads, writes, redirects
- **Progress indicator overhead**: Adds latency to every command

## Discovered Bottlenecks

### 🔥 CRITICAL: Progress Indicator Sleep (200ms per command!)

**Location**: `src/executor/mod.rs:505`

```rust
// Wait a bit to see if command completes quickly
thread::sleep(Duration::from_millis(crate::progress::PROGRESS_THRESHOLD_MS));  // 200ms!

// Check if command is still running
let progress = match child.try_wait() {
    Ok(Some(_)) => None, // Command already finished
    _ => {
        // Command still running, show progress indicator
        ...
    }
};
```

**Problem**: Rush sleeps for 200ms BEFORE checking if a command finished!
- Fast commands like `pwd`, `echo`, `ls` complete in <10ms
- But we always wait 200ms before checking
- This adds **200ms overhead to every external command**

**Impact**:
- Current session benchmark: 5.05ms per command
- Estimated with progress sleep: **Could be adding ~200ms per external command**
- This might not show up in our RUSH_PERF measurements (only measures lex/parse/execute)

**Potential Fix Options**:

1. **Immediate check first** (Low risk, high reward):
   ```rust
   // Check immediately if command finished
   let progress = match child.try_wait() {
       Ok(Some(_)) => None, // Command already finished, no sleep needed!
       Ok(None) => {
           // Still running, wait threshold before showing progress
           thread::sleep(Duration::from_millis(PROGRESS_THRESHOLD_MS));
           match child.try_wait() {
               Ok(Some(_)) => None, // Finished during sleep
               _ => Some(ProgressIndicator::new(format!("Running {}", command.name)))
           }
       }
       Err(_) => None,
   };
   ```
   **Expected savings**: Up to 200ms for fast commands

2. **Reduce threshold** (Quick win):
   ```rust
   pub const PROGRESS_THRESHOLD_MS: u64 = 50;  // Down from 200ms
   ```
   **Expected savings**: 150ms per command

3. **Adaptive threshold** (More sophisticated):
   - Track average command execution times
   - Only show progress for commands that historically take >1s
   **Expected savings**: Varies, but eliminates delay for fast commands

### 🎯 MEDIUM: Polling Loop Overhead

**Location**: `src/executor/mod.rs:547-553, 581-587`

```rust
Ok(None) => {
    // Still running, sleep briefly and check again
    thread::sleep(Duration::from_millis(100));  // 100ms polling interval
}
```

**Problem**: Uses 100ms polling instead of async or event-based waiting
- Adds latency (0-100ms depending on timing)
- CPU wakes up every 100ms to check

**Potential Fix**:
1. **Reduce polling interval**:
   ```rust
   thread::sleep(Duration::from_millis(10));  // 10ms instead of 100ms
   ```
   **Expected savings**: 0-90ms latency reduction

2. **Use wait() directly** (if signals not needed):
   ```rust
   let output = child.wait_with_output()?;
   ```
   **Expected savings**: Eliminates polling overhead entirely

3. **Async process handling** (larger refactor):
   - Use tokio::process::Command
   - Await completion instead of polling
   **Expected savings**: Best latency, but requires async runtime

### ⚡ LOW-MEDIUM: Environment Copy Overhead

**Location**: `src/executor/mod.rs:357`

```rust
cmd.args(&args)
    .current_dir(self.runtime.get_cwd())
    .envs(self.runtime.get_env());  // Copies entire environment HashMap
```

**Problem**: Copies entire environment for every external command
- If environment has 50 variables, that's 50 string clones
- Each variable name + value is cloned

**Potential Fix**:
1. **Cache env as Vec<(OsString, OsString)>** in Runtime:
   ```rust
   pub struct Runtime {
       env_cache: Option<Vec<(OsString, OsString)>>,
       env_dirty: bool,
   }
   ```
   Only rebuild cache when env changes (on `export`)

2. **Use std::env::vars_os() directly** (if no modifications):
   ```rust
   // If environment hasn't been modified, use system env
   if !self.runtime.has_custom_env() {
       cmd.envs(std::env::vars_os());
   } else {
       cmd.envs(self.runtime.get_env());
   }
   ```

**Expected savings**: 20-50µs per command

### 💧 LOW: String Conversion Allocations

**Location**: `src/executor/mod.rs:597-598`

```rust
let mut stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
```

**Problem**: `from_utf8_lossy` + `to_string()` creates temporary Cow and then allocates
- For valid UTF-8, could use `from_utf8_unchecked` (unsafe but faster)
- For small outputs, unnecessary allocation

**Potential Fix**:
```rust
// Try fast path first
let stdout_str = match String::from_utf8(output.stdout) {
    Ok(s) => s,  // No allocation, reuse Vec's buffer
    Err(e) => String::from_utf8_lossy(e.as_bytes()).to_string(),  // Fallback
};
```

**Expected savings**: 10-30µs per command

## Optimization Priority

### Phase 3A: Quick Wins (Target: -200ms for external commands)

1. ✅ **Remove/reduce progress indicator sleep** (CRITICAL)
   - Expected: -200ms → -50ms for fast commands
   - Risk: Low
   - Implementation: 1 hour

2. ✅ **Reduce polling interval** (MEDIUM)
   - Expected: -50ms average latency
   - Risk: Very low
   - Implementation: 5 minutes

3. ✅ **Optimize string conversions** (LOW)
   - Expected: -20µs per command
   - Risk: Low
   - Implementation: 30 minutes

**Total expected improvement**: ~250ms for commands with external process spawning

### Phase 3B: Deeper Optimizations

1. **Cache environment variables**
   - Expected: -30µs per command
   - Risk: Medium (need to track dirty state)
   - Implementation: 2 hours

2. **Async process spawning**
   - Expected: Best latency, better CPU utilization
   - Risk: High (major refactor, adds tokio dependency)
   - Implementation: 1-2 days

## Testing Plan

1. **Before/after benchmark**:
   ```bash
   # Test fast external commands
   time (for i in {1..100}; do echo "pwd" | ./target/release/rush; done)
   ```

2. **Progress indicator test**:
   ```bash
   # Fast command should not show progress
   echo "ls" | RUSH_PERF=1 ./target/release/rush

   # Slow command should show progress after threshold
   echo "sleep 1" | ./target/release/rush
   ```

3. **Session benchmark**:
   ```bash
   bash benches/session_benchmark.sh
   ```

## Estimated Impact

Current session performance: **5.05ms per command**

After Phase 3A:
- Pure builtins: ~3µs (no change, already optimal)
- External commands: **~5.0ms → ~0.8ms** (removing 200ms sleep effect)
- Mixed workload: **~5.05ms → ~1.5ms** 🎯

**This would achieve the <2ms goal and possibly reach 1.5ms!**

## Implementation Order

1. ⏳ Remove progress sleep (immediate check first)
2. ⏳ Reduce polling interval to 10ms
3. ⏳ Optimize string conversions
4. ⏳ Benchmark and measure
5. ⏳ (Optional) Environment caching if still not at goal
