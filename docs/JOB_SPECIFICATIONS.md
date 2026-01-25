# POSIX Job Specifications Implementation

## Overview

This document describes the implementation of full POSIX job specifications in rush shell (bead rush-dgr.26).

## Job Specification Syntax

The following job specification patterns are supported:

### 1. By Job Number: `%n`

Refers to job number n.

```bash
kill %1       # Kill job 1
fg %2         # Bring job 2 to foreground
wait %3       # Wait for job 3
```

### 2. Current Job: `%%` or `%+` or `%`

Refers to the most recently started or foregrounded job (current job).

```bash
fg %%         # Foreground current job
bg %+         # Background current job (equivalent to %%)
kill %        # Kill current job (equivalent to %%)
```

### 3. Previous Job: `%-`

Refers to the second most recent job (previous job).

```bash
fg %-         # Foreground previous job
bg %-         # Background previous job
```

### 4. By Command Prefix: `%string`

Refers to a job whose command line starts with the given string.

```bash
fg %vim       # Foreground job starting with "vim"
kill %sleep   # Kill job starting with "sleep"
wait %grep    # Wait for job starting with "grep"
```

If multiple jobs match, returns an ambiguous error.

### 5. By Command Substring: `%?string`

Refers to a job whose command line contains the given string.

```bash
fg %?document      # Foreground job containing "document"
kill %?pattern     # Kill job containing "pattern"
wait %?300         # Wait for job containing "300"
```

If multiple jobs match, returns an ambiguous error.

### 6. Plain Number: `n`

Can also specify job by number without the % prefix (for compatibility).

```bash
fg 1          # Same as fg %1
bg 2          # Same as bg %2
```

## Implementation

### Core Function

The `JobManager::parse_job_spec()` method in `src/jobs/mod.rs` implements the parsing logic:

```rust
pub fn parse_job_spec(&self, spec: &str) -> Result<Job, String>
```

This method:
- Handles all POSIX job specification patterns
- Returns a `Job` struct if successful
- Returns a descriptive error string if:
  - The job spec doesn't match any job
  - The job spec is ambiguous (matches multiple jobs)
  - The job spec is invalid

### Helper Functions

Two private helper functions support the implementation:

1. `find_job_starting_with(&self, prefix: &str) -> Result<Job, String>`
   - Finds jobs where command starts with prefix
   - Returns ambiguous error if multiple matches

2. `find_job_containing(&self, substring: &str) -> Result<Job, String>`
   - Finds jobs where command contains substring
   - Returns ambiguous error if multiple matches

### Builtins Updated

The following builtins now support full job specifications:

1. **fg** (`src/builtins/jobs.rs`)
   - Brings a job to foreground
   - Usage: `fg [jobspec]`
   - Default: current job if no argument

2. **bg** (`src/builtins/jobs.rs`)
   - Continues a stopped job in background
   - Usage: `bg [jobspec]`
   - Default: current job if no argument

3. **wait** (`src/builtins/wait.rs`)
   - Waits for job to complete
   - Usage: `wait [jobspec...]`
   - Default: waits for all jobs if no argument

4. **kill** (`src/builtins/kill.rs`)
   - Sends signal to job
   - Usage: `kill [-signal] jobspec...`
   - Supports: `kill %1`, `kill -INT %?pattern`, etc.

## Error Handling

The implementation provides clear, descriptive error messages:

- **No current job**: When using %%, %+, or % and no jobs exist
- **No previous job**: When using %- and fewer than 2 jobs exist
- **No such job**: When job number doesn't exist or prefix has no matches
- **No job contains 'X'**: When %?string has no matches
- **Ambiguous job specification**: When multiple jobs match a pattern
- **Invalid job specification**: When format is incorrect (e.g., bare %?)

## Examples

### Basic Usage

```bash
# Start some background jobs
sleep 100 &          # Job 1
vim document.txt &   # Job 2
grep -r pattern . &  # Job 3

# Use job specifications
jobs                 # List all jobs
fg %2                # Foreground vim job
bg %1                # Background sleep job (if it was stopped)
kill %3              # Kill grep job

# Current and previous
fg %%                # Foreground most recent job (job 3)
bg %-                # Background previous job (job 2)

# By command prefix
fg %vim              # Foreground the vim job
kill %sleep          # Kill the sleep job

# By command substring
fg %?document        # Foreground job containing "document"
wait %?pattern       # Wait for job containing "pattern"
```

### Advanced Examples

```bash
# Multiple sleep jobs - will error on %sleep
sleep 100 &
sleep 200 &
fg %sleep            # Error: Ambiguous job specification

# But can use %?100 or %?200 to distinguish
fg %?100             # Foreground the "sleep 100" job
fg %?200             # Foreground the "sleep 200" job

# Can also use job numbers
fg %1                # Foreground job 1
fg %2                # Foreground job 2
```

## Testing

A comprehensive test suite exists in `tests/job_spec_tests.rs` covering:

- All job specification patterns
- Ambiguous matches
- Non-existent jobs
- Edge cases (empty job list, single job, etc.)
- Integration with fg, bg, wait, kill builtins

Manual testing was performed with a standalone Rust program that validates:
- All 12 core patterns work correctly
- Error cases are handled properly
- Ambiguous matches are detected

## POSIX Compliance

This implementation follows POSIX.1-2017 specifications for job control:

- ✓ %n - job number n
- ✓ %% or %+ - current job
- ✓ %- - previous job
- ✓ %string - job starting with string
- ✓ %?string - job containing string
- ✓ Ambiguous match detection
- ✓ Clear error messages

## Files Modified

1. **src/jobs/mod.rs**
   - Added `parse_job_spec()` method to JobManager
   - Added `find_job_starting_with()` helper
   - Added `find_job_containing()` helper

2. **src/builtins/jobs.rs**
   - Updated `builtin_fg()` to use JobManager::parse_job_spec
   - Updated `builtin_bg()` to use JobManager::parse_job_spec
   - Replaced local parse_job_spec with centralized version

3. **src/builtins/wait.rs**
   - Updated to use JobManager::parse_job_spec
   - Replaced local duplicate implementation

4. **src/builtins/kill.rs**
   - Added support for job specifications
   - Updated documentation to show job spec examples

## Future Enhancements

Potential improvements for future work:

1. Support for negative job numbers (process groups)
2. Case-insensitive command matching option
3. Regular expression support for %?pattern
4. Tab completion for job specifications
5. More detailed job status in error messages

## References

- POSIX.1-2017 Shell & Utilities: https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_12
- Bash Job Control: https://www.gnu.org/software/bash/manual/html_node/Job-Control-Basics.html
