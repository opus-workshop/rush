#!/usr/bin/env bash
# Test script with conditional execution
echo "Testing conditionals"
true && echo "AND succeeded"
false && echo "This should not print"
false || echo "OR fallback worked"
echo "Conditionals completed"
