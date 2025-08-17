#!/bin/bash

# TTLog Comprehensive Benchmark Runner
# Runs all benchmark types: distributed, stress, performance, and simulations

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
OUTPUT_DIR="target/benchmark_results"
REPORT_FILE="comprehensive_benchmark_report.txt"
QUICK_MODE=false
VERBOSE=false
RUN_STRESS=false
RUN_DISTRIBUTED=false
RUN_PERFORMANCE=false
RUN_SIMULATIONS=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -q|--quick)
            QUICK_MODE=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -s|--stress)
            RUN_STRESS=true
            shift
            ;;
        -d|--distributed)
            RUN_DISTRIBUTED=true
            shift
            ;;
        -p|--performance)
            RUN_PERFORMANCE=true
            shift
            ;;
        -i|--simulations)
            RUN_SIMULATIONS=true
            shift
            ;;
        -a|--all)
            RUN_STRESS=true
            RUN_DISTRIBUTED=true
            RUN_PERFORMANCE=true
            RUN_SIMULATIONS=true
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo "Options:"
            echo "  -q, --quick        Quick mode with reduced samples"
            echo "  -v, --verbose      Verbose output"
            echo "  -s, --stress       Run stress tests"
            echo "  -d, --distributed  Run distributed benchmarks"
            echo "  -p, --performance  Run performance tests"
            echo "  -i, --simulations  Run simulation tests"
            echo "  -a, --all          Run all tests (default)"
            echo "  -h, --help         Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Default to running all if no specific tests selected
if [ "$RUN_STRESS" = false ] && [ "$RUN_DISTRIBUTED" = false ] && [ "$RUN_PERFORMANCE" = false ] && [ "$RUN_SIMULATIONS" = false ]; then
    RUN_STRESS=true
    RUN_DISTRIBUTED=true
    RUN_PERFORMANCE=true
    RUN_SIMULATIONS=true
fi

echo -e "${BLUE}🚀 TTLog Comprehensive Benchmark Suite${NC}"
echo -e "${BLUE}=====================================${NC}"
echo ""

# Set quick mode if requested
if [ "$QUICK_MODE" = true ]; then
    export CRITERION_SAMPLE_SIZE=20
    export CRITERION_MEASUREMENT_TIME=2000
    export CRITERION_WARM_UP_TIME=500
    echo -e "${YELLOW}⚡ Quick mode enabled - reduced sample sizes for faster results${NC}"
    echo ""
fi

# Build the project
echo -e "${BLUE}🔨 Building TTLog project...${NC}"
cargo build --release
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Function to run benchmark and capture output
run_benchmark() {
    local name="$1"
    local description="$2"
    local command="$3"
    
    echo -e "${CYAN}📊 Running: ${description}${NC}"
    
    if [ "$VERBOSE" = true ]; then
        $command 2>&1 | tee "${OUTPUT_DIR}/${name}_output.txt"
    else
        $command > "${OUTPUT_DIR}/${name}_output.txt" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ ${description} completed${NC}"
    else
        echo -e "${RED}❌ ${description} failed${NC}"
        return 1
    fi
    echo ""
}

# Function to run binary test
run_binary_test() {
    local name="$1"
    local description="$2"
    local args="$3"
    
    echo -e "${CYAN}🔧 Running: ${description}${NC}"
    
    if [ "$VERBOSE" = true ]; then
        cargo run --bin "$name" $args 2>&1 | tee "${OUTPUT_DIR}/${name}_output.txt"
    else
        cargo run --bin "$name" $args > "${OUTPUT_DIR}/${name}_output.txt" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✅ ${description} completed${NC}"
    else
        echo -e "${RED}❌ ${description} failed${NC}"
        return 1
    fi
    echo ""
}

# Run all selected benchmarks
echo -e "${BLUE}🎯 Starting Comprehensive Benchmark Suite${NC}"
echo ""

# 1. Distributed System Benchmarks
if [ "$RUN_DISTRIBUTED" = true ]; then
    echo -e "${PURPLE}🌐 Distributed System Benchmarks${NC}"
    echo "=========================================="
    
    run_benchmark "distributed_bench" "Distributed System Benchmarks" \
        "cargo bench --bench distributed_bench"
fi

# 2. Stress Testing
if [ "$RUN_STRESS" = true ]; then
    echo -e "${PURPLE}🔥 Heavy Stress Testing${NC}"
    echo "============================="
    
    run_binary_test "heavy_stress_test" "Memory Stress Test" "memory"
    run_binary_test "heavy_stress_test" "CPU Stress Test" "cpu"
    run_binary_test "heavy_stress_test" "Network Stress Test" "network"
    run_binary_test "heavy_stress_test" "Comprehensive Stress Test" "all"
fi

# 3. Performance Testing
if [ "$RUN_PERFORMANCE" = true ]; then
    echo -e "${PURPLE}🚀 Maximum Performance Testing${NC}"
    echo "====================================="
    
    run_binary_test "max_performance" "Throughput Tests" "throughput"
    run_binary_test "max_performance" "Concurrency Tests" "concurrency"
    run_binary_test "max_performance" "Memory Efficiency Tests" "memory"
    run_binary_test "max_performance" "Comprehensive Performance Tests" "all"
fi

# 4. Distributed Simulations
if [ "$RUN_SIMULATIONS" = true ]; then
    echo -e "${PURPLE}🌐 Distributed System Simulations${NC}"
    echo "=========================================="
    
    run_binary_test "distributed_simulator" "Database Simulation" "database"
    run_binary_test "distributed_simulator" "Microservice Simulation" "microservice"
    run_binary_test "distributed_simulator" "Message Queue Simulation" "messagequeue"
    run_binary_test "distributed_simulator" "Cache Simulation" "cache"
    run_binary_test "distributed_simulator" "Comprehensive Simulation" "all"
fi

# Generate comprehensive report
echo -e "${BLUE}📋 Generating Comprehensive Benchmark Report...${NC}"
{
    echo "TTLog Comprehensive Benchmark Report"
    echo "==================================="
    echo "Generated: $(date)"
    echo "Quick Mode: $QUICK_MODE"
    echo "Tests Run:"
    echo "  - Distributed Benchmarks: $RUN_DISTRIBUTED"
    echo "  - Stress Testing: $RUN_STRESS"
    echo "  - Performance Testing: $RUN_PERFORMANCE"
    echo "  - Simulations: $RUN_SIMULATIONS"
    echo ""
    echo "Summary of Results"
    echo "=================="
    echo ""
} > "$REPORT_FILE"

# Extract results from each test type
if [ "$RUN_DISTRIBUTED" = true ]; then
    {
        echo "=== Distributed System Benchmarks ==="
        if [ -f "${OUTPUT_DIR}/distributed_bench_output.txt" ]; then
            grep -A 5 -B 1 "time:" "${OUTPUT_DIR}/distributed_bench_output.txt" | grep -E "(time:|thrpt:)" || echo "No performance data found"
        fi
        echo ""
    } >> "$REPORT_FILE"
fi

if [ "$RUN_STRESS" = true ]; then
    {
        echo "=== Heavy Stress Testing ==="
        for test in memory cpu network all; do
            if [ -f "${OUTPUT_DIR}/heavy_stress_test_output.txt" ]; then
                echo "Stress Test: $test"
                grep -A 10 -B 5 "$test" "${OUTPUT_DIR}/heavy_stress_test_output.txt" || echo "No data found for $test"
                echo ""
            fi
        done
    } >> "$REPORT_FILE"
fi

if [ "$RUN_PERFORMANCE" = true ]; then
    {
        echo "=== Maximum Performance Testing ==="
        for test in throughput concurrency memory all; do
            if [ -f "${OUTPUT_DIR}/max_performance_output.txt" ]; then
                echo "Performance Test: $test"
                grep -A 10 -B 5 "$test" "${OUTPUT_DIR}/max_performance_output.txt" || echo "No data found for $test"
                echo ""
            fi
        done
    } >> "$REPORT_FILE"
fi

if [ "$RUN_SIMULATIONS" = true ]; then
    {
        echo "=== Distributed System Simulations ==="
        for sim in database microservice messagequeue cache all; do
            if [ -f "${OUTPUT_DIR}/distributed_simulator_output.txt" ]; then
                echo "Simulation: $sim"
                grep -A 10 -B 5 "$sim" "${OUTPUT_DIR}/distributed_simulator_output.txt" || echo "No data found for $sim"
                echo ""
            fi
        done
    } >> "$REPORT_FILE"
fi

# Generate summary statistics
echo -e "${BLUE}📊 Generating Summary Statistics...${NC}"
{
    echo "Performance Summary"
    echo "=================="
    echo ""
    
    # Find best performers across all tests
    echo "Top Performance Highlights:"
    echo "---------------------------"
    
    # Extract all throughput numbers and sort them
    find "$OUTPUT_DIR" -name "*_output.txt" -exec grep -h "thrpt:" {} \; | \
        sort -k2 -nr | head -10 | while read -r line; do
        echo "🚀 $line"
    done
    
    echo ""
    echo "Stress Test Results:"
    echo "-------------------"
    
    # Extract stress test results
    find "$OUTPUT_DIR" -name "*_output.txt" -exec grep -h "completed successfully\|failed\|error" {} \; | \
        head -20 | while read -r line; do
        if [[ $line == *"completed successfully"* ]]; then
            echo "✅ $line"
        elif [[ $line == *"failed"* ]] || [[ $line == *"error"* ]]; then
            echo "❌ $line"
        else
            echo "ℹ️  $line"
        fi
    done
    
} >> "$REPORT_FILE"

echo ""
echo -e "${GREEN}🎉 Comprehensive Benchmark Suite Completed!${NC}"
echo ""
echo -e "${BLUE}📁 Results Available:${NC}"
echo -e "  📊 Benchmark Results: ${GREEN}${OUTPUT_DIR}/${NC}"
echo -e "  📋 Comprehensive Report: ${GREEN}${REPORT_FILE}${NC}"
echo -e "  🔧 Binary Test Outputs: ${GREEN}${OUTPUT_DIR}/*_output.txt${NC}"
echo ""
echo -e "${BLUE}🔍 What Was Tested:${NC}"
if [ "$RUN_DISTRIBUTED" = true ]; then
    echo -e "  ✅ Distributed system performance"
fi
if [ "$RUN_STRESS" = true ]; then
    echo -e "  ✅ Extreme stress conditions"
fi
if [ "$RUN_PERFORMANCE" = true ]; then
    echo -e "  ✅ Maximum performance limits"
fi
if [ "$RUN_SIMULATIONS" = true ]; then
    echo -e "  ✅ Realistic distributed scenarios"
fi
echo ""
echo -e "${BLUE}🎯 This gives you complete performance numbers for every aspect of TTLog!${NC}"
echo -e "${BLUE}🚀 From basic operations to extreme distributed system performance!${NC}"
