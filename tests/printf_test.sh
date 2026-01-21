#!/usr/bin/env bash
# Test script for printf builtin
# This script tests various printf functionality

echo "Testing printf builtin..."

# Test 1: Simple string formatting
result=$(printf "Hello %s" "World")
expected="Hello World"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 1: Simple string formatting passed"
else
    echo "✗ Test 1 failed: expected '$expected', got '$result'"
fi

# Test 2: Decimal formatting
result=$(printf "Count: %d" 42)
expected="Count: 42"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 2: Decimal formatting passed"
else
    echo "✗ Test 2 failed: expected '$expected', got '$result'"
fi

# Test 3: Float with precision
result=$(printf "Price: %.2f" 3.14159)
expected="Price: 3.14"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 3: Float with precision passed"
else
    echo "✗ Test 3 failed: expected '$expected', got '$result'"
fi

# Test 4: Hex formatting
result=$(printf "Hex: %x" 255)
expected="Hex: ff"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 4: Hex formatting passed"
else
    echo "✗ Test 4 failed: expected '$expected', got '$result'"
fi

# Test 5: Octal formatting
result=$(printf "Octal: %o" 255)
expected="Octal: 377"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 5: Octal formatting passed"
else
    echo "✗ Test 5 failed: expected '$expected', got '$result'"
fi

# Test 6: Width formatting
result=$(printf "%10s" "test")
expected="      test"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 6: Width formatting passed"
else
    echo "✗ Test 6 failed: expected '$expected', got '$result'"
fi

# Test 7: Left alignment
result=$(printf "%-10s" "test")
expected="test      "
if [ "$result" = "$expected" ]; then
    echo "✓ Test 7: Left alignment passed"
else
    echo "✗ Test 7 failed: expected '$expected', got '$result'"
fi

# Test 8: Escape sequences
result=$(printf "Line 1\nLine 2")
expected="Line 1
Line 2"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 8: Escape sequences passed"
else
    echo "✗ Test 8 failed"
fi

# Test 9: No automatic newline
result=$(printf "Hello")
expected="Hello"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 9: No automatic newline passed"
else
    echo "✗ Test 9 failed: expected '$expected', got '$result'"
fi

# Test 10: Multiple arguments
result=$(printf "Name: %s, Age: %d" "Alice" 30)
expected="Name: Alice, Age: 30"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 10: Multiple arguments passed"
else
    echo "✗ Test 10 failed: expected '$expected', got '$result'"
fi

# Test 11: Reuse format
result=$(printf "%s\n" one two three)
expected="one
two
three
"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 11: Reuse format passed"
else
    echo "✗ Test 11 failed"
fi

# Test 12: Mixed formats
result=$(printf "Hex: %x, Octal: %o, Decimal: %d\n" 255 255 255)
expected="Hex: ff, Octal: 377, Decimal: 255
"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 12: Mixed formats passed"
else
    echo "✗ Test 12 failed: expected '$expected', got '$result'"
fi

# Test 13: Percent escape
result=$(printf "100%% complete\n")
expected="100% complete
"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 13: Percent escape passed"
else
    echo "✗ Test 13 failed: expected '$expected', got '$result'"
fi

# Test 14: Aligned columns
result=$(printf "%-20s %10.2f\n" "Apple" 1.99)
expected="Apple                       1.99
"
if [ "$result" = "$expected" ]; then
    echo "✓ Test 14: Aligned columns passed"
else
    echo "✗ Test 14 failed: expected '$expected', got '$result'"
fi

echo ""
echo "All tests completed!"
