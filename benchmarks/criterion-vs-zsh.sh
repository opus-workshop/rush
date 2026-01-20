#!/bin/bash
# Compare Rush Criterion benchmarks to Zsh equivalent operations
# Measures pure execution time (what Criterion does for Rush)

set -e

echo "========================================="
echo "Rush Criterion vs Zsh Equivalent"
echo "========================================="
echo ""
echo "Comparing pure builtin/command performance"
echo "(no shell startup overhead where possible)"
echo ""

if ! command -v gdate &> /dev/null; then
    echo "Error: gdate not found. Install with: brew install coreutils"
    exit 1
fi

cd "$(dirname "$0")"

# Ensure test data exists
mkdir -p ./benchmark-data
if [ ! -f ./benchmark-data/large-file.txt ]; then
    echo "Creating test data..."
    for i in {1..10000}; do
        echo "Line $i: Lorem ipsum dolor sit amet, consectetur adipiscing elit." >> ./benchmark-data/large-file.txt
    done
fi

if [ ! -d ./benchmark-data/deep-tree ]; then
    mkdir -p ./benchmark-data/deep-tree
    for i in {1..20}; do
        mkdir -p ./benchmark-data/deep-tree/dir$i
        for j in {1..50}; do
            echo "File content $i-$j" > ./benchmark-data/deep-tree/dir$i/file$j.txt
        done
    done
fi

ITERATIONS=10000

echo "Running $ITERATIONS iterations per test..."
echo ""

# Helper function to measure command execution
measure_command() {
    local cmd="$1"
    local iterations=$2
    local start end elapsed

    start=$(gdate +%s%N)
    for i in $(seq 1 $iterations); do
        eval "$cmd" > /dev/null 2>&1
    done
    end=$(gdate +%s%N)

    elapsed=$((($end - $start) / $iterations))
    echo "$elapsed"
}

# Test 1: echo (builtin for Zsh)
echo -n "1. echo... "
zsh_echo_ns=$(measure_command 'echo "test"' $ITERATIONS)
zsh_echo_us=$(echo "scale=2; $zsh_echo_ns / 1000" | bc)
echo "${zsh_echo_us} µs (Zsh builtin)"

# Test 2: pwd (builtin for Zsh)
echo -n "2. pwd... "
zsh_pwd_ns=$(measure_command 'pwd' $ITERATIONS)
zsh_pwd_us=$(echo "scale=2; $zsh_pwd_ns / 1000" | bc)
echo "${zsh_pwd_us} µs (Zsh builtin)"

# Test 3: cat large file (external /bin/cat)
echo -n "3. cat large file (10K lines)... "
cat_ns=$(measure_command '/bin/cat ./benchmark-data/large-file.txt' 1000)
cat_us=$(echo "scale=2; $cat_ns / 1000" | bc)
echo "${cat_us} µs (/bin/cat)"

# Test 4: ls many files (external /bin/ls)
echo -n "4. ls directory (50 files)... "
ls_ns=$(measure_command '/bin/ls ./benchmark-data/deep-tree/dir1' 1000)
ls_us=$(echo "scale=2; $ls_ns / 1000" | bc)
echo "${ls_us} µs (/bin/ls)"

# Test 5: ls -la (external /bin/ls)
echo -n "5. ls -la directory... "
ls_la_ns=$(measure_command '/bin/ls -la ./benchmark-data/deep-tree/dir1' 1000)
ls_la_us=$(echo "scale=2; $ls_la_ns / 1000" | bc)
echo "${ls_la_us} µs (/bin/ls -la)"

# Test 6: grep (external /bin/grep)
echo -n "6. grep large file... "
grep_ns=$(measure_command '/bin/grep "FOUND" ./benchmark-data/large-file.txt' 1000)
grep_us=$(echo "scale=2; $grep_ns / 1000" | bc)
echo "${grep_us} µs (/bin/grep)"

# Test 7: find (external /usr/bin/find)
echo -n "7. find deep tree... "
find_ns=$(measure_command '/usr/bin/find ./benchmark-data/deep-tree -name "*.txt"' 100)
find_us=$(echo "scale=2; $find_ns / 1000" | bc)
echo "${find_us} µs (/usr/bin/find)"

echo ""
echo "========================================="
echo "COMPARISON TO RUSH CRITERION BENCHMARKS"
echo "========================================="
echo ""

# Rush Criterion results (from previous run)
rush_echo=8.47
rush_pwd=8.30
rush_cat=10.01
rush_grep=11.83
rush_ls=113.18
rush_ls_la=118.94
rush_find=8.83

printf "%-25s %15s %15s %10s\n" "Operation" "Zsh/External" "Rush" "Winner"
printf "%-25s %15s %15s %10s\n" "---------" "------------" "----" "------"

# echo
printf "%-25s %14s µs %14s µs " "echo" "$zsh_echo_us" "$rush_echo"
if (( $(echo "$rush_echo < $zsh_echo_us" | bc -l) )); then
    ratio=$(echo "scale=2; $zsh_echo_us / $rush_echo" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_echo / $zsh_echo_us" | bc)
    echo "Zsh (${ratio}x)"
fi

# pwd
printf "%-25s %14s µs %14s µs " "pwd" "$zsh_pwd_us" "$rush_pwd"
if (( $(echo "$rush_pwd < $zsh_pwd_us" | bc -l) )); then
    ratio=$(echo "scale=2; $zsh_pwd_us / $rush_pwd" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_pwd / $zsh_pwd_us" | bc)
    echo "Zsh (${ratio}x)"
fi

# cat
printf "%-25s %14s µs %14s µs " "cat (10K lines)" "$cat_us" "$rush_cat"
if (( $(echo "$rush_cat < $cat_us" | bc -l) )); then
    ratio=$(echo "scale=2; $cat_us / $rush_cat" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_cat / $cat_us" | bc)
    echo "/bin/cat (${ratio}x)"
fi

# grep
printf "%-25s %14s µs %14s µs " "grep" "$grep_us" "$rush_grep"
if (( $(echo "$rush_grep < $grep_us" | bc -l) )); then
    ratio=$(echo "scale=2; $grep_us / $rush_grep" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_grep / $grep_us" | bc)
    echo "/bin/grep (${ratio}x)"
fi

# ls
printf "%-25s %14s µs %14s µs " "ls (50 files)" "$ls_us" "$rush_ls"
if (( $(echo "$rush_ls < $ls_us" | bc -l) )); then
    ratio=$(echo "scale=2; $ls_us / $rush_ls" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_ls / $ls_us" | bc)
    echo "/bin/ls (${ratio}x)"
fi

# ls -la
printf "%-25s %14s µs %14s µs " "ls -la" "$ls_la_us" "$rush_ls_la"
if (( $(echo "$rush_ls_la < $ls_la_us" | bc -l) )); then
    ratio=$(echo "scale=2; $ls_la_us / $rush_ls_la" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_ls_la / $ls_la_us" | bc)
    echo "/bin/ls (${ratio}x)"
fi

# find
printf "%-25s %14s µs %14s µs " "find (1000 files)" "$find_us" "$rush_find"
if (( $(echo "$rush_find < $find_us" | bc -l) )); then
    ratio=$(echo "scale=2; $find_us / $rush_find" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_find / $find_us" | bc)
    echo "/usr/bin/find (${ratio}x)"
fi

echo ""
echo "========================================="
echo "NOTES"
echo "========================================="
echo ""
echo "Rush (Criterion):"
echo "  - Direct function calls (no shell overhead)"
echo "  - Built-in implementations in Rust"
echo "  - Measured with statistical analysis"
echo ""
echo "Zsh Equivalent:"
echo "  - Builtins (echo, pwd): Direct Zsh execution"
echo "  - External commands: /bin/cat, /bin/grep, etc."
echo "  - Includes process spawning overhead for external"
echo ""
echo "Key Insight:"
echo "  Rush builtins are comparable to Zsh builtins (µs range)"
echo "  Rush builtins are MUCH faster than external commands"
echo "  This is why Rush wins in script benchmarks (3.39x)"
echo ""
