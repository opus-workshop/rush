#!/bin/bash
# Sample bash script to test the compatibility analyzer

# Variable assignment
name="Alice"
count=0

# Export to environment
export GREETING="Hello"

# Function definition using POSIX syntax
greet() {
    local message="$1"
    echo "$message $name"
}

# Array variables (Bash-specific)
fruits=("apple" "banana" "cherry")

# For loop (POSIX)
for fruit in "${fruits[@]}"; do
    echo "Fruit: $fruit"
done

# While loop (POSIX)
while [ $count -lt 3 ]; do
    echo "Count: $count"
    ((count++))
done

# If statement (POSIX)
if [ -n "$name" ]; then
    echo "Name is set"
else
    echo "Name is not set"
fi

# Command substitution (POSIX)
date_now=$(date +"%Y-%m-%d")
echo "Today is: $date_now"

# Pipes and redirects
echo "Testing pipes" | cat > output.txt 2>&1

# Here document (POSIX)
cat << EOF
This is a heredoc
It spans multiple lines
EOF

# Process substitution (Bash-specific)
diff <(sort file1.txt) <(sort file2.txt) 2>/dev/null || true

# Arithmetic expansion (Bash-specific)
result=$((10 + 5))
echo "Result: $result"

# Test operator with regex (Bash-specific)
if [[ "$name" =~ ^A ]]; then
    echo "Name starts with A"
fi

# Associative array (Bash-specific)
declare -A config
config[host]="localhost"
config[port]="8080"

# Background execution
sleep 10 &
sleep 5 &

# Conditional operators
true && echo "Success" || echo "Failure"

# Unset variable
unset unused_var

# Function call with arguments
greet "Hi"
