#!/usr/bin/env bash

# check_regression.sh - Detect performance regressions in benchmarks
# Compares current benchmark results against baseline and alerts on significant changes
# Usage: check_regression.sh --baseline-ref <ref> --current-ref <ref> --threshold <percent>

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default threshold: 10% regression
THRESHOLD=10
BASELINE_REF=""
CURRENT_REF=""
REGRESSION_DETECTED=0
REGRESSION_REPORT="regression_report.txt"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --baseline-ref)
            BASELINE_REF="$2"
            shift 2
            ;;
        --current-ref)
            CURRENT_REF="$2"
            shift 2
            ;;
        --threshold)
            THRESHOLD="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 --baseline-ref <ref> --current-ref <ref> --threshold <percent>"
            exit 1
            ;;
    esac
done

# Validate arguments
if [ -z "$BASELINE_REF" ] || [ -z "$CURRENT_REF" ]; then
    echo "Error: --baseline-ref and --current-ref are required"
    exit 1
fi

echo -e "${BLUE}=== Performance Regression Detection ===${NC}\n"
echo "Baseline:  $BASELINE_REF"
echo "Current:   $CURRENT_REF"
echo "Threshold: ${THRESHOLD}%\n"

# Create regression report file
cat > "$REGRESSION_REPORT" << 'REPORT_EOF'
## Benchmark Regression Analysis

REPORT_EOF

# Function to extract metrics from criterion JSON output
extract_metrics() {
    local bench_dir="$1"
    local metrics=""

    if [ ! -d "$bench_dir" ]; then
        echo ""
        return
    fi

    # Find all benchmark results (looking for estimates.json files)
    for estimate_file in "$bench_dir"/*/base/estimates.json; do
        if [ -f "$estimate_file" ]; then
            local bench_name=$(basename $(dirname $(dirname "$estimate_file")))
            local mean=$(grep '"mean"' "$estimate_file" | head -1 | grep -o '[0-9.]*' | head -1)
            if [ -n "$mean" ]; then
                metrics="$metrics
$bench_name: $mean"
            fi
        fi
    done

    echo "$metrics"
}

# Function to compare two metric values and detect regression
check_metric_regression() {
    local name="$1"
    local baseline="$2"
    local current="$3"

    if [ -z "$baseline" ] || [ -z "$current" ]; then
        return 0
    fi

    # Calculate percentage change: ((current - baseline) / baseline) * 100
    local change=$(awk "BEGIN {printf \"%.2f\", (($current - $baseline) / $baseline) * 100}")

    # Check if regression exceeds threshold
    if (( $(echo "$change > $THRESHOLD" | bc -l) )); then
        echo -e "${RED}✗ REGRESSION${NC} $name: +${change}% (baseline: ${baseline}ms, current: ${current}ms)"
        echo "- **$name**: ${change}% regression (${baseline}ms → ${current}ms)" >> "$REGRESSION_REPORT"
        REGRESSION_DETECTED=1
        return 1
    elif (( $(echo "$change < -5" | bc -l) )); then
        echo -e "${GREEN}✓ IMPROVEMENT${NC} $name: ${change}% (baseline: ${baseline}ms, current: ${current}ms)"
        echo "- **$name**: ${change}% improvement" >> "$REGRESSION_REPORT"
        return 0
    else
        echo -e "${GREEN}✓ OK${NC} $name: ${change}% change (within tolerance)"
        return 0
    fi
}

# Check if criterion results exist
if [ ! -d "target/criterion" ]; then
    echo -e "${YELLOW}Warning: No criterion benchmark results found${NC}"
    exit 0
fi

echo -e "${BLUE}Analyzing benchmark results...${NC}\n"

# Extract metrics from current run
CURRENT_METRICS=$(extract_metrics "target/criterion")

# Try to get baseline metrics from git history
# For PR context, we'll try to fetch from the base branch
BASELINE_METRICS=""

if [ -d ".git" ] && git rev-parse "$BASELINE_REF" > /dev/null 2>&1; then
    echo -e "${BLUE}Fetching baseline metrics from $BASELINE_REF...${NC}"

    # Create temporary directory for baseline
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    # Try to get criterion data from baseline commit
    if git show "$BASELINE_REF:target/criterion" > /dev/null 2>&1; then
        git show "$BASELINE_REF:target/criterion" | tar -xz -C "$TEMP_DIR" 2>/dev/null || true
        BASELINE_METRICS=$(extract_metrics "$TEMP_DIR")
    fi
fi

# If we have baseline metrics, compare them
if [ -n "$BASELINE_METRICS" ]; then
    echo -e "${YELLOW}Comparing benchmarks...${NC}\n"

    # Parse metrics and compare
    while IFS=': ' read -r name current_value; do
        if [ -n "$name" ] && [ -n "$current_value" ]; then
            # Find corresponding baseline value
            baseline_value=$(echo "$BASELINE_METRICS" | grep "^$name:" | cut -d' ' -f2)

            if [ -n "$baseline_value" ]; then
                check_metric_regression "$name" "$baseline_value" "$current_value"
            fi
        fi
    done <<< "$CURRENT_METRICS"
else
    echo -e "${YELLOW}No baseline metrics available for comparison${NC}"
    echo "This is the first benchmark run or baseline is not in git history"
    echo "" >> "$REGRESSION_REPORT"
    echo "**Note:** Baseline metrics not available. Skipping regression comparison." >> "$REGRESSION_REPORT"
fi

# Analyze criterion output directly if JSON parsing failed
echo ""
echo -e "${BLUE}Analyzing criterion output...${NC}\n"

if [ -f "benchmark_output.txt" ]; then
    # Look for regression patterns in text output
    if grep -i "regression" benchmark_output.txt > /dev/null; then
        echo -e "${YELLOW}⚠️  Criterion detected regressions${NC}"
        REGRESSION_DETECTED=1
    fi
fi

# Summary
echo ""
echo -e "${YELLOW}=== Summary ===${NC}"

if [ $REGRESSION_DETECTED -eq 0 ]; then
    echo -e "${GREEN}✓ No significant regressions detected (threshold: ${THRESHOLD}%)${NC}"
    echo "" >> "$REGRESSION_REPORT"
    echo "✅ **Result**: No significant performance regressions detected" >> "$REGRESSION_REPORT"
    exit 0
else
    echo -e "${RED}✗ Performance regressions detected!${NC}"
    echo "" >> "$REGRESSION_REPORT"
    echo "Please investigate and optimize before merging." >> "$REGRESSION_REPORT"
    exit 1
fi
