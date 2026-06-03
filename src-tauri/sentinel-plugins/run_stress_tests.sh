#!/bin/bash

# Plugin System Stress Test Runner
# 
# Usage:
#   ./run_stress_tests.sh [category]
#
# Categories:
#   all         - Run all tests (default)
#   memory      - Memory leak tests
#   cpu         - CPU intensive tests
#   concurrency - Concurrency tests
#   v8          - V8 limits tests
#   robustness  - Robustness / resilience tests
#   network     - Network stress tests
#   basic       - Basic stress tests

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

CATEGORY="${1:-all}"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
OUTPUT_DIR="stress_test_results"
OUTPUT_FILE="$OUTPUT_DIR/stress_test_${CATEGORY}_${TIMESTAMP}.log"

# 创建输出目录
mkdir -p "$OUTPUT_DIR"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}Plugin System Stress Test Runner${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo -e "Category: ${GREEN}$CATEGORY${NC}"
echo -e "Output: ${GREEN}$OUTPUT_FILE${NC}"
echo ""

# 检查系统信息
echo -e "${YELLOW}System Information:${NC}"
echo "  OS: $(uname -s)"
echo "  Architecture: $(uname -m)"
echo "  CPU Cores: $(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 'unknown')"
echo "  Total Memory: $(free -h 2>/dev/null | grep Mem | awk '{print $2}' || sysctl -n hw.memsize 2>/dev/null | awk '{print $1/1024/1024/1024 " GB"}' || echo 'unknown')"
echo ""

# 运行测试函数
run_tests() {
    local test_file=$1
    local test_name=$2
    
    echo -e "${YELLOW}Running $test_name...${NC}"
    echo "========================================" | tee -a "$OUTPUT_FILE"
    echo "Test: $test_name" | tee -a "$OUTPUT_FILE"
    echo "Started at: $(date)" | tee -a "$OUTPUT_FILE"
    echo "========================================" | tee -a "$OUTPUT_FILE"
    
    # V8 limit tests include memory-pressure cases. Running them in parallel can
    # cause fatal V8 OOM (process abort) due to combined heap pressure.
    local extra_test_args=""
    if [ "$test_file" = "v8_limits_tests" ]; then
        extra_test_args="--test-threads=1"
    fi

    if cargo test --test "$test_file" --release -- --ignored --nocapture $extra_test_args 2>&1 | tee -a "$OUTPUT_FILE"; then
        echo -e "${GREEN}✓ $test_name completed successfully${NC}"
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        return 1
    fi
    
    echo "" | tee -a "$OUTPUT_FILE"
}

# 开始测试
START_TIME=$(date +%s)

case "$CATEGORY" in
    all)
        echo -e "${YELLOW}Running all stress tests...${NC}"
        echo ""
        
        run_tests "stress_tests" "Basic Stress Tests"
        run_tests "memory_leak_tests" "Memory Leak Tests"
        run_tests "cpu_stress_tests" "CPU Intensive Tests"
        run_tests "concurrency_tests" "Concurrency Tests"
        run_tests "v8_limits_tests" "V8 Limits Tests"
        run_tests "robustness_tests" "Robustness Tests"
        run_tests "network_stress_tests" "Network Stress Tests"
        ;;
    
    memory)
        run_tests "memory_leak_tests" "Memory Leak Tests"
        ;;
    
    cpu)
        run_tests "cpu_stress_tests" "CPU Intensive Tests"
        ;;
    
    concurrency)
        run_tests "concurrency_tests" "Concurrency Tests"
        ;;
    
    v8)
        run_tests "v8_limits_tests" "V8 Limits Tests"
        ;;

    robustness)
        run_tests "robustness_tests" "Robustness Tests"
        ;;
    
    network)
        run_tests "network_stress_tests" "Network Stress Tests"
        ;;
    
    basic)
        run_tests "stress_tests" "Basic Stress Tests"
        ;;
    
    *)
        echo -e "${RED}Unknown category: $CATEGORY${NC}"
        echo ""
        echo "Available categories:"
        echo "  all         - Run all tests"
        echo "  memory      - Memory leak tests"
        echo "  cpu         - CPU intensive tests"
        echo "  concurrency - Concurrency tests"
        echo "  v8          - V8 limits tests"
        echo "  robustness  - Robustness tests"
        echo "  network     - Network stress tests"
        echo "  basic       - Basic stress tests"
        exit 1
        ;;
esac

# 计算总时间
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))
MINUTES=$((DURATION / 60))
SECONDS=$((DURATION % 60))

echo ""
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}All tests completed!${NC}"
echo -e "${BLUE}======================================${NC}"
echo ""
echo "Total Duration: ${MINUTES}m ${SECONDS}s"
echo "Results saved to: $OUTPUT_FILE"
echo ""

# 生成摘要
echo -e "${YELLOW}Generating summary...${NC}"
echo ""
echo "Summary:" | tee -a "$OUTPUT_FILE"
echo "--------" | tee -a "$OUTPUT_FILE"

# 统计通过/失败
TOTAL_TESTS=$(grep -c "test result:" "$OUTPUT_FILE" || echo "0")
PASSED_TESTS=$(grep "test result: ok" "$OUTPUT_FILE" | wc -l || echo "0")
FAILED_TESTS=$(grep "test result: FAILED" "$OUTPUT_FILE" | wc -l || echo "0")

echo "Total Test Suites: $TOTAL_TESTS" | tee -a "$OUTPUT_FILE"
echo "Passed: $PASSED_TESTS" | tee -a "$OUTPUT_FILE"
echo "Failed: $FAILED_TESTS" | tee -a "$OUTPUT_FILE"
echo "" | tee -a "$OUTPUT_FILE"

# 检查是否有错误
if [ "$FAILED_TESTS" -gt 0 ]; then
    echo -e "${RED}⚠️  Some tests failed. Please review the log file.${NC}"
    exit 1
else
    echo -e "${GREEN}✓ All tests passed!${NC}"
fi

# 提示查看详细报告
echo ""
echo -e "${BLUE}To view detailed results:${NC}"
echo "  cat $OUTPUT_FILE"
echo ""
echo -e "${BLUE}To generate a report:${NC}"
echo "  # Add report generation code here"
echo ""

