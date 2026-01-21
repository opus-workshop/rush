#!/usr/bin/env rush

# Test string operations
test -z "" && echo "Empty string test: PASS" || echo "Empty string test: FAIL"
test -n "hello" && echo "Non-empty string test: PASS" || echo "Non-empty string test: FAIL"
test "hello" = "hello" && echo "String equality test: PASS" || echo "String equality test: FAIL"
test "hello" != "world" && echo "String inequality test: PASS" || echo "String inequality test: FAIL"

# Test numeric comparisons
test 5 -eq 5 && echo "Numeric equality test: PASS" || echo "Numeric equality test: FAIL"
test 3 -lt 5 && echo "Less than test: PASS" || echo "Less than test: FAIL"
test 5 -gt 3 && echo "Greater than test: PASS" || echo "Greater than test: FAIL"
test 5 -ge 5 && echo "Greater or equal test: PASS" || echo "Greater or equal test: FAIL"

# Test file operations
test -f test_script.sh && echo "Regular file test: PASS" || echo "Regular file test: FAIL"
test -d . && echo "Directory test: PASS" || echo "Directory test: FAIL"
test -e test_script.sh && echo "File exists test: PASS" || echo "File exists test: FAIL"
test -r test_script.sh && echo "Readable test: PASS" || echo "Readable test: FAIL"

# Test [ bracket syntax
[ -f test_script.sh ] && echo "Bracket syntax test: PASS" || echo "Bracket syntax test: FAIL"
[ 10 -gt 5 ] && echo "Bracket numeric test: PASS" || echo "Bracket numeric test: FAIL"

# Test negation
! test -z "hello" && echo "Negation test: PASS" || echo "Negation test: FAIL"

# Test boolean operators
test -n "hello" -a 5 -eq 5 && echo "AND operator test: PASS" || echo "AND operator test: FAIL"
test -z "hello" -o 5 -eq 5 && echo "OR operator test: PASS" || echo "OR operator test: FAIL"

echo ""
echo "All tests completed!"
