# Rush Performance Optimization Plan

## Current Performance
**Baseline: 2.31ms per command** (tested with RUSH_PERF=1)
**After Phase 1+2.1: 1.71ms per command** ✅ 26% improvement!

Breakdown:
- Lex: 1µs (0.1%)
- Parse: 1µs (0.1%)
- **Execute: 1706µs (99.9%)** ← FOCUS HERE

## Target
**1.0ms per command** (need 41% more reduction from 1.71ms)

---

## Phase 1: Quick Wins ✅ COMPLETED

### 1.1 Fast Path for Simple Arguments ✅
**Implemented:** Check if all arguments are Literal/Flag/Path and skip expansion

**Impact:** Reduced unnecessary resolve_argument calls

### 1.2 Inline resolve_argument for Literals ✅
**Implemented:** Added `#[inline]` hint

**Impact:** Better function inlining by compiler

### 1.3 Reduce Unnecessary Clones ✅
**Implemented:** Preallocate Vec capacity, handle Flag/Path in fast path

**Impact:** Fewer allocations on hot path

**Phase 1 Result:** Part of 26% total improvement

---

## Phase 2: Medium Wins (PARTIAL)

### 2.1 Optimize Builtin Dispatch ✅
**Implemented:** Added `#[inline]` hints to `is_builtin()` and `execute()`

**Impact:** Reduced HashMap lookup overhead on hot path

**Phase 2.1 Result:** Combined with Phase 1 = 26% total improvement

### 2.2 Lazy Git Context (NOT IMPLEMENTED)
**Status:** Git operations not in test workload, skipping for now
