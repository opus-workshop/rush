#!/bin/bash

# Run the benchmark 10 times and collect results
echo "Running startup benchmarks (10 iterations)..."
echo ""

total=0
for i in {1..10}; do
    # Use gtime (GNU time) if available, otherwise fallback to bash TIMEFORMAT
    start=$(gdate +%s%N 2>/dev/null || date +%s000000000)
    ./target/release/rush -c "exit" > /dev/null 2>&1
    end=$(gdate +%s%N 2>/dev/null || date +%s000000000)

    elapsed_ns=$((end - start))
    elapsed_ms=$(echo "scale=2; $elapsed_ns / 1000000" | bc)

    echo "Run $i: ${elapsed_ms}ms"
    total=$(echo "$total + $elapsed_ms" | bc)
done

echo ""
mean=$(echo "scale=2; $total / 10" | bc)
echo "Mean startup time: ${mean}ms"
