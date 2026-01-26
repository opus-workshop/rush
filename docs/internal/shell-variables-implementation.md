# Shell Variables Implementation (rush-dgr.28)

## Overview
Implementation of POSIX standard shell variables for Rush shell.

## Variables Implemented

### 1. SHELL
- **Description**: Path to the rush executable
- **Location**: Set in `init_runtime_variables()` in `src/main.rs`
- **Behavior**: Initialized from environment variable `SHELL` which is set to current executable path in `init_environment_variables()`

### 2. PPID
- **Description**: Parent process ID
- **Location**: Set in `init_runtime_variables()` in `src/main.rs`
- **Behavior**:
  - On Unix systems, uses `std::os::unix::process::parent_id()`
  - Marked as readonly to prevent modification
  - Attempting to set PPID will result in "readonly variable" error

### 3. SHLVL
- **Description**: Shell nesting level
- **Location**:
  - Initial set in `init_runtime_variables()` in `src/main.rs`
  - Incremented in `execute_subshell()` in `src/executor/mod.rs`
- **Behavior**:
  - Reads from environment variable `SHLVL`, defaults to 0
  - Increments by 1 on shell startup
  - Increments by 1 in subshells (commands in parentheses)
  - Environment variable is also updated so child processes inherit correct value

### 4. PWD
- **Description**: Current working directory
- **Location**:
  - Initial set in `init_runtime_variables()` in `src/main.rs`
  - Updated in `builtin_cd()` in `src/builtins/mod.rs`
- **Behavior**:
  - Initialized to current working directory on startup
  - Updated every time `cd` command is executed
  - Always contains absolute path

### 5. OLDPWD
- **Description**: Previous working directory
- **Location**: Updated in `builtin_cd()` in `src/builtins/mod.rs`
- **Behavior**:
  - Set to previous value of PWD before changing directory
  - Used by `cd -` to return to previous directory
  - Not set until first `cd` command is executed

## Special Functionality

### cd -
The `cd -` command switches to the directory stored in `OLDPWD`:
- Prints the target directory to stdout (bash-compatible behavior)
- Returns error if OLDPWD is not set
- Updates PWD and OLDPWD as normal cd would

## Files Modified

1. **src/main.rs**
   - `init_runtime_variables()`: Added initialization of SHELL, PPID, SHLVL, PWD
   - `run_script()`: Added calls to initialize environment and runtime variables

2. **src/builtins/mod.rs**
   - `builtin_cd()`: Updated to set PWD and OLDPWD, implement cd -

3. **src/executor/mod.rs**
   - `execute_subshell()`: Added SHLVL increment for subshell execution

4. **tests/shell_variables_tests.rs** (new file)
   - Comprehensive integration tests for all shell variables
   - Tests for SHELL, PPID, SHLVL, PWD, OLDPWD
   - Tests for cd - functionality
   - Tests for PPID readonly behavior
   - Tests for SHLVL in nested subshells

## Testing

Tests are located in `tests/shell_variables_tests.rs` and include:
- `test_shell_variable_set`: Verify SHELL is set correctly
- `test_ppid_variable`: Verify PPID is a valid number
- `test_ppid_readonly`: Verify PPID cannot be modified
- `test_shlvl_increments`: Verify SHLVL starts at 1
- `test_shlvl_increments_in_subshell`: Verify SHLVL=2 in subshell
- `test_shlvl_nested_subshells`: Verify SHLVL=3 in nested subshell
- `test_pwd_variable`: Verify PWD is set to current directory
- `test_pwd_updates_with_cd`: Verify PWD updates when changing directory
- `test_oldpwd_tracks_previous_directory`: Verify OLDPWD contains previous dir
- `test_cd_dash_uses_oldpwd`: Verify cd - switches to OLDPWD
- `test_cd_dash_prints_directory`: Verify cd - prints target directory
- `test_cd_dash_without_oldpwd`: Verify cd - errors without OLDPWD
- `test_oldpwd_chain`: Verify OLDPWD updates through multiple cd commands
- `test_pwd_stays_in_sync`: Verify PWD matches pwd builtin output
- `test_all_standard_variables_present`: Verify all variables are set

## POSIX Compliance

This implementation satisfies POSIX requirements for:
- SHELL: pathname of the shell ✓
- PWD: current working directory ✓
- OLDPWD: previous working directory ✓
- PPID: parent process ID (readonly) ✓
- SHLVL: shell nesting level (common extension, not strictly POSIX) ✓

## Known Limitations

1. On non-Unix platforms, PPID may not be set (requires platform-specific implementation)
2. SHLVL in background jobs is not yet tested

## Future Enhancements

1. Ensure SHLVL is properly inherited by background jobs
2. Add support for CD_PATH environment variable
3. Consider adding OLDPWD to exported variables
