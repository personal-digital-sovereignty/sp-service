#!/bin/bash
# sp-service E2E Tests - Run Script
# Usage: ./run_tests.sh [options]

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVICE_DIR="$(dirname "$SCRIPT_DIR")"
E2E_DIR="$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Functions
print_header() {
    echo -e "\n${GREEN}========================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${GREEN}========================================${NC}\n"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_success() {
    echo -e "${GREEN}SUCCESS: $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

check_service() {
    local url="${SP_SERVICE_URL:-http://localhost:8080}"
    local max_retries=10
    local retry_delay=2
    
    echo "Checking service availability at $url..."
    
    for i in $(seq 1 $max_retries); do
        if curl -s -o /dev/null -w "%{http_code}" "$url/health" | grep -q "200"; then
            echo "Service is available!"
            return 0
        fi
        
        if [ $i -lt $max_retries ]; then
            echo "Service not ready, retrying in ${retry_delay}s... (attempt $i/$max_retries)"
            sleep $retry_delay
        fi
    done
    
    print_error "Service not available after $max_retries retries"
    return 1
}

start_service() {
    echo "Starting sp-service..."
    cd "$SERVICE_DIR"
    cargo run --release &
    SERVICE_PID=$!
    echo "sp-service started with PID $SERVICE_PID"
    
    # Wait for service to be ready
    sleep 10
    
    cd "$E2E_DIR"
}

stop_service() {
    if [ ! -z "$SERVICE_PID" ]; then
        echo "Stopping sp-service (PID $SERVICE_PID)..."
        kill $SERVICE_PID 2>/dev/null || true
        print_success "sp-service stopped"
    fi
}

install_deps() {
    echo "Installing Python dependencies..."
    cd "$E2E_DIR"
    pip install -r requirements.txt
    print_success "Dependencies installed"
    cd "$SCRIPT_DIR"
}

run_tests() {
    local test_args="$@"
    
    cd "$E2E_DIR"
    
    if [ -z "$test_args" ]; then
        echo "Running all E2E tests..."
        pytest tests/ -v --tb=short
    else
        echo "Running tests: $test_args"
        pytest tests/ $test_args
    fi
    
    local exit_code=$?
    
    cd "$SCRIPT_DIR"
    
    if [ $exit_code -eq 0 ]; then
        print_success "All tests passed!"
    else
        print_error "Some tests failed (exit code: $exit_code)"
    fi
    
    return $exit_code
}

# Parse command line arguments
START_SERVICE=false
STOP_SERVICE=false
INSTALL_DEPS=false
TEST_FILTER=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --start)
            START_SERVICE=true
            shift
            ;;
        --stop)
            STOP_SERVICE=true
            shift
            ;;
        --install-deps)
            INSTALL_DEPS=true
            shift
            ;;
        --filter)
            TEST_FILTER="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --start          Start sp-service before tests"
            echo "  --stop           Stop sp-service after tests"
            echo "  --install-deps   Install Python dependencies"
            echo "  --filter FILTER  Run specific tests (pytest filter)"
            echo "  --help           Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --install-deps --start --stop"
            echo "  $0 --filter 'test_health.py'"
            echo "  $0 --filter 'test_chat_api.py::TestChatCompletions::test_chat_basic'"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Main execution
print_header "sp-service E2E Test Runner"

# Install dependencies if requested
if [ "$INSTALL_DEPS" = true ]; then
    install_deps
fi

# Start service if requested
if [ "$START_SERVICE" = true ]; then
    start_service
    trap stop_service EXIT
fi

# Check service availability
if ! check_service; then
    print_error "Please start sp-service first or use --start flag"
    exit 1
fi

# Run tests
run_tests $TEST_FILTER
exit_code=$?

# Summary
print_header "Test Summary"
echo "Exit code: $exit_code"

if [ $exit_code -eq 0 ]; then
    print_success "E2E tests completed successfully!"
else
    print_error "E2E tests failed!"
fi

exit $exit_code
