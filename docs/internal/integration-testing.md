# Rush Shell Integration Testing

## Overview

This document describes the integration test suite for Rush shell, specifically designed to verify that Rush works correctly as a login shell in various non-interactive scenarios including piped input, script execution, CI/CD pipelines, and cron jobs.

## Test Suite Structure

The integration test suite is organized into focused test scripts, each covering a specific area of functionality:

```
tests/
├── integration/
│   ├── run_all.sh              # Master test runner
│   ├── login_shell_test.sh     # Comprehensive login shell tests
│   ├── non_tty_test.sh         # Non-TTY mode tests
│   ├── signal_test.sh          # Signal handling tests
│   ├── redirection_test.sh     # File redirection tests
│   ├── pipeline_test.sh        # Pipeline tests
│   └── job_control_test.sh     # Job control tests
└── fixtures/
    ├── simple_script.sh        # Basic test script
    ├── exit_code_script.sh     # Exit code testing
    ├── pipeline_script.sh      # Pipeline scenarios
    ├── redirection_script.sh   # Redirection scenarios
    ├── conditional_script.sh   # Conditional execution
    └── variable_script.sh      # Variable handling
```

## Running Tests

### Run All Tests

```bash
# From project root
./tests/integration/run_all.sh
```

### Run Individual Test Suites

```bash
# Login shell tests (30 tests)
./tests/integration/login_shell_test.sh

# Non-TTY mode tests (20 tests)
./tests/integration/non_tty_test.sh

# Signal handling tests (15 tests)
./tests/integration/signal_test.sh

# Redirection tests (20 tests)
./tests/integration/redirection_test.sh

# Pipeline tests (20 tests)
./tests/integration/pipeline_test.sh

# Job control tests (20 tests)
./tests/integration/job_control_test.sh
```

### Run with Timeout

All tests are designed to complete within 60 seconds. Use timeout to prevent hanging:

```bash
timeout 60 ./tests/integration/login_shell_test.sh
```

## Test Coverage

### 1. Login Shell Tests (`login_shell_test.sh`)

Comprehensive integration tests covering all major use cases:

- **Non-Interactive Mode (5 tests)**
  - Piped input: `echo "pwd" | rush`
  - Multiple commands via pipe
  - Empty input handling
  - Comments-only input

- **Script Execution (2 tests)**
  - Simple script execution
  - Scripts with conditionals

- **Command Substitution (3 tests)**
  - `-c` flag with simple commands
  - `-c` flag with builtins
  - Multiple commands with `-c`

- **Exit Codes (4 tests)**
  - Success exit codes
  - Variable script execution
  - Conditional operators (`&&`, `||`)

- **Redirection (3 tests)**
  - Output redirection (`>`)
  - Append redirection (`>>`)
  - Input redirection (`<`)

- **Pipelines (3 tests)**
  - Simple pipelines (`echo | cat`)
  - Pipelines with grep
  - Multi-stage pipelines

- **Signal Handling (2 tests)**
  - Background job syntax
  - Quick process termination

- **Stdin from File (2 tests)**
  - Commands from file via stdin
  - Scripts with comments via stdin

- **Builtin Commands (3 tests)**
  - `cd`, `echo`, `cat` builtins

- **Error Handling (3 tests)**
  - Command not found handling
  - Continue after failed commands
  - Parse error handling

### 2. Non-TTY Tests (`non_tty_test.sh`)

Tests scenarios where stdin is not a TTY (20 tests):

- Piped input to rush
- Multiple commands via pipe
- Stdin redirection from files
- Here-document simulation
- Empty and whitespace-only input
- Long input streams (50+ commands)
- Builtin and external commands via pipe
- Pipelines in non-TTY mode
- Redirection in non-TTY mode
- Cron job simulation
- CI/CD pipeline simulation
- EOF handling
- Rapid input streams

### 3. Signal Handling Tests (`signal_test.sh`)

Tests signal handling and process cleanup (15 tests):

- Basic process termination
- Timeout handling
- SIGTERM handling
- SIGINT handling (Ctrl-C simulation)
- SIGKILL handling
- Clean exit without signals
- Background process handling
- Nested process termination
- Rapid start-stop cycles
- Signal during pipeline execution
- Process cleanup verification
- Sequential signals
- Signal during script execution

### 4. Redirection Tests (`redirection_test.sh`)

Comprehensive file redirection testing (20 tests):

- Simple output redirection (`>`)
- Overwrite with redirection
- Append redirection (`>>`)
- Multiple appends
- Input redirection (`<`)
- Sequential redirects to multiple files
- Redirect with builtin commands
- Redirect with external commands
- Redirect empty output
- Redirect with spaces in filenames
- Chain of redirects
- Redirect pwd output
- Redirect after pipeline
- Multiple lines redirect
- Redirect to /dev/null
- Large output redirect
- Redirect with conditionals

### 5. Pipeline Tests (`pipeline_test.sh`)

Pipeline functionality testing (20 tests):

- Simple pipeline (`echo | cat`)
- Pipeline with grep
- Pipeline with builtin cat
- Multi-line through pipeline
- Three-stage pipelines
- Pipeline with wc
- Pipeline data integrity
- Empty input through pipeline
- Pipeline with head/tail
- Pipeline with sort/uniq
- Pipeline exit codes
- Pipeline with output redirect
- Pipeline from file input
- Complex pipeline chains
- Pipeline with tr/cut
- Pipeline preserving newlines
- Pipeline with conditionals

### 6. Job Control Tests (`job_control_test.sh`)

Job control and background jobs testing (20 tests):

- Background job syntax (`&`)
- Quick background jobs
- Foreground job completion
- Sequential foreground jobs
- Job with stdout
- Job exit codes
- Sequential job execution
- Job with pipeline
- Job with redirection
- Job with conditional
- Long-running foreground jobs
- Job completion checks
- Multiple quick jobs
- Job with environment/variables
- Job isolation
- Job with builtins/external commands
- Process cleanup after jobs
- Job error handling
- Job state consistency

## CI/CD Integration

The integration tests are automatically run in GitHub Actions CI on every push and pull request.

### GitHub Actions Workflow

File: `.github/workflows/integration-tests.yml`

```yaml
name: Integration Tests

on:
  push:
    branches: [ master, main ]
  pull_request:
    branches: [ master, main ]

jobs:
  integration-tests:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]

    steps:
    - uses: actions/checkout@v4
    - name: Build rush (release)
      run: cargo build --release
    - name: Run integration tests
      run: ./tests/integration/run_all.sh
```

## Test Fixtures

Test fixtures are simple shell scripts in `tests/fixtures/` that provide consistent test scenarios:

- `simple_script.sh`: Basic script with echo and pwd
- `exit_code_script.sh`: Tests different exit codes
- `pipeline_script.sh`: Pipeline scenarios with file operations
- `redirection_script.sh`: Redirection scenarios
- `conditional_script.sh`: Conditional execution tests
- `variable_script.sh`: Variable assignment and expansion

## Test Output

Each test suite provides colored output:
- **GREEN**: Test passed
- **RED**: Test failed
- **YELLOW**: Warnings or notes

Example output:
```
======================================
Rush Shell Integration Tests
======================================

Section 1: Non-Interactive Mode Tests
======================================
Testing: Piped echo command ... PASS
Testing: Multiple piped commands ... PASS
Testing: pwd command ... PASS

======================================
Test Summary
======================================
Tests Passed: 27
Tests Failed: 3
Total Tests:  30
```

## Test Requirements

### Prerequisites

1. **Rush binary**: Tests require the release binary
   ```bash
   cargo build --release
   ```

2. **Executable permissions**: Test scripts must be executable
   ```bash
   chmod +x tests/integration/*.sh tests/fixtures/*.sh
   ```

3. **Timeout command**: For preventing hanging tests
   - Available on Linux and macOS by default

### Environment

Tests are designed to work on:
- **Linux** (Ubuntu, Debian, etc.)
- **macOS** (tested on macOS 10.15+)
- **CI environments** (GitHub Actions)

## Writing New Tests

### Test Structure

```bash
#!/bin/bash

TESTS_PASSED=0
TESTS_FAILED=0

# Test case
test_case "Description of test"
OUTPUT=$(./target/release/rush -c "command" 2>&1)
assert_contains "$OUTPUT" "expected" "Test name"

# Summary
echo "Tests Passed: $TESTS_PASSED"
echo "Tests Failed: $TESTS_FAILED"
exit $([[ $TESTS_FAILED -eq 0 ]] && echo 0 || echo 1)
```

### Assertion Helpers

```bash
# Check if command succeeded ($? == 0)
assert_success "Test name"

# Check specific exit code
assert_exit_code <expected> <actual> "Test name"

# Check if output contains string
assert_contains "$output" "substring" "Test name"

# Check if output doesn't contain string
assert_not_contains "$output" "substring" "Test name"
```

## Debugging Tests

### Run with Trace Mode

```bash
bash -x ./tests/integration/login_shell_test.sh
```

### Run Individual Test

```bash
# Extract and run single test
./target/release/rush -c "echo test" 2>&1
```

### Check Test Output

```bash
# Save output for inspection
./tests/integration/pipeline_test.sh > test_output.txt 2>&1
```

## Known Issues

Some tests may have edge cases or platform-specific behaviors:

1. **Signal handling**: Tests involving SIGINT/SIGTERM may behave differently on different platforms
2. **Background jobs**: Job control requires terminal support
3. **Timeout command**: Behavior varies between GNU and BSD versions

## Future Improvements

Potential areas for expansion:

1. **Performance tests**: Measure execution speed
2. **Stress tests**: High-load scenarios
3. **Memory tests**: Memory usage verification
4. **Concurrency tests**: Parallel execution
5. **Security tests**: Privilege and permission handling
6. **Cross-platform tests**: Windows support (WSL)

## Related Documentation

- [Command History Implementation](./command-history.md)
- [Tab Completion](./tab-completion.md)
- [Context Detection](./context-detection.md)
- [Testing Guide](../TESTING_GUIDE.md)

## Maintenance

Tests should be updated when:
- New features are added
- Bugs are fixed
- Behavior changes
- New edge cases are discovered

Run tests before committing:
```bash
cargo test && ./tests/integration/run_all.sh
```
