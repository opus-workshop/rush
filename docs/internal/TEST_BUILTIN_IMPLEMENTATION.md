# Test and [ Builtins Implementation

## Overview

The `test` and `[` builtins have been fully implemented for the Rush shell, providing comprehensive file, string, and numeric testing capabilities compatible with POSIX shell standards.

## Implementation Location

- **File**: `/Users/asher/knowledge/rush/src/builtins/test.rs`
- **Registration**: `/Users/asher/knowledge/rush/src/builtins/mod.rs` (lines 59-60)

## Features Implemented

### File Tests
- `-e` - File exists (any type)
- `-f` - Regular file exists
- `-d` - Directory exists
- `-r` - File is readable
- `-w` - File is writable
- `-x` - File is executable
- `-s` - File exists and is non-empty (size > 0)

### String Tests
- `-z` - String is empty (zero length)
- `-n` - String is non-empty
- `=` or `==` - Strings are equal
- `!=` - Strings are not equal

### Numeric Tests
- `-eq` - Numbers are equal
- `-ne` - Numbers are not equal
- `-lt` - Less than
- `-le` - Less than or equal
- `-gt` - Greater than
- `-ge` - Greater than or equal

### Boolean Operators
- `!` - Logical NOT (negation)
- `-a` - Logical AND (higher precedence)
- `-o` - Logical OR (lower precedence)

## Key Implementation Details

### 1. Exit Codes
- Returns exit code `0` for true conditions
- Returns exit code `1` for false conditions
- Follows standard POSIX shell conventions

### 2. Bracket Builtin
- `[` requires a closing `]` as the last argument
- The closing `]` is validated and then stripped before evaluation
- Returns error if `]` is missing

### 3. Path Resolution
- File paths are resolved relative to the current working directory
- Absolute paths are used as-is
- Uses Runtime's `get_cwd()` for proper context

### 4. Expression Evaluation
- Handles operator precedence: `!` > `-a` > `-o`
- Supports complex expressions with multiple operators
- Single argument test returns true if string is non-empty

### 5. Unix Permissions
- Uses Unix file permissions API (`PermissionsExt`)
- Checks actual file mode bits for `-r`, `-w`, `-x`
- Provides fallback behavior for non-Unix systems

## Architecture

```
builtin_test() / builtin_bracket()
    ↓
evaluate_test()
    ↓
evaluate_expression()
    ↓
    ├── evaluate_unary()  (file tests, string tests)
    ├── evaluate_binary() (comparisons)
    └── recursive calls for ! -a -o operators
```

## Usage Examples

### File Tests
```bash
# Check if file exists
[ -f Cargo.toml ] && echo "File exists"

# Check if directory exists
[ -d src ] && echo "Directory exists"

# Check if file is readable and writable
[ -r file.txt -a -w file.txt ] && echo "Can read and write"

# Check if file has content
[ -s data.txt ] && echo "File is not empty"
```

### String Tests
```bash
# Check if string is empty
[ -z "$VAR" ] && echo "Variable is empty"

# Check if string is not empty
[ -n "$VAR" ] && echo "Variable has value"

# String comparison
[ "$NAME" = "Alice" ] && echo "Hello Alice"
```

### Numeric Tests
```bash
# Check if numbers are equal
[ 5 -eq 5 ] && echo "Equal"

# Range check
[ $COUNT -gt 0 -a $COUNT -lt 100 ] && echo "In range"

# Compare values
[ $A -lt $B ] && echo "A is less than B"
```

### Boolean Operations
```bash
# Negation
[ ! -f missing.txt ] && echo "File does not exist"

# AND operation
[ -f file.txt -a -r file.txt ] && echo "File exists and is readable"

# OR operation
[ -f config.yaml -o -f config.yml ] && echo "Config file found"
```

## Test Coverage

The implementation includes 13 comprehensive unit tests covering:
- String empty/non-empty tests
- String equality/inequality
- All numeric comparison operators
- File existence, type, and attributes
- File size (non-empty) checks
- Negation operator
- Boolean AND/OR operators
- Bracket builtin validation
- Single argument behavior
- Edge cases (missing files, empty files, etc.)

All tests pass successfully:
```
test result: ok. 13 passed; 0 failed; 0 ignored
```

## Performance Characteristics

- **Fast**: Direct system calls for file operations
- **Efficient**: Short-circuit evaluation for boolean operators
- **Minimal allocations**: Uses string slices where possible
- **Path caching**: Resolves paths once per operation

## Compatibility Notes

1. **POSIX Compliance**: Follows POSIX test command specification
2. **Bash Compatibility**: Supports `==` as alias for `=`
3. **Unix-focused**: File permission checks are Unix-specific but degrade gracefully
4. **Operator Precedence**: Matches standard shell behavior (`!` > `-a` > `-o`)

## Known Limitations

1. The shell's tokenizer may require quotes around filenames with special characters
2. Some complex expressions with `!=` may trigger tokenizer issues (shell parser limitation, not builtin issue)
3. Symbolic link tests (`-L`, `-h`) are not yet implemented
4. Advanced file tests (`-b`, `-c`, `-p`, `-S`, `-t`) are not yet implemented

## Future Enhancements

Potential additions for full bash compatibility:
- `-L` or `-h` - Symbolic link test
- `-nt` / `-ot` - File modification time comparisons
- `-ef` - Same file test (hard links)
- `<` / `>` - Lexicographic string comparison
- `\(` / `\)` - Expression grouping
- Extended test `[[` builtin with regex support

## Testing

Run unit tests:
```bash
cargo test test:: --lib
```

Run integration tests:
```bash
./test_test_builtin.sh
```

## References

- Implementation: `src/builtins/test.rs`
- Registration: `src/builtins/mod.rs`
- Test suite: `src/builtins/test.rs` (mod tests)
- POSIX spec: IEEE Std 1003.1-2017 (test utility)
