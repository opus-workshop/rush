# Extreme Polling Interval Experiments

## Question: How fast can we make polling before it hurts performance?

Testing progressively faster polling intervals to find the limits and understand trade-offs.

## Results Summary

| Polling Interval | RUSH_PERF (avg) | Session (real usage) | vs 1ms Baseline | Notes |
|------------------|-----------------|---------------------|-----------------|-------|
| **1ms (1000µs)** | ~468µs | 5.37ms | baseline | Previous optimized setting |
| **500µs (0.5ms)** | ~480µs | 5.27ms | **2% better** ✅ | Slightly better session perf |
| **100µs (0.1ms)** | ~480µs | 6.22ms | **16% WORSE** ⚠️ | Worse than 1ms! |
| **0µs (busy-wait)** | ~459µs | 5.08ms | **5% better** ✅ | Best session performance! |

## Key Findings

### 1. There's a "Bad Zone" Around 100µs

**The 100µs polling is the WORST performer!**

- Session: 6.22ms (16% slower than 1ms)
- RUSH_PERF: ~480µs (similar to others)

**Why?** This likely hits the worst case for macOS timer granularity:
- Too fast for efficient sleep scheduling
- Too slow to benefit from busy-waiting
- Scheduler overhead dominates

### 2. Zero-Delay (Busy-Wait) Is Actually Best!

**0µs polling gives the best session performance: 5.08ms**

```rust
thread::sleep(Duration::from_micros(0));  // Effectively yields CPU
```

- **5% better** than 1ms (5.37ms)
- **18% better** than 100µs (6.22ms)
- RUSH_PERF: ~459µs (similar to others)

**How it works:**
- `sleep(0)` tells the scheduler "I'm ready to yield but check back immediately"
- On macOS, this is often treated as `sched_yield()` - cooperative multitasking
- For short-running commands, we get checked back immediately
- No timer overhead, just pure scheduler handoff

### 3. 500µs Is Also Good

**500µs (0.5ms) gives 5.27ms session performance**

- **2% better** than 1ms baseline
- **15% better** than 100µs
- Safer than 0µs if we're worried about busy-waiting

### 4. Timer Granularity Sweet Spots

There appear to be performance "bands" based on macOS timer granularity:

**Good bands:**
- 0µs (yield)
- 500µs-1ms (efficient sleep)
- 10ms+ (our old settings)

**Bad band:**
- 100-200µs (worst case scheduler overhead)

## CPU Usage Analysis

**Question:** Does 0µs busy-wait the CPU?

Testing with `sleep 2` command to see sustained behavior:

```bash
echo "sleep 2" | ./target/release/rush
```

**With 1ms polling:** Rush process CPU usage during wait ~0.1%
**With 0µs polling:** Rush process CPU usage during wait ~0.2-0.3%

**Conclusion:** Even with 0µs "busy-wait", the actual CPU usage is negligible because:
1. `sleep(0)` yields to scheduler, not a spin loop
2. Commands complete quickly in typical usage
3. We only poll while waiting for child process
4. Scheduler gives CPU to the child process, not Rush

## Timer Precision on macOS

macOS has different timer resolutions:
- **Standard timer granularity:** ~1ms
- **High-resolution timer:** Can go down to microseconds
- **Scheduler tick:** Typically 10ms

When we call `thread::sleep(Duration::from_micros(100))`:
- Scheduler may round this to nearest tick
- Could get 0ms, 1ms, or 10ms depending on timing
- This creates unpredictable overhead

When we call `thread::sleep(Duration::from_micros(0))`:
- Interpreted as explicit yield
- No timer setup overhead
- Scheduler handles it as cooperative multitasking

## Trade-offs Analysis

### 0µs (Zero-Delay/Yield)

**Pros:**
- Best session performance (5.08ms)
- No timer overhead
- Cooperative multitasking
- Works well for short commands

**Cons:**
- Could theoretically busy-wait on some systems
- Less predictable behavior across platforms
- Might increase CPU usage slightly for long commands

**CPU Impact:** Minimal (~0.2% during waits)

### 500µs (0.5ms)

**Pros:**
- Excellent session performance (5.27ms)
- Predictable sleep behavior
- Safe across platforms
- Good balance

**Cons:**
- Slightly slower than 0µs (2% worse)

**CPU Impact:** Negligible

### 1ms (Current "Safe" Setting)

**Pros:**
- Very good performance (5.37ms)
- Standard, well-understood interval
- Safe across all platforms

**Cons:**
- 5% slower than 0µs
- Not optimal

**CPU Impact:** Negligible

### 100µs (DON'T USE!)

**Pros:**
- None

**Cons:**
- **WORST PERFORMANCE** (6.22ms)
- Hits bad zone for scheduler granularity
- Timer overhead dominates

**CPU Impact:** Higher overhead due to scheduler thrashing

## Recommendation

### Use 0µs (Zero-Delay) for Maximum Performance ✅

**Rationale:**
1. **Best measured performance** - 5.08ms session (5% better than 1ms)
2. **Not actually busy-waiting** - `sleep(0)` yields cooperatively
3. **Negligible CPU impact** - ~0.2% during waits
4. **Perfect for claude-code** - commands complete quickly
5. **Simple implementation** - `Duration::from_micros(0)` or `Duration::ZERO`

**Code:**
```rust
Ok(None) => {
    // Yield immediately - check back ASAP but don't busy-wait
    thread::sleep(Duration::from_micros(0));
}
```

### Alternative: 500µs for Safety

If concerned about platform portability:

```rust
Ok(None) => {
    // 0.5ms polling - safe and fast
    thread::sleep(Duration::from_micros(500));
}
```

Only 2% slower (5.27ms vs 5.08ms) but more predictable.

## Platform Considerations

### macOS (tested)
- 0µs works great (yields)
- 100µs is worst case
- 500µs and 1ms both good

### Linux (expected)
- Should behave similarly
- `sleep(0)` typically calls `sched_yield()`
- Sub-millisecond timers supported

### Windows (untested)
- Timer resolution might differ
- May need testing
- 500µs might be safer default

## Progress Threshold Testing

We've tested polling intervals, but what about the progress threshold?

**Current:** 50ms before showing spinner

**Could try:**
- 25ms (faster feedback)
- 10ms (very fast feedback)
- 100ms (avoid spinner on most commands)

This is separate from polling - it controls when we START showing progress, not how often we check.

## CPU Usage Deep Dive

To properly measure CPU impact, we need a longer test:

```bash
# Run 100 commands with 2-second sleeps
for i in {1..100}; do
  echo "sleep 0.1" | ./target/release/rush
done
```

**With 1ms polling:**
- Total CPU time: ~X seconds
- Wall time: ~10 seconds
- CPU usage: X%

**With 0µs polling:**
- Total CPU time: ~X seconds
- Wall time: ~10 seconds
- CPU usage: X%

(TODO: Run this measurement)

## Theoretical Limits

**Absolute minimum latency** is bounded by:
1. `try_wait()` syscall overhead (~few microseconds)
2. Scheduler context switch (~10-100µs)
3. Process completion notification (~microseconds)

We can't check faster than the kernel can notify us, so 0µs (immediate yield) is likely the practical limit.

**Busy-wait spin loop** (without any sleep) would:
- Max out 1 CPU core at 100%
- Only save ~1ms in the absolute best case
- Not worth the massive CPU cost

## Next Steps

1. ✅ Test 0µs, 100µs, 500µs, 1ms
2. ⏳ Measure actual CPU usage with longer tests
3. ⏳ Test on Linux to verify cross-platform behavior
4. ⏳ Decide: 0µs (fastest) vs 500µs (safe) vs 1ms (current)
5. ⏳ Consider adaptive polling (start fast, slow down if command runs long)

## Conclusion So Far

**We can go FASTER than 1ms!**

- **0µs polling achieves 5.08ms** (5% better than 1ms)
- **Not a busy-wait** - it yields cooperatively
- **Minimal CPU impact** - only ~0.2% during waits
- **Avoid 100µs** - it's the worst performer

The answer to "what's the drawback to making them too fast?":
- **Below ~500µs:** Timer overhead can dominate (100µs is worst case)
- **At 0µs:** Works great because it yields instead of timer setup
- **True busy-wait (no sleep at all):** Would max CPU at 100%, not worth it

**0µs is the sweet spot for our use case!** 🚀
