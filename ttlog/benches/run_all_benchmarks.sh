#!/bin/bash

# TTLog Comprehensive Benchmark Runner
# Provides reliable and configurable benchmark execution for the TTLog library

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
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BENCHMARK_DIR="$SCRIPT_DIR"
REPORT_DIR="$PROJECT_ROOT/benchmark_reports"
LOG_DIR="$PROJECT_ROOT/benchmark_logs"

# Default settings
QUICK_MODE=false
VERBOSE=false
RUN_DISTRIBUTED=true
RUN_STRESS=true
RUN_PERFORMANCE=true
RUN_SIMULATIONS=true
GENERATE_REPORT=true
CLEAN_SNAPSHOTS=true

# Benchmark configuration
CRITERION_SAMPLE_SIZE=${CRITERION_SAMPLE_SIZE:-30}
CRITERION_MEASUREMENT_TIME=${CRITERION_MEASUREMENT_TIME:-10000}
CRITERION_WARM_UP_TIME=${CRITERION_WARM_UP_TIME:-5000}

# Quick mode overrides
if [ "$QUICK_MODE" = true ]; then
    CRITERION_SAMPLE_SIZE=10
    CRITERION_MEASUREMENT_TIME=2000
    CRITERION_WARM_UP_TIME=500
fi

# Functions
print_header() {
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}========================================${NC}"
}

print_section() {
    echo -e "${CYAN}$1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${PURPLE}ℹ️  $1${NC}"
}

# Create necessary directories
create_directories() {
    mkdir -p "$REPORT_DIR"
    mkdir -p "$LOG_DIR"
    print_success "Created benchmark directories"
}

# Get system information
get_system_info() {
    print_section "System Information"
    
    echo "CPU: $(nproc) cores"
    echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
    echo "Rust Version: $(rustc --version)"
    echo "Cargo Version: $(cargo --version)"
    echo "OS: $(uname -a)"
    echo "Architecture: $(uname -m)"
    
    # Check for performance governors
    if [ -f /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
        echo "CPU Governor: $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
    fi
    
    # Check for NUMA nodes
    if command -v numactl >/dev/null 2>&1; then
        echo "NUMA Nodes: $(numactl --hardware | grep 'available:' | awk '{print $2}')"
    fi
    
    echo ""
}

# Check prerequisites
check_prerequisites() {
    print_section "Checking Prerequisites"
    
    # Check if we're in the right directory
    if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
        print_error "Not in TTLog project root directory"
        exit 1
    fi
    
    # Check Rust toolchain
    if ! command -v rustc >/dev/null 2>&1; then
        print_error "Rust toolchain not found"
        exit 1
    fi
    
    # Check Cargo
    if ! command -v cargo >/dev/null 2>&1; then
        print_error "Cargo not found"
        exit 1
    fi
    
    # Check for nightly toolchain (for some benchmarks)
    if ! rustup toolchain list | grep -q nightly; then
        print_warning "Nightly toolchain not installed (some benchmarks may fail)"
    fi
    
    print_success "Prerequisites check passed"
    echo ""
}

# Clean up before benchmarks
cleanup_before() {
    print_section "Cleaning Up Before Benchmarks"
    
    if [ "$CLEAN_SNAPSHOTS" = true ]; then
        find /tmp -name "ttlog-*.bin" -delete 2>/dev/null || true
        print_success "Cleaned snapshot files"
    fi
    
    # Clean build artifacts
    cargo clean --workspace >/dev/null 2>&1 || true
    print_success "Cleaned build artifacts"
    echo ""
}

# Run Criterion benchmarks
run_criterion_benchmarks() {
    print_section "Running Criterion Benchmarks"
    
    local log_file="$LOG_DIR/criterion_benchmarks.log"
    
    print_info "Sample Size: $CRITERION_SAMPLE_SIZE"
    print_info "Measurement Time: ${CRITERION_MEASUREMENT_TIME}ms"
    print_info "Warm-up Time: ${CRITERION_WARM_UP_TIME}ms"
    
    export CRITERION_SAMPLE_SIZE
    export CRITERION_MEASUREMENT_TIME
    export CRITERION_WARM_UP_TIME
    
    if [ "$VERBOSE" = true ]; then
        cargo bench --workspace 2>&1 | tee "$log_file"
    else
        cargo bench --workspace > "$log_file" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Criterion benchmarks completed"
    else
        print_error "Criterion benchmarks failed"
        return 1
    fi
    
    echo ""
}

# Run distributed benchmarks
run_distributed_benchmarks() {
    if [ "$RUN_DISTRIBUTED" != true ]; then
        return 0
    fi
    
    print_section "Running Distributed System Benchmarks"
    
    local log_file="$LOG_DIR/distributed_benchmarks.log"
    
    if [ "$VERBOSE" = true ]; then
        cargo run --bin distributed_bench 2>&1 | tee "$log_file"
    else
        cargo run --bin distributed_bench > "$log_file" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Distributed benchmarks completed"
    else
        print_error "Distributed benchmarks failed"
        return 1
    fi
    
    echo ""
}

# Run stress tests
run_stress_tests() {
    if [ "$RUN_STRESS" != true ]; then
        return 0
    fi
    
    print_section "Running Stress Tests"
    
    local log_file="$LOG_DIR/stress_tests.log"
    
    # Run heavy stress test
    print_info "Running heavy stress test..."
    if [ "$VERBOSE" = true ]; then
        cargo run --bin heavy_stress_test all 2>&1 | tee -a "$log_file"
    else
        cargo run --bin heavy_stress_test all >> "$log_file" 2>&1
    fi
    
    # Run max performance test
    print_info "Running max performance test..."
    if [ "$VERBOSE" = true ]; then
        cargo run --bin max_performance all 2>&1 | tee -a "$log_file"
    else
        cargo run --bin max_performance all >> "$log_file" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Stress tests completed"
    else
        print_error "Stress tests failed"
        return 1
    fi
    
    echo ""
}

# Run performance tests
run_performance_tests() {
    if [ "$RUN_PERFORMANCE" != true ]; then
        return 0
    fi
    
    print_section "Running Performance Tests"
    
    local log_file="$LOG_DIR/performance_tests.log"
    
    if [ "$VERBOSE" = true ]; then
        cargo run --bin test_performance 2>&1 | tee "$log_file"
    else
        cargo run --bin test_performance > "$log_file" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Performance tests completed"
    else
        print_error "Performance tests failed"
        return 1
    fi
    
    echo ""
}

# Run simulations
run_simulations() {
    if [ "$RUN_SIMULATIONS" != true ]; then
        return 0
    fi
    
    print_section "Running Distributed Simulations"
    
    local log_file="$LOG_DIR/simulations.log"
    
    if [ "$VERBOSE" = true ]; then
        cargo run --bin distributed_simulator all 2>&1 | tee "$log_file"
    else
        cargo run --bin distributed_simulator all > "$log_file" 2>&1
    fi
    
    if [ $? -eq 0 ]; then
        print_success "Simulations completed"
    else
        print_error "Simulations failed"
        return 1
    fi
    
    echo ""
}

# Generate comprehensive report
generate_report() {
    if [ "$GENERATE_REPORT" != true ]; then
        return 0
    fi
    
    print_section "Generating Comprehensive Report"
    
    local report_file="$REPORT_DIR/comprehensive_benchmark_report.txt"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    # Create report header
    cat > "$report_file" << EOF
TTLog Comprehensive Benchmark Report
====================================
Generated: $timestamp

System Information:
==================
CPU: $(nproc) cores
Memory: $(free -h | grep Mem | awk '{print $2}')
Rust Version: $(rustc --version)
OS: $(uname -a)
Architecture: $(uname -m)

Benchmark Configuration:
=======================
Sample Size: $CRITERION_SAMPLE_SIZE
Measurement Time: ${CRITERION_MEASUREMENT_TIME}ms
Warm-up Time: ${CRITERION_WARM_UP_TIME}ms
Quick Mode: $QUICK_MODE

EOF
    
    # Add Criterion results if available
    if [ -f "$LOG_DIR/criterion_benchmarks.log" ]; then
        echo "Criterion Benchmark Results:" >> "$report_file"
        echo "============================" >> "$report_file"
        cat "$LOG_DIR/criterion_benchmarks.log" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add distributed benchmark results
    if [ -f "$LOG_DIR/distributed_benchmarks.log" ]; then
        echo "Distributed Benchmark Results:" >> "$report_file"
        echo "==============================" >> "$report_file"
        cat "$LOG_DIR/distributed_benchmarks.log" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add stress test results
    if [ -f "$LOG_DIR/stress_tests.log" ]; then
        echo "Stress Test Results:" >> "$report_file"
        echo "====================" >> "$report_file"
        cat "$LOG_DIR/stress_tests.log" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add performance test results
    if [ -f "$LOG_DIR/performance_tests.log" ]; then
        echo "Performance Test Results:" >> "$report_file"
        echo "=========================" >> "$report_file"
        cat "$LOG_DIR/performance_tests.log" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add simulation results
    if [ -f "$LOG_DIR/simulations.log" ]; then
        echo "Simulation Results:" >> "$report_file"
        echo "===================" >> "$report_file"
        cat "$LOG_DIR/simulations.log" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add summary
    echo "Summary:" >> "$report_file"
    echo "========" >> "$report_file"
    echo "Total benchmark time: $(date -d@$SECONDS -u +%H:%M:%S)" >> "$report_file"
    echo "Benchmarks completed successfully!" >> "$report_file"
    
    print_success "Comprehensive report generated: $report_file"
    echo ""
}

# Show usage
show_usage() {
    cat << EOF
TTLog Comprehensive Benchmark Runner

Usage: $0 [OPTIONS]

Options:
    -h, --help              Show this help message
    -q, --quick             Run in quick mode (fewer samples, shorter times)
    -v, --verbose           Verbose output
    --no-distributed        Skip distributed benchmarks
    --no-stress             Skip stress tests
    --no-performance        Skip performance tests
    --no-simulations        Skip simulations
    --no-report             Don't generate comprehensive report
    --no-clean              Don't clean snapshots before running

Environment Variables:
    CRITERION_SAMPLE_SIZE       Number of samples (default: 30, quick: 10)
    CRITERION_MEASUREMENT_TIME  Measurement time in ms (default: 10000, quick: 2000)
    CRITERION_WARM_UP_TIME      Warm-up time in ms (default: 5000, quick: 500)

Examples:
    $0                    # Run all benchmarks with default settings
    $0 --quick           # Run quick benchmarks for faster feedback
    $0 --verbose         # Run with verbose output
    $0 --no-stress       # Skip stress tests
    $0 --no-report       # Don't generate report

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                show_usage
                exit 0
                ;;
            -q|--quick)
                QUICK_MODE=true
                CRITERION_SAMPLE_SIZE=10
                CRITERION_MEASUREMENT_TIME=2000
                CRITERION_WARM_UP_TIME=500
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --no-distributed)
                RUN_DISTRIBUTED=false
                shift
                ;;
            --no-stress)
                RUN_STRESS=false
                shift
                ;;
            --no-performance)
                RUN_PERFORMANCE=false
                shift
                ;;
            --no-simulations)
                RUN_SIMULATIONS=false
                shift
                ;;
            --no-report)
                GENERATE_REPORT=false
                shift
                ;;
            --no-clean)
                CLEAN_SNAPSHOTS=false
                shift
                ;;
            *)
                print_error "Unknown option: $1"
                show_usage
                exit 1
                ;;
        esac
    done
}

# Main function
main() {
    local start_time=$SECONDS
    
    print_header "TTLog Comprehensive Benchmark Runner"
    
    # Parse arguments
    parse_args "$@"
    
    # Change to project root
    cd "$PROJECT_ROOT"
    
    # Run benchmarks
    check_prerequisites
    create_directories
    get_system_info
    cleanup_before
    
    local failed=false
    
    # Run all benchmark types
    run_criterion_benchmarks || failed=true
    run_distributed_benchmarks || failed=true
    run_stress_tests || failed=true
    run_performance_tests || failed=true
    run_simulations || failed=true
    
    # Generate report
    generate_report
    
    # Final summary
    local elapsed_time=$((SECONDS - start_time))
    print_header "Benchmark Summary"
    
    if [ "$failed" = true ]; then
        print_error "Some benchmarks failed. Check logs in $LOG_DIR"
        exit 1
    else
        print_success "All benchmarks completed successfully!"
        print_info "Total time: $(date -d@$elapsed_time -u +%H:%M:%S)"
        print_info "Logs available in: $LOG_DIR"
        print_info "Report available in: $REPORT_DIR"
        
        if [ "$GENERATE_REPORT" = true ]; then
            print_info "Comprehensive report: $REPORT_DIR/comprehensive_benchmark_report.txt"
        fi
    fi
}

# Run main function with all arguments
main "$@"
