# Rush Beans: Verification Patterns - Complete Index

## Overview

This directory contains comprehensive analysis of acceptance criteria patterns from the first 20 Rush project beans. The research identifies 7 distinct verification patterns that correlate with bean types and provides concrete bash commands for verifying each pattern.

**Quick Start**: If you have a bean to verify, go to **File 2** (Quick Reference CSV) for the verify command, or use the **Decision Tree** in File 4.

---

## Files in This Analysis

### 1. **acceptance_criteria_patterns.md** (14 KB)
**Purpose**: Detailed, academic analysis of acceptance criteria patterns
**Audience**: Team leads, architects, pattern designers
**Contains**:
- Executive summary of 7 patterns
- Pattern 1-7 detailed explanations with philosophy
- Acceptance criteria templates with examples from real beans
- Verify command recommendations
- Key insights and patterns
- File locations and structure guidance

**When to use**: Deep dive into why each pattern exists and what it means

---

### 2. **verify_commands_quick_ref.csv** (5.1 KB)
**Purpose**: Quick lookup table for bean verification commands
**Audience**: Individual contributors, engineers implementing beans
**Contains**:
- All 20 beans in CSV format
- Columns: Bean_ID, Title, Pattern, Key_Acceptance_Criteria, Suggested_Verify_Command, Effort_Days
- One row per bean for quick reference
- Direct copy-paste verify commands

**When to use**:
```bash
# Look up verify command for bean 32:
grep "^32," verify_commands_quick_ref.csv | cut -d, -f5
# Result: cargo build --release && time ./target/release/rush -c 'git_log --json -n 10' | jq '.[] | {hash, author}'
```

**Format**: Machine-readable CSV, can be parsed with awk/cut/jq

---

### 3. **verification_patterns_by_type.md** (9.1 KB)
**Purpose**: Pattern-by-pattern cookbook with concrete bash examples
**Audience**: Engineers needing to verify beans, CI/CD engineers
**Contains**:
- Pattern 1-7 breakdown with real bash commands
- Test categories for each pattern
- Primary and fallback verify commands
- Expected results for each pattern
- Complete verification checklist template
- Performance baseline reference

**When to use**: Copy bash commands directly into your terminal or CI scripts

**Example sections**:
- Pattern 4: Performance Benchmarking (with hyperfine examples)
- Pattern 5: JSON Structured Output (with jq validation examples)
- Pattern 6: Stability & Signal Handling (with timeout/signal examples)

---

### 4. **pattern_visual_reference.txt** (30 KB)
**Purpose**: Visual ASCII reference for patterns and decision tree
**Audience**: All users, especially visual learners
**Contains**:
- ASCII-box diagrams for all 7 patterns
- Visual breakdown of verification layers
- Test categories in visual format
- Universal verification checklist (boxed)
- Decision tree flowchart (boxed)
- Quick pattern reference table

**When to use**: Print and post on your desk, or grep for quick lookup

**To use the decision tree**:
```bash
# Just answer yes/no questions following the decision tree in the file
# It will tell you which pattern to use
```

---

### 5. **bean_verification_summary.txt** (12 KB)
**Purpose**: Executive summary with all essentials in one document
**Audience**: Project leads, new team members, quick reference
**Contains**:
- All 7 patterns with quick descriptions
- One summary table showing all patterns at a glance
- Quick decision tree (condensed version)
- Universal verification checklist
- Key insights (5 findings)
- Next steps/how to use this research

**When to use**: First document to read when joining the project, or when you need a quick refresh

**Recommendation**: Read this first (5 min), then go to specific files as needed.

---

## Pattern Quick Reference

| Pattern | Count | Use When | Primary Command |
|---------|-------|----------|-----------------|
| **1: Cargo Checks** | 20/20 | Always (universal) | `cargo build && cargo test && cargo clippy` |
| **2: Functional Tests** | 7 | Shell feature work | `./tests/smoke_test.sh` or `echo "cmd" \| rush` |
| **3: File Checks** | 2 | Create/modify files | `test -f FILE && grep PATTERN FILE` |
| **4: Performance** | 4 | Optimization work | `hyperfine before/after` |
| **5: JSON Output** | 3 | git_* with --json | `command --json \| jq .` |
| **6: Stability** | 3 | System integration | `echo "cmd" \| rush` or `timeout/signal` tests |
| **7: Epic Reference** | 3 | Multi-bean features | `cargo test` + child bean verification |

---

## Beans by Pattern Type

### Pattern 1: Cargo Checks (Universal)
**All 20 beans** require: `cargo build --release && cargo test && cargo clippy -- -D warnings`

### Pattern 2: Functional Tests (7 beans)
18, 19, 21, 22.1, 23, 43, 50
- Verify actual shell behavior
- Primary: `./tests/smoke_test.sh` or `echo "cmd" | rush`

### Pattern 3: File Checks (2 beans)
58, 4
- Verify files exist with correct content
- Primary: `test -f FILE && grep PATTERN FILE`

### Pattern 4: Performance (4 beans)
141, 5, 12, 13
- Verify measurable improvement
- Primary: `hyperfine before/after`

### Pattern 5: JSON Output (3 beans)
32, 34, 35
- Verify JSON validity and performance
- Primary: `command --json | jq .`

### Pattern 6: Stability (3 beans)
43, 49, 50
- Verify system integration
- Primary: `echo "cmd" | rush` or signal tests

### Pattern 7: Epic Reference (3 beans)
2, 6, 19
- Verify all child beans pass
- Primary: `cargo test` + child verification

---

## How to Use This Research

### Scenario 1: "I need to implement bean 32 (git_log --json)"
1. Open **verify_commands_quick_ref.csv**
2. Grep for bean 32
3. Copy the Suggested_Verify_Command
4. You've got your verification command ready

```bash
# From CSV:
time ./target/release/rush -c 'git_log --json -n 10' | jq '.[] | {hash, author}'
```

### Scenario 2: "I'm stuck on acceptance criteria for my bean"
1. Determine your bean type (functional? performance? JSON?)
2. Open **verification_patterns_by_type.md**
3. Find your pattern section
4. Copy the acceptance criteria template and verify commands
5. Adapt to your specific bean

### Scenario 3: "I need to verify 5 beans in CI"
1. Open **verify_commands_quick_ref.csv**
2. Extract rows for your 5 beans
3. Create a bash loop with the verify commands
4. Run in CI pipeline

```bash
#!/bin/bash
for bean_id in 32 34 35 43 49; do
  cmd=$(grep "^$bean_id," verify_commands_quick_ref.csv | cut -d, -f5)
  eval "$cmd" || exit 1
done
```

### Scenario 4: "I'm new to the project and confused about verification"
1. Read **bean_verification_summary.txt** (5 minutes)
2. Look at **pattern_visual_reference.txt** decision tree
3. When you need implementation details, go to **verification_patterns_by_type.md**
4. Keep **verify_commands_quick_ref.csv** handy for quick lookups

### Scenario 5: "I want to understand WHY each pattern exists"
1. Read **acceptance_criteria_patterns.md**
2. Look for section on "Key Insights"
3. Understand the philosophy behind each pattern

---

## File Selection Guide

**Need a command to run?**
→ Use **File 2** (CSV) or **File 3** (patterns_by_type.md)

**Need to understand patterns?**
→ Use **File 1** (patterns.md) or **File 5** (summary.txt)

**Visual learner?**
→ Use **File 4** (visual_reference.txt)

**Getting started?**
→ Start with **File 5** (summary.txt), then pick specific files as needed

**CI/CD engineer?**
→ Use **File 2** (CSV) + **File 3** (patterns_by_type.md)

---

## Key Statistics from Analysis

- **20 beans analyzed** from `bn ready | head -20`
- **7 distinct patterns** identified
- **100%** of beans require cargo checks (Pattern 1)
- **35%** of beans require functional shell tests (7 beans)
- **20%** of beans require performance benchmarking (4 beans)
- **15%** of beans require JSON structured output (3 beans)
- **15%** of beans require system stability testing (3 beans)
- **15%** of beans are epics that decompose to sub-beans (3 beans)

---

## Verification Checklist for Any Bean

```bash
#!/bin/bash
set -e

echo "=== Bean Verification Checklist ==="

# 1. Universal (PATTERN 1: Cargo Checks)
echo "1. Building..."
cargo build --release

echo "2. Testing..."
cargo test

echo "3. Code quality..."
cargo clippy -- -D warnings

# 4. Pattern-specific (determine from acceptance criteria)
echo "4. Feature-specific verification..."
# [Insert pattern-specific command from CSV/patterns_by_type.md]

# 5. Regression check
echo "5. Smoke test..."
./tests/smoke_test.sh

echo "✓ Bean verification complete"
```

---

## How Data Was Collected

1. Ran `bn ready | head -20` to get first 20 beans
2. Extracted bean IDs: 58, 141, 143, 2, 4, 5, 6, 12, 13, 18, 19, 21, 22.1, 23, 32, 34, 35, 43, 49, 50
3. For each bean, ran `bn show <id> --json`
4. Extracted acceptance criteria (the `[ ]` checkbox items) from descriptions
5. Categorized criteria by type (cargo, functional, file, performance, JSON, stability, epic)
6. Identified 7 distinct patterns
7. Documented verify commands for each pattern

---

## Next Steps

1. **Use this for new beans**: When implementing a new bean, check if it matches one of these 7 patterns
2. **Standardize on patterns**: New beans should use one of these patterns or propose a new one
3. **Automate verification**: Create a `verify.sh` script that auto-detects bean pattern and runs verify command
4. **CI integration**: Add pattern-based verification to CI/CD pipeline
5. **Train team**: Share this index with the team, especially the decision tree and quick reference

---

## Questions?

- **What pattern does my bean use?** → Check decision tree in File 4 (pattern_visual_reference.txt)
- **What command should I run?** → Check File 2 (CSV) or File 3 (patterns_by_type.md)
- **Why does this pattern exist?** → Check File 1 (acceptance_criteria_patterns.md)
- **Show me an example** → Check File 5 (summary.txt) for concrete examples

---

## Files at a Glance

```
/Users/asher/tt/rush/research/
├── acceptance_criteria_patterns.md        [14 KB] - Detailed analysis
├── verify_commands_quick_ref.csv          [5.1 KB] - Machine-readable lookup
├── verification_patterns_by_type.md       [9.1 KB] - Cookbook with examples
├── pattern_visual_reference.txt           [30 KB] - Visual ASCII reference
├── bean_verification_summary.txt          [12 KB] - Executive summary
└── BEAN_VERIFICATION_INDEX.md             [This file]
```

**Total Analysis**: ~80 KB of comprehensive verification guidance
**First 20 Beans**: Fully mapped with concrete verify commands
**Patterns Identified**: 7 distinct types
**Time to Read Complete Guide**: ~15 minutes
**Time to Look Up One Verify Command**: ~10 seconds (using CSV)
