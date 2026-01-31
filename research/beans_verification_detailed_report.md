# Bean Verification Report - Rush Project

**Generated:** Saturday, January 31, 2026 09:46 PST
**Repository:** /Users/asher/tt/rush

## Executive Summary

Verification was run on ALL beans in the rush project that have verify commands defined. The results show:

- **Total beans in project:** 192
- **Beans with verify commands:** 143 (including non-numeric IDs like "15.1", "15.2", etc.)
- **Numeric beans with verify commands:** 134
- **Passed verification:** 0 beans
- **Failed verification:** 134 beans

## Key Finding

All 134 numeric beans with verify commands are currently **FAILING** because the underlying codebase has **7 failing unit tests**. These tests must be fixed before any bean can pass verification.

## Failing Tests

The following tests are preventing all beans from passing their verify gates:

1. `builtins::printf::tests::test_printf_aligned_columns` - Column alignment issue in printf output
2. `completion::tests::test_path_completion_current_dir` - Path completion missing expected results
3. `executor::error_formatter::tests::test_format_error_with_help_text` - Help text formatting not working
4. `executor::tests::test_for_loop_nested` - Nested for loop whitespace handling
5. `executor::tests::test_while_nested` - Nested while loop whitespace handling
6. `executor::value::tests::test_json_roundtrip` - JSON serialization issue (missing field)
7. `parser::tests::test_parse_while_loop_with_newlines` - Parser issue with newlines in conditions

## Failed Bean IDs (count=134)

```
3, 9, 10, 14, 15, 16, 17, 20, 22, 24, 25, 26, 27, 28, 29, 30, 31, 33,
36, 37, 38, 39, 40, 41, 42, 44, 45, 46, 47, 48, 53, 54, 55, 56, 58, 60,
61, 62, 64, 65, 66, 68, 69, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82,
83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 103, 104,
105, 106, 107, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121,
122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136,
137, 138, 139, 140, 141, 143, 145, 146, 162, 163, 164, 167, 169, 170, 171,
172, 173, 174, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190,
191, 192
```

## Beans WITHOUT Verify Commands

The following 58 beans do NOT have verify commands defined:
- 1, 2, 4, 5, 6, 7, 8, 11, 12, 13, 18, 19, 21, 23, 32, 34, 35, 43, 49, 50, 51, 52, 57, 59, 63, 67, 70, 71, 87, 99, 100, 101, 102, 108, 109, 141, 142, 144, 147-161, 165, 166, 168, and non-numeric beans (15.1, 15.2, 24.1, 28.1-28.4, rush-hn2.1, rush-hn3.1, rush-hn5.1, rush-hn6.4)

These beans cannot be verified and closed without first adding verify commands.

## Recommendations

1. **Priority 1:** Fix the 7 failing tests listed above
   - These are blocking ALL bean verifications
   - Once fixed, re-run verification to identify which beans actually pass

2. **Priority 2:** Add verify commands to beans without them
   - 58 numeric beans are missing verify commands
   - Determine appropriate verification steps for each

3. **Monitor Verification Progress:**
   - After fixing tests, run: `bn verify <bean_id>` for each bean
   - Use the formatted results to identify closeable beans
   - Close verified beans with: `bn close <bean_id>`

## Files

- **Results file:** `/Users/asher/tt/rush/research/beans_verification_results.txt`
- **Detailed report:** This file

## Command to Reproduce

```bash
# Verify all numeric beans with verify commands
for id in $(grep -l "^verify:" /Users/asher/tt/rush/.beans/*.yaml | \
            xargs basename -a | sed 's/.yaml$//' | \
            grep -E '^[0-9]+$' | sort -n); do
  echo "Verifying bean $id..."
  bn verify "$id"
done
```
