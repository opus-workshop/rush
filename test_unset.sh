#!/usr/bin/env bash
# Manual test script for unset builtin
# This can be run once the codebase compiles

echo "Testing unset builtin..."

# Test 1: Basic variable unset
export TEST_VAR=hello
echo "Before: TEST_VAR=$TEST_VAR"
unset TEST_VAR
echo "After: TEST_VAR=$TEST_VAR (should be empty)"

# Test 2: Multiple variables
a=1 b=2 c=3
echo "Before: a=$a b=$b c=$c"
unset a b c
echo "After: a=$a b=$b c=$c (should all be empty)"

# Test 3: Explicit -v flag
export EXPLICIT=value
unset -v EXPLICIT
echo "Explicit unset: EXPLICIT=$EXPLICIT (should be empty)"

# Test 4: Unset function
myfunc() { echo "Hello from function"; }
myfunc
unset -f myfunc
myfunc 2>&1 | head -1  # Should error

# Test 5: Nonexistent variable (should not error)
unset NONEXISTENT_VAR && echo "No error on nonexistent (correct)"

# Test 6: Try to unset special variable (should error)
unset '?' 2>&1 | grep "cannot unset" && echo "Protected special variables (correct)"

echo "All tests completed!"
