#!/bin/bash

echo "=== Testing Rush test and [ builtins ==="
echo ""

# Use the release binary
RUSH="./target/release/rush"

echo "String Tests:"
$RUSH -c 'test -z "" && echo "  -z (empty): PASS" || echo "  -z (empty): FAIL"'
$RUSH -c 'test -n "hello" && echo "  -n (non-empty): PASS" || echo "  -n (non-empty): FAIL"'
$RUSH -c 'test "hello" = "hello" && echo "  = (equal): PASS" || echo "  = (equal): FAIL"'
$RUSH -c 'test "hello" != "world" && echo "  != (not equal): PASS" || echo "  != (not equal): FAIL"'

echo ""
echo "Numeric Tests:"
$RUSH -c 'test 5 -eq 5 && echo "  -eq (equal): PASS" || echo "  -eq (equal): FAIL"'
$RUSH -c 'test 5 -ne 3 && echo "  -ne (not equal): PASS" || echo "  -ne (not equal): FAIL"'
$RUSH -c 'test 3 -lt 5 && echo "  -lt (less than): PASS" || echo "  -lt (less than): FAIL"'
$RUSH -c 'test 5 -le 5 && echo "  -le (less or equal): PASS" || echo "  -le (less or equal): FAIL"'
$RUSH -c 'test 5 -gt 3 && echo "  -gt (greater than): PASS" || echo "  -gt (greater than): FAIL"'
$RUSH -c 'test 5 -ge 5 && echo "  -ge (greater or equal): PASS" || echo "  -ge (greater or equal): FAIL"'

echo ""
echo "File Tests:"
$RUSH -c 'test -e Cargo.toml && echo "  -e (exists): PASS" || echo "  -e (exists): FAIL"'
$RUSH -c 'test -f Cargo.toml && echo "  -f (regular file): PASS" || echo "  -f (regular file): FAIL"'
$RUSH -c 'test -d src && echo "  -d (directory): PASS" || echo "  -d (directory): FAIL"'
$RUSH -c 'test -r Cargo.toml && echo "  -r (readable): PASS" || echo "  -r (readable): FAIL"'
$RUSH -c 'test -s Cargo.toml && echo "  -s (non-empty): PASS" || echo "  -s (non-empty): FAIL"'

echo ""
echo "Bracket [ ] Syntax:"
$RUSH -c '[ 5 -gt 3 ] && echo "  Numeric comparison: PASS" || echo "  Numeric comparison: FAIL"'
$RUSH -c '[ "hello" = "hello" ] && echo "  String equality: PASS" || echo "  String equality: FAIL"'
$RUSH -c '[ -f Cargo.toml ] && echo "  File test: PASS" || echo "  File test: FAIL"'

echo ""
echo "Boolean Operators:"
$RUSH -c 'test -n "hello" -a 5 -eq 5 && echo "  -a (and): PASS" || echo "  -a (and): FAIL"'
$RUSH -c 'test -z "hello" -o 5 -eq 5 && echo "  -o (or): PASS" || echo "  -o (or): FAIL"'

echo ""
echo "=== All tests completed! ==="
