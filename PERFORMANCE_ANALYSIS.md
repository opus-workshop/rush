# Rush Performance Analysis

## Current Performance: 2.3ms per command

### Profiling Breakdown

```
Lex:       3.07µs (  0.1%)
Parse:     2.86µs (  0.1%)
Execute: 2305.97µs ( 99.7%)
Total:   2311.91µs per command
```

## Key Findings

### 1. Parsing is NOT the bottleneck
- Lexer + Parser combined: **6µs** (0.2% of total time)
- This means parser caching would save at most 6µs per command
- **Not worth optimizing**

### 2. Execute phase is 99.7% of time
The executor includes:
- Argument expansion (variables, globs, command substitution)
- Builtin lookup
- External command spawning
- I/O operations
- Git operations

**This is where we need to optimize**

## Optimization Opportunities

### High Impact (Expected 50%+ improvement)

1. **Lazy Git Context** - Don't check git status on every command
   - Currently: GitContext created for every prompt update
   - Solution: Cache git context, invalidate on cd/git commands
   - Expected savings: 50-100µs per command

2. **Reduce Cloning** - Minimize Vec/String allocations
   - Use `&str` instead of `String` where possible
   - Use `Cow<str>` for conditional ownership
   - Expected savings: 20-50µs per command

3. **Optimize Argument Expansion** - Fast path for simple cases
   - Check if expansion is needed before allocating
   - Use slice operations instead of collecting
   - Expected savings: 10-30µs per command

### Medium Impact (Expected 10-30% improvement)

4. **Builtin Dispatch** - Use match instead of HashMap
   - HashMap lookup: ~10-20ns, but happens on every command
   - Direct match: ~1-2ns
   - Expected savings: 5-10µs per command

5. **Environment Variable Lookup** - Cache common vars
   - $HOME, $USER, $PATH are read frequently
   - Cache in Runtime with invalidation
   - Expected savings: 5-15µs per command

### Low Impact (Expected <10% improvement)

6. **Parser Caching** - Cache AST for repeated commands
   - Only saves 6µs per command
   - Complex invalidation logic
   - **Skip this**

## Target Performance Goals

**Current:** 2.3ms per command
**Goal 1 (50% faster):** 1.15ms per command  ✓ Achievable
**Goal 2 (1ms):** 1.0ms per command  ✓ Stretch but possible
**Goal 3 (Sub-ms):** <1.0ms per command  ⚠️ Very difficult

## Implementation Plan

### Phase 1: Quick Wins (Target: 1.5ms)
1. Lazy git context caching
2. Remove unnecessary clones in executor
3. Fast path for simple commands (no expansion needed)

### Phase 2: Deep Optimization (Target: 1.0ms)
1. Optimize argument expansion
2. Switch builtin dispatch to match
3. Cache environment variables

### Phase 3: Micro-optimizations (Target: <1ms)
1. Use `SmallVec` for small argument lists
2. Arena allocation for temporary strings
3. Inline hot functions
4. Profile-guided optimization (PGO)

## Measuring Progress

Run with profiling enabled:
```bash
RUSH_PERF=1 ./target/release/rush < test_script.sh
```

Compare before/after:
```bash
# Before optimization
Total:   2311.91µs per command

# After Phase 1 (target)
Total:   1500.00µs per command  (-35%)

# After Phase 2 (target)
Total:   1000.00µs per command  (-57%)

# After Phase 3 (stretch)
Total:    900.00µs per command  (-61%)
```

## Next Steps

1. ✅ Add profiling instrumentation
2. ✅ Identify bottlenecks (Execute: 99.7%)
3. 🔄 Add detailed executor profiling
4. ⏳ Implement Phase 1 optimizations
5. ⏳ Measure and iterate
