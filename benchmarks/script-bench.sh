#!/bin/bash
# Fair Rush vs Zsh Script-Based Benchmark
# Measures end-to-end script performance (startup happens once per script)

set -e

echo "========================================="
echo "Rush vs Zsh Script Performance"
echo "========================================="
echo ""
echo "This benchmark runs test scripts that perform real work."
echo "Startup overhead is amortized across all operations in the script."
echo ""

# Check prerequisites
if ! command -v gdate &> /dev/null; then
    echo "Error: gdate not found. Install with: brew install coreutils"
    exit 1
fi

# Build Rush
cd "$(dirname "$0")/.."
echo "Building Rush (release mode)..."
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" || true
echo ""

cd benchmarks

# Ensure test data exists
mkdir -p ./benchmark-data
if [ ! -f ./benchmark-data/large-file.txt ]; then
    echo "Creating test data..."
    for i in {1..10000}; do
        echo "Line $i: Lorem ipsum dolor sit amet, consectetur adipiscing elit." >> ./benchmark-data/large-file.txt
    done
fi

RUSH="../target/release/rush"

echo "========================================="
echo "BENCHMARK TESTS"
echo "========================================="
echo ""

# Test 1: File Processing Script
cat > /tmp/rush-test-1.sh <<'SCRIPT'
#!/usr/bin/env rush
# Process a large file multiple times
for i in {1..20}; do
    cat ./benchmark-data/large-file.txt | grep -v "Lorem" | wc -l > /dev/null
done
SCRIPT

chmod +x /tmp/rush-test-1.sh

echo -n "1. File processing pipeline (20 iterations)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-1.sh
end=$(gdate +%s%N)
rush_time1=$((($end - $start) / 1000000))

# Same script for Zsh (change shebang)
cat > /tmp/zsh-test-1.sh <<'SCRIPT'
#!/bin/zsh
for i in {1..20}; do
    cat ./benchmark-data/large-file.txt | grep -v "Lorem" | wc -l > /dev/null
done
SCRIPT

chmod +x /tmp/zsh-test-1.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-1.sh
end=$(gdate +%s%N)
zsh_time1=$((($end - $start) / 1000000))

echo "Rush: ${rush_time1}ms, Zsh: ${zsh_time1}ms"

# Test 2: Loop Performance
cat > /tmp/rush-test-2.sh <<'SCRIPT'
#!/usr/bin/env rush
# Simple loop with builtin commands
count=0
for i in {1..100}; do
    echo "test" > /dev/null
    count=$((count + 1))
done
SCRIPT

chmod +x /tmp/rush-test-2.sh

echo -n "2. Loop with builtins (100 iterations)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-2.sh
end=$(gdate +%s%N)
rush_time2=$((($end - $start) / 1000000))

cat > /tmp/zsh-test-2.sh <<'SCRIPT'
#!/bin/zsh
count=0
for i in {1..100}; do
    echo "test" > /dev/null
    count=$((count + 1))
done
SCRIPT

chmod +x /tmp/zsh-test-2.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-2.sh
end=$(gdate +%s%N)
zsh_time2=$((($end - $start) / 1000000))

echo "Rush: ${rush_time2}ms, Zsh: ${zsh_time2}ms"

# Test 3: Mixed Operations
cat > /tmp/rush-test-3.sh <<'SCRIPT'
#!/usr/bin/env rush
# Mixed file operations
for i in {1..10}; do
    ls ./benchmark-data > /dev/null
    cat ./benchmark-data/large-file.txt | head -100 > /dev/null
    grep "Line 500" ./benchmark-data/large-file.txt > /dev/null
done
SCRIPT

chmod +x /tmp/rush-test-3.sh

echo -n "3. Mixed file operations (10 iterations)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-3.sh
end=$(gdate +%s%N)
rush_time3=$((($end - $start) / 1000000))

cat > /tmp/zsh-test-3.sh <<'SCRIPT'
#!/bin/zsh
for i in {1..10}; do
    ls ./benchmark-data > /dev/null
    cat ./benchmark-data/large-file.txt | head -100 > /dev/null
    grep "Line 500" ./benchmark-data/large-file.txt > /dev/null
done
SCRIPT

chmod +x /tmp/zsh-test-3.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-3.sh
end=$(gdate +%s%N)
zsh_time3=$((($end - $start) / 1000000))

echo "Rush: ${rush_time3}ms, Zsh: ${zsh_time3}ms"

# Calculate totals
rush_total=$(($rush_time1 + $rush_time2 + $rush_time3))
zsh_total=$(($zsh_time1 + $zsh_time2 + $zsh_time3))

# Results
echo ""
echo "========================================="
echo "RESULTS"
echo "========================================="
echo ""

printf "%-40s %10s %10s %10s\n" "Test" "Rush" "Zsh" "Winner"
printf "%-40s %10s %10s %10s\n" "----" "----" "---" "------"

printf "%-40s %9sms %9sms " "File processing pipeline (20x)" "$rush_time1" "$zsh_time1"
if [ $rush_time1 -lt $zsh_time1 ]; then
    ratio=$(echo "scale=2; $zsh_time1 / $rush_time1" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time1 / $zsh_time1" | bc)
    echo "Zsh (${ratio}x)"
fi

printf "%-40s %9sms %9sms " "Loop with builtins (100x)" "$rush_time2" "$zsh_time2"
if [ $rush_time2 -lt $zsh_time2 ]; then
    ratio=$(echo "scale=2; $zsh_time2 / $rush_time2" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time2 / $zsh_time2" | bc)
    echo "Zsh (${ratio}x)"
fi

printf "%-40s %9sms %9sms " "Mixed file operations (10x)" "$rush_time3" "$zsh_time3"
if [ $rush_time3 -lt $zsh_time3 ]; then
    ratio=$(echo "scale=2; $zsh_time3 / $rush_time3" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time3 / $zsh_time3" | bc)
    echo "Zsh (${ratio}x)"
fi

echo ""
printf "%-40s %9sms %9sms " "TOTAL" "$rush_total" "$zsh_total"
if [ $rush_total -lt $zsh_total ]; then
    speedup=$(echo "scale=2; $zsh_total / $rush_total" | bc)
    improvement=$(echo "scale=1; ($speedup - 1) * 100" | bc)
    echo "Rush (${speedup}x, ${improvement}% faster)"
else
    slowdown=$(echo "scale=2; $rush_total / $zsh_total" | bc)
    decline=$(echo "scale=1; ($slowdown - 1) * 100" | bc)
    echo "Zsh (${slowdown}x, Rush ${decline}% slower)"
fi

echo ""
echo "========================================="
echo "NOTES"
echo "========================================="
echo ""
echo "✓ Startup overhead amortized (happens once per script)"
echo "✓ Real-world script workloads"
echo "✓ Tests loops, pipes, and file operations"
echo ""
echo "Comparison:"
echo "  - Rush uses builtins (cat, grep, ls, echo, etc.)"
echo "  - Zsh may use external commands (/bin/cat, etc.)"
echo "  - This measures end-to-end script performance"
echo ""
echo "For pure builtin performance (no startup):"
echo "  cargo bench --bench shell_comparison"
echo ""

# Cleanup
rm -f /tmp/rush-test-*.sh /tmp/zsh-test-*.sh
