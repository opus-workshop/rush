# Non-TTY Mode Testing Documentation

## Overview

The `tests/non_tty_tests.rs` file contains comprehensive integration tests for rush shell's non-TTY mode support. These tests ensure that rush works correctly when used in automated environments, scripts, and non-interactive contexts.

## Test Categories

### 1. Piped Input Tests

Tests that verify rush can receive commands via stdin pipe:

- **test_piped_input_single_command**: Basic piping of a single command
  ```bash
  echo "pwd" | rush
  ```

- **test_piped_input_echo_command**: Piping echo commands
  ```bash
  echo "echo hello world" | rush
  ```

- **test_piped_input_multiple_commands**: Multiple commands piped in sequence
  ```bash
  echo -e "echo first\necho second\necho third" | rush
  ```

- **test_piped_input_with_builtin_commands**: Tests rush's builtin commands via pipe

### 2. Stdin Redirection Tests

Tests that verify rush can read commands from redirected files:

- **test_stdin_redirection_simple_script**: Basic file redirection
  ```bash
  rush < script.sh
  ```

- **test_stdin_redirection_with_comments**: Ensures comments are properly ignored
- **test_stdin_redirection_with_empty_lines**: Verifies empty lines don't cause issues
- **test_stdin_redirection_with_whitespace**: Tests whitespace handling

### 3. Command Substitution Tests

Tests that verify rush works in command substitution scenarios:

- **test_command_substitution_with_c_flag**: Basic command substitution
  ```bash
  result=$(rush -c "echo test")
  ```

- **test_command_substitution_pwd**: Capturing directory output
- **test_command_substitution_cat**: File content capture

### 4. Error Handling Tests

Tests that verify rush handles errors gracefully in non-TTY mode:

- **test_error_handling_failed_command**: Failed commands don't crash rush
- **test_error_handling_multiple_commands_one_fails**: Subsequent commands execute after failures
- **test_error_handling_parse_error**: Parse errors are handled gracefully

### 5. Cron Job Simulation Tests

Tests that simulate real-world cron job scenarios:

- **test_cron_job_scenario_simple**: Basic automated script execution
- **test_cron_job_scenario_with_file_operations**: File manipulation in cron context

### 6. Exit Code Tests

Tests that verify proper exit code handling:

- **test_exit_code_success**: Successful commands return 0
- **test_exit_code_via_stdin**: Exit codes work with stdin input

### 7. Pipeline Tests

Tests that verify pipelines work in non-TTY mode:

- **test_pipeline_via_stdin**: Basic pipeline execution
- **test_complex_pipeline_via_stdin**: Multi-stage pipeline operations

### 8. CI/CD Scenario Tests

Tests that simulate continuous integration/deployment scenarios:

- **test_ci_cd_scenario**: Complete automated testing script

### 9. Edge Case Tests

Tests that verify rush handles unusual inputs:

- **test_empty_input**: Empty stdin doesn't crash
- **test_only_whitespace_input**: Whitespace-only input is handled
- **test_only_comments_input**: Comment-only scripts work

## Running the Tests

### Run all non-TTY tests:
```bash
cargo test --test non_tty_tests
```

### Run a specific test:
```bash
cargo test --test non_tty_tests test_piped_input_single_command
```

### Run tests with output:
```bash
cargo test --test non_tty_tests -- --nocapture
```

### Build the binary first (required):
```bash
cargo build --release
```

## Test Design Principles

1. **Black Box Testing**: Tests spawn rush as a subprocess and test it as a complete system
2. **Realistic Scenarios**: Tests mirror real-world usage patterns (cron, CI/CD, scripting)
3. **Comprehensive Coverage**: Tests cover success paths, error paths, and edge cases
4. **Self-Contained**: Tests create and clean up their own temporary files
5. **Deterministic**: Tests don't rely on external state or timing

## Common Use Cases Covered

### Login Shell Support
Rush can be used as a login shell, reading from `.profile` or initialization scripts:
```bash
rush < ~/.rushrc
```

### Automated Scripts
Rush can execute automation scripts in cron or systemd:
```bash
#!/usr/bin/env rush
echo "Running automated task"
# ... more commands
```

### Command Substitution
Rush can be used in command substitution like other shells:
```bash
current_dir=$(rush -c "pwd")
file_count=$(rush -c "ls | wc -l")
```

### CI/CD Pipelines
Rush can execute test and deployment scripts:
```bash
echo "npm test" | rush
rush < deploy.sh
```

## Test Output Examples

Successful test run:
```
running 26 tests
test test_piped_input_single_command ... ok
test test_piped_input_echo_command ... ok
test test_piped_input_multiple_commands ... ok
...
test result: ok. 26 passed; 0 failed; 0 ignored
```

## Troubleshooting

### Tests fail with "binary not found"
- Ensure you've built the release binary: `cargo build --release`
- Check that `target/release/rush` exists

### Tests timeout
- Non-TTY mode should exit after processing all input
- If tests hang, rush may not be detecting EOF correctly

### Flaky tests
- Tests use `/tmp` for temporary files
- Ensure proper cleanup in each test
- Use unique file names to avoid conflicts

## Future Enhancements

Potential additions to the test suite:

1. **Batch mode tests**: Test processing large batch files
2. **Performance tests**: Benchmark non-TTY mode vs TTY mode
3. **Signal handling tests**: Test SIGTERM, SIGINT in non-TTY contexts
4. **Large input tests**: Test with scripts containing thousands of lines
5. **Binary input tests**: Verify handling of binary data in pipes
6. **Resource limit tests**: Test behavior under resource constraints

## Related Documentation

- [Command History Implementation](./command-history.md)
- [Context Detection](./context-detection.md)
- [C Flag Implementation](./c-flag-implementation.md)
- [Testing Guide](../TESTING_GUIDE.md)
