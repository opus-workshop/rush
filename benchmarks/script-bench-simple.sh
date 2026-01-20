#!/bin/bash
# Fair Rush vs Zsh Script-Based Benchmark (Simplified)
# Tests what Rush currently supports: commands and pipelines

set -e

echo "========================================="
echo "Rush vs Zsh Script Performance"
echo "========================================="
echo ""
echo "Measuring script execution (startup happens once)"
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

# Test 1: Repeated pipeline operations in a script
cat > /tmp/rush-test-1.sh <<'SCRIPT'
#!/usr/bin/env rush
cat ./benchmark-data/large-file.txt | grep "Lorem" | wc -l
cat ./benchmark-data/large-file.txt | grep "ipsum" | wc -l
cat ./benchmark-data/large-file.txt | grep "dolor" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 1" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 2" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 3" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 4" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 5" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 6" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 7" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 8" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 9" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 10" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 11" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 12" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 13" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 14" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 15" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 16" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 17" | wc -l
SCRIPT

chmod +x /tmp/rush-test-1.sh

echo -n "1. Pipeline processing (20 pipes × 10K lines)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-1.sh > /dev/null
end=$(gdate +%s%N)
rush_time1=$((($end - $start) / 1000000))

cat > /tmp/zsh-test-1.sh <<'SCRIPT'
#!/bin/zsh
cat ./benchmark-data/large-file.txt | grep "Lorem" | wc -l
cat ./benchmark-data/large-file.txt | grep "ipsum" | wc -l
cat ./benchmark-data/large-file.txt | grep "dolor" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 1" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 2" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 3" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 4" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 5" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 6" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 7" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 8" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 9" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 10" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 11" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 12" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 13" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 14" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 15" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 16" | wc -l
cat ./benchmark-data/large-file.txt | grep "Line 17" | wc -l
SCRIPT

chmod +x /tmp/zsh-test-1.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-1.sh > /dev/null
end=$(gdate +%s%N)
zsh_time1=$((($end - $start) / 1000000))

echo "Rush: ${rush_time1}ms, Zsh: ${zsh_time1}ms"

# Test 2: Simple command sequences
cat > /tmp/rush-test-2.sh <<'SCRIPT'
#!/usr/bin/env rush
echo "test1"
echo "test2"
echo "test3"
echo "test4"
echo "test5"
pwd
pwd
pwd
pwd
pwd
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
cat ./benchmark-data/large-file.txt
cat ./benchmark-data/large-file.txt
cat ./benchmark-data/large-file.txt
SCRIPT

chmod +x /tmp/rush-test-2.sh

echo -n "2. Simple commands (18 commands)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-2.sh > /dev/null
end=$(gdate +%s%N)
rush_time2=$((($end - $start) / 1000000))

cat > /tmp/zsh-test-2.sh <<'SCRIPT'
#!/bin/zsh
echo "test1"
echo "test2"
echo "test3"
echo "test4"
echo "test5"
pwd
pwd
pwd
pwd
pwd
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
ls ./benchmark-data
cat ./benchmark-data/large-file.txt
cat ./benchmark-data/large-file.txt
cat ./benchmark-data/large-file.txt
SCRIPT

chmod +x /tmp/zsh-test-2.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-2.sh > /dev/null
end=$(gdate +%s%N)
zsh_time2=$((($end - $start) / 1000000))

echo "Rush: ${rush_time2}ms, Zsh: ${zsh_time2}ms"

# Test 3: Grep-heavy workload
cat > /tmp/rush-test-3.sh <<'SCRIPT'
#!/usr/bin/env rush
grep "Line 100" ./benchmark-data/large-file.txt
grep "Line 200" ./benchmark-data/large-file.txt
grep "Line 300" ./benchmark-data/large-file.txt
grep "Line 400" ./benchmark-data/large-file.txt
grep "Line 500" ./benchmark-data/large-file.txt
grep "Line 600" ./benchmark-data/large-file.txt
grep "Line 700" ./benchmark-data/large-file.txt
grep "Line 800" ./benchmark-data/large-file.txt
grep "Line 900" ./benchmark-data/large-file.txt
grep "Line 1000" ./benchmark-data/large-file.txt
SCRIPT

chmod +x /tmp/rush-test-3.sh

echo -n "3. Grep operations (10× on 10K lines)... "
start=$(gdate +%s%N)
$RUSH /tmp/rush-test-3.sh > /dev/null
end=$(gdate +%s%N)
rush_time3=$((($end - $start) / 1000000))

cat > /tmp/zsh-test-3.sh <<'SCRIPT'
#!/bin/zsh
grep "Line 100" ./benchmark-data/large-file.txt
grep "Line 200" ./benchmark-data/large-file.txt
grep "Line 300" ./benchmark-data/large-file.txt
grep "Line 400" ./benchmark-data/large-file.txt
grep "Line 500" ./benchmark-data/large-file.txt
grep "Line 600" ./benchmark-data/large-file.txt
grep "Line 700" ./benchmark-data/large-file.txt
grep "Line 800" ./benchmark-data/large-file.txt
grep "Line 900" ./benchmark-data/large-file.txt
grep "Line 1000" ./benchmark-data/large-file.txt
SCRIPT

chmod +x /tmp/zsh-test-3.sh

start=$(gdate +%s%N)
zsh /tmp/zsh-test-3.sh > /dev/null
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

printf "%-45s %10s %10s %10s\n" "Test" "Rush" "Zsh" "Winner"
printf "%-45s %10s %10s %10s\n" "----" "----" "---" "------"

printf "%-45s %9sms %9sms " "Pipeline processing (20 pipes)" "$rush_time1" "$zsh_time1"
if [ $rush_time1 -lt $zsh_time1 ]; then
    ratio=$(echo "scale=2; $zsh_time1 / $rush_time1" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time1 / $zsh_time1" | bc)
    echo "Zsh (${ratio}x)"
fi

printf "%-45s %9sms %9sms " "Simple commands (18 commands)" "$rush_time2" "$zsh_time2"
if [ $rush_time2 -lt $zsh_time2 ]; then
    ratio=$(echo "scale=2; $zsh_time2 / $rush_time2" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time2 / $zsh_time2" | bc)
    echo "Zsh (${ratio}x)"
fi

printf "%-45s %9sms %9sms " "Grep operations (10×)" "$rush_time3" "$zsh_time3"
if [ $rush_time3 -lt $zsh_time3 ]; then
    ratio=$(echo "scale=2; $zsh_time3 / $rush_time3" | bc)
    echo "Rush (${ratio}x)"
else
    ratio=$(echo "scale=2; $rush_time3 / $zsh_time3" | bc)
    echo "Zsh (${ratio}x)"
fi

echo ""
printf "%-45s %9sms %9sms " "TOTAL" "$rush_total" "$zsh_total"
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
echo "ANALYSIS"
echo "========================================="
echo ""
echo "✓ Startup happens ONCE per script (fair comparison)"
echo "✓ Rush uses builtins: cat, grep, ls, echo, pwd"
echo "✓ Zsh uses external: /bin/cat, /bin/grep, etc."
echo ""

if [ $rush_total -lt $zsh_total ]; then
    echo "🎉 Rush is faster! Built-in commands outperform external processes."
    echo ""
    echo "Why Rush wins:"
    echo "  - No process spawning overhead for cat, grep, ls"
    echo "  - Memory-mapped I/O for file operations"
    echo "  - Zero-copy pipelines (in-memory buffers)"
else
    echo "⚠️  Rush is slower in this test."
    echo ""
    echo "Likely reasons:"
    echo "  - Script parsing overhead"
    echo "  - Pipeline implementation needs optimization"
    echo "  - External commands may be cached/optimized by OS"
fi

echo ""
echo "For pure builtin microbenchmarks:"
echo "  cargo bench --bench shell_comparison"
echo ""

# Cleanup
rm -f /tmp/rush-test-*.sh /tmp/zsh-test-*.sh
