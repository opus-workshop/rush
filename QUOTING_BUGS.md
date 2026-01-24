# Quoting Behavior Audit Results

## Summary

Comprehensive quoting tests were added in `tests/quoting_tests.rs` with 38 test cases covering all POSIX shell quoting edge cases. Test results: **19 passed, 19 failed**.

## Test Results

**Passed (19 tests):**
- All single quote literal preservation tests (6 tests)
- Empty string handling (3 tests)
- Whitespace preservation in quotes (3 tests)
- Quote nesting (2 tests)
- Unquoted variable/command expansion (2 tests)
- Miscellaneous quote behavior (3 tests)

**Failed (19 tests):**
- Variable expansion in double quotes (7 tests)
- Command substitution in double quotes (4 tests)
- Escape sequences in double quotes (5 tests)
- Glob expansion blocking (2 tests)
- Adjacent quoted string concatenation (1 test)

## Bugs Found

### 1. Variable Expansion in Double Quotes [CRITICAL]

**Description**: Variables inside double quotes are treated as literals instead of being expanded.

**Examples**:
```bash
VAR=expanded
echo "value: $VAR"        # Expected: "value: expanded"  | Actual: "value: $VAR"
echo "value: ${VAR}"      # Expected: "value: expanded"  | Actual: "value: ${VAR}"
echo "exit: $?"           # Expected: "exit: 0"          | Actual: Error
EMPTY=""
echo "before:$EMPTY:after" # Expected: "before::after"   | Actual: "before:$EMPTY:after"
```

**POSIX Requirement**: Section 2.2.3 - Double-Quotes must allow parameter expansion.

**Root Cause**: Parser treats all quoted strings as literals (src/parser/mod.rs:273-276):
```rust
Some(Token::String(s)) | Some(Token::SingleQuotedString(s)) => {
    let unquoted = s.trim_matches('"').trim_matches('\'');
    Ok(Argument::Literal(unquoted.to_string()))
}
```

**Failed Tests**:
- test_double_quotes_expand_variable
- test_double_quotes_expand_braced_variable
- test_double_quotes_expand_special_variables
- test_mixed_quoted_arguments
- test_empty_variable_in_quotes
- test_unset_variable_in_quotes
- test_variable_with_special_chars_in_quotes

---

### 2. Command Substitution in Double Quotes [CRITICAL]

**Description**: Command substitutions inside double quotes are treated as literals instead of being executed.

**Examples**:
```bash
echo "result: $(echo test)"           # Expected: "result: test"  | Actual: "result: $(echo test)"
echo "result: `echo test`"            # Expected: "result: test"  | Actual: "result: `echo test`"
echo "outer: $(echo $(echo inner))"   # Expected: "outer: inner"  | Actual: "outer: $(echo $(echo inner))"
```

**POSIX Requirement**: Section 2.2.3 - Double-Quotes must allow command substitution.

**Root Cause**: Same as Bug #1 - parser doesn't process content inside double quotes.

**Failed Tests**:
- test_double_quotes_expand_command_substitution
- test_command_substitution_in_double_quotes
- test_backtick_substitution_in_double_quotes
- test_nested_command_substitution_with_quotes

---

### 3. Escape Sequences in Double Quotes [CRITICAL]

**Description**: Backslash escapes inside double quotes are not processed.

**Examples**:
```bash
echo "quote: \"hello\""    # Expected: 'quote: "hello"'  | Actual: 'quote: \"hello\"'
echo "\$VAR"               # Expected: "$VAR"            | Actual: "\$VAR"
echo "back\\slash"         # Expected: "back\slash"      | Actual: "back\\slash"
echo "\`backtick\`"        # Expected: "`backtick`"      | Actual: "\`backtick\`"
VAR=value
echo "\\$VAR=$$VAR"        # Expected: "$VAR=value"      | Actual: "\\$VAR=$VAR"
```

**POSIX Requirement**: Section 2.2.3 - Backslash inside double quotes shall retain special meaning only when followed by: $ ` " \ or newline.

**Root Cause**: No escape sequence processing in parser.

**Failed Tests**:
- test_double_quotes_escape_dollar
- test_double_quotes_escape_double_quote
- test_double_quotes_escape_backtick
- test_double_quotes_escape_backslash
- test_quote_escaping_with_variable

---

### 4. Glob Expansion in Quoted Strings [MINOR]

**Description**: Glob patterns inside both single and double quotes are being expanded to filenames.

**Examples**:
```bash
echo "*.txt"    # Expected: "*.txt"              | Actual: "while_loop_patch.txt" (actual file)
echo '*.rs'     # Expected: "*.rs"               | Actual: Error (tries to glob)
```

**POSIX Requirement**: Section 2.2 - Quote removal shall be performed before pathname expansion. Quoted characters shall not be subject to pathname expansion.

**Root Cause**: Glob expansion happens after quotes are removed, but before quote context is considered.

**Failed Tests**:
- test_double_quotes_block_glob_expansion
- test_single_quotes_block_glob_expansion

---

### 5. Adjacent Quoted Strings Not Concatenated [MINOR]

**Description**: Adjacent quoted strings aren't concatenated into a single argument.

**Examples**:
```bash
echo 'hello'" world"    # Expected: "hello world"   | Actual: "hello" (only first part)
```

**POSIX Requirement**: Section 2.2.3 - Adjacent strings shall be concatenated.

**Root Cause**: Parser treats each quoted string as a separate token.

**Failed Tests**:
- test_adjacent_quoted_strings

---

## Implementation Recommendations

To fix these bugs, the following changes are needed:

### 1. Lexer Changes (src/lexer/mod.rs)
- Keep the current regex-based tokenization for quoted strings
- Add state tracking for quote context to handle nested quotes

### 2. Parser Changes (src/parser/mod.rs)
- **Line 273-276**: Split handling of `Token::String` and `Token::SingleQuotedString`
  - Single quotes: strip quotes, return Literal (current behavior)
  - Double quotes: parse content, expand variables/commands, process escapes
- Add `parse_double_quoted_string()` method to:
  - Strip outer quotes
  - Process backslash escapes (\$, \`, \", \\, \newline)
  - Identify and expand variables ($VAR, ${VAR}, special vars)
  - Identify and expand command substitutions ($(cmd), `cmd`)
  - Return a potentially complex Argument type or Expression
- Add `parse_adjacent_strings()` to concatenate adjacent quoted/unquoted tokens

### 3. Executor Changes (src/executor/mod.rs)
- Ensure glob expansion respects quote context
- Add quote-aware argument resolution

### 4. AST Changes (src/parser/ast.rs)
- Consider adding quote context to Argument enum:
  ```rust
  pub enum Argument {
      Literal(String),                    // Unquoted
      SingleQuoted(String),               // Single-quoted (no expansion)
      DoubleQuoted(Vec<ArgumentPart>),    // Double-quoted (with expansion)
      // ...
  }

  pub enum ArgumentPart {
      Literal(String),
      Variable(String),
      CommandSubstitution(String),
  }
  ```

### 5. Testing
- All 38 tests in `tests/quoting_tests.rs` should pass
- Run: `cargo test quoting_tests`

---

## Test Coverage

The test suite covers:
- **Single quotes**: Literal preservation of $, \, `, ", whitespace, newlines (6 tests)
- **Double quotes**: Variable expansion, command substitution, escape sequences (13 tests)
- **Whitespace**: Leading/trailing spaces in quotes (3 tests)
- **Empty strings**: "", '', multiple empties (3 tests)
- **Quote nesting**: "...'...'..." and '..."...\''  (3 tests)
- **Glob expansion**: Blocking in quotes (2 tests)
- **Command substitution**: $(cmd), `cmd`, nesting (4 tests)
- **Complex scenarios**: Mixed quotes, adjacency, escaping (4 tests)

---

## POSIX Compliance Status

**Quoting (Section 2.2)**: ❌ **NON-COMPLIANT**
- Single quotes: ✅ Compliant (all tests pass)
- Double quotes: ❌ Non-compliant (expansion/escaping broken)
- Escape characters: ❌ Non-compliant (not processed in double quotes)
- Quote removal: ⚠️  Partial (basic removal works, context awareness missing)

---

## Priority

**CRITICAL**: Bugs #1, #2, #3 (double quote expansion and escaping)
- These are fundamental shell features used in virtually every script
- Breaking these makes the shell unusable for most real-world tasks

**MINOR**: Bugs #4, #5 (glob expansion, adjacent strings)
- Less common edge cases
- Can be worked around with different syntax

---

## References

- POSIX Shell Command Language: https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html
- Section 2.2: Quoting
- Section 2.2.1: Escape Character (Backslash)
- Section 2.2.2: Single-Quotes
- Section 2.2.3: Double-Quotes
- Section 2.6: Word Expansions
