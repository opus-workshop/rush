# Test and [ Builtins - Quick Reference

## Syntax
```bash
test EXPRESSION
[ EXPRESSION ]
```

## File Tests
| Operator | Description | Example |
|----------|-------------|---------|
| `-e file` | File exists (any type) | `[ -e config.txt ]` |
| `-f file` | Regular file exists | `[ -f script.sh ]` |
| `-d path` | Directory exists | `[ -d src/ ]` |
| `-r file` | File is readable | `[ -r data.txt ]` |
| `-w file` | File is writable | `[ -w output.log ]` |
| `-x file` | File is executable | `[ -x run.sh ]` |
| `-s file` | File exists and not empty | `[ -s data.csv ]` |

## String Tests
| Operator | Description | Example |
|----------|-------------|---------|
| `-z str` | String is empty | `[ -z "$var" ]` |
| `-n str` | String is not empty | `[ -n "$name" ]` |
| `str1 = str2` | Strings are equal | `[ "$a" = "$b" ]` |
| `str1 != str2` | Strings are not equal | `[ "$x" != "$y" ]` |

## Numeric Tests
| Operator | Description | Example |
|----------|-------------|---------|
| `n1 -eq n2` | Numbers are equal | `[ $count -eq 5 ]` |
| `n1 -ne n2` | Numbers are not equal | `[ $status -ne 0 ]` |
| `n1 -lt n2` | Less than | `[ $age -lt 18 ]` |
| `n1 -le n2` | Less than or equal | `[ $score -le 100 ]` |
| `n1 -gt n2` | Greater than | `[ $temp -gt 30 ]` |
| `n1 -ge n2` | Greater than or equal | `[ $level -ge 5 ]` |

## Boolean Operators
| Operator | Description | Example |
|----------|-------------|---------|
| `! expr` | Logical NOT | `[ ! -f missing.txt ]` |
| `expr1 -a expr2` | Logical AND | `[ -f file -a -r file ]` |
| `expr1 -o expr2` | Logical OR | `[ -f a.txt -o -f b.txt ]` |

## Common Patterns

### Check file before reading
```bash
[ -f "$config" -a -r "$config" ] && source "$config"
```

### Validate input
```bash
[ -z "$name" ] && echo "Error: name is required" && exit 1
```

### Numeric range check
```bash
[ $value -ge 0 -a $value -le 100 ] && echo "Valid percentage"
```

### Multiple file check
```bash
[ -f config.yaml ] || [ -f config.yml ] && echo "Config found"
```

### Directory creation
```bash
[ ! -d backup ] && mkdir backup
```

### Error handling
```bash
[ $status -ne 0 ] && echo "Command failed" && exit $status
```

## Exit Codes
- `0` - Expression is true
- `1` - Expression is false
- `>1` - Error in expression

## Tips
1. Always quote variables: `[ -z "$var" ]` not `[ -z $var ]`
2. Use `[` with closing `]` for better readability
3. Combine with `&&` and `||` for control flow
4. Operator precedence: `!` > `-a` > `-o`
5. File paths are resolved relative to current directory

## Examples

### Basic file checks
```bash
# Check if file exists before reading
[ -f data.txt ] && cat data.txt

# Create directory if missing
[ ! -d logs ] && mkdir logs

# Check if file is executable
[ -x script.sh ] && ./script.sh
```

### String validation
```bash
# Check if variable is set
[ -n "$USER" ] && echo "Hello $USER"

# Compare strings
[ "$MODE" = "debug" ] && set -x

# Check for empty string
[ -z "$result" ] && echo "No result"
```

### Numeric comparisons
```bash
# Age validation
[ $age -ge 18 ] && echo "Adult"

# Range check
[ $score -gt 0 -a $score -le 100 ] && echo "Valid score"

# Status check
[ $? -eq 0 ] && echo "Success" || echo "Failed"
```

### Complex expressions
```bash
# Multiple conditions
[ -f config.txt -a -r config.txt -a ! -z "$HOME" ] && echo "OK"

# Either/or logic
[ -f .env -o -f .env.local ] && echo "Environment file found"

# Nested logic with negation
[ ! -f lock.pid -o ! -d temp ] && echo "Safe to proceed"
```

## Implementation Details
- **File**: `src/builtins/test.rs`
- **Lines of code**: ~465 lines including tests
- **Test coverage**: 13 unit tests, all passing
- **Dependencies**: Standard library only (fs, path, anyhow)
- **Platform**: Unix-focused with non-Unix fallbacks
