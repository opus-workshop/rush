# printf Quick Reference

## Syntax

```bash
printf format [arguments...]
```

## Format Specifiers

| Specifier | Type | Example | Output |
|-----------|------|---------|--------|
| `%s` | String | `printf "%s" "hello"` | `hello` |
| `%d` | Decimal | `printf "%d" 42` | `42` |
| `%i` | Integer | `printf "%i" 42` | `42` |
| `%f` | Float | `printf "%f" 3.14` | `3.140000` |
| `%x` | Hex (lower) | `printf "%x" 255` | `ff` |
| `%o` | Octal | `printf "%o" 8` | `10` |

## Modifiers

### Width

```bash
printf "%10s" "test"      # "      test" (right-aligned, width 10)
printf "%5d" 42           # "   42" (right-aligned, width 5)
```

### Precision (floats only)

```bash
printf "%.2f" 3.14159     # "3.14" (2 decimal places)
printf "%.0f" 3.14159     # "3" (0 decimal places)
printf "%.4f" 3.14159     # "3.1416" (4 decimal places)
```

### Width + Precision

```bash
printf "%10.2f" 3.14159   # "      3.14" (width 10, 2 decimals)
```

### Left Alignment

```bash
printf "%-10s" "test"     # "test      " (left-aligned, width 10)
printf "%-5d" 42          # "42   " (left-aligned, width 5)
```

## Escape Sequences

| Sequence | Result |
|----------|--------|
| `\n` | Newline |
| `\t` | Tab |
| `\r` | Carriage return |
| `\\` | Backslash |
| `\'` | Single quote |
| `\"` | Double quote |
| `%%` | Percent sign |

## Examples

### Basic

```bash
printf "Hello, %s!\n" "World"
# Hello, World!

printf "Count: %d\n" 42
# Count: 42

printf "Price: $%.2f\n" 19.99
# Price: $19.99
```

### Tables

```bash
printf "%-15s %8s %10s\n" "Product" "Qty" "Price"
printf "%-15s %8d %10.2f\n" "Apple" 5 1.99
printf "%-15s %8d %10.2f\n" "Orange" 12 0.89
# Product              Qty      Price
# Apple                  5       1.99
# Orange                12       0.89
```

### Multiple Arguments (Format Reuse)

```bash
printf "%s\n" one two three
# one
# two
# three
```

### Number Conversions

```bash
printf "Decimal: %d, Hex: %x, Octal: %o\n" 255 255 255
# Decimal: 255, Hex: ff, Octal: 377
```

### Progress Bar

```bash
printf "[%-50s] %d%%\n" "#####################" 42
# [#####################                             ] 42%
```

### No Newline

```bash
printf "Loading"
printf "."
printf "."
printf ".\n"
# Loading...
```

## Common Patterns

### Aligned Columns

```bash
printf "%-20s: %s\n" "Name" "Alice"
printf "%-20s: %d\n" "Age" 30
printf "%-20s: %.2f\n" "Score" 95.7
# Name                : Alice
# Age                 : 30
# Score               : 95.70
```

### Currency

```bash
printf "$%,.2f\n" 1234.56
# Note: Thousands separator not yet supported
# Output: $1234.56
```

### Padding Numbers

```bash
printf "%05d\n" 42
# Note: Zero-padding not yet supported
# Current output: "   42"
```

### Scientific Notation

```bash
# Not yet supported
# Use %f for now
printf "%f\n" 0.000123
# 0.000123
```

## Tips

1. **Always quote format strings** to prevent shell interpretation:
   ```bash
   printf "%s\n" "value"    # Good
   printf %s\n value        # May fail
   ```

2. **Use `\n` for newlines**, printf doesn't add them automatically:
   ```bash
   printf "Line 1\n"
   printf "Line 2\n"
   ```

3. **Format reuse** is automatic with extra arguments:
   ```bash
   printf "%s=%s\n" key1 val1 key2 val2
   # key1=val1
   # key2=val2
   ```

4. **Invalid numbers default to 0**:
   ```bash
   printf "%d\n" "abc"    # 0
   ```

5. **Missing arguments use defaults**:
   ```bash
   printf "%s %d\n" "test"    # test 0
   ```

## Differences from Echo

| Feature | echo | printf |
|---------|------|--------|
| Automatic newline | Yes | No |
| Format specifiers | No | Yes |
| Width/alignment | No | Yes |
| Type conversion | No | Yes |
| Escape sequences | Limited | Full |

## Error Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | Invalid format specifier or usage error |

## See Also

- Full documentation: `/docs/PRINTF_BUILTIN_IMPLEMENTATION.md`
- Test suite: `/tests/printf_test.sh`
- Source code: `/src/builtins/printf.rs`
