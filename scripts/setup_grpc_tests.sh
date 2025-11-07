#!/bin/bash
# Setup script for gRPC integration tests
# This script installs prerequisites and validates the test environment

set -e

echo "=========================================="
echo "gRPC Integration Test Setup"
echo "=========================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if protoc is installed
echo "Checking for protoc (Protocol Buffer Compiler)..."
if command -v protoc &> /dev/null; then
    PROTOC_VERSION=$(protoc --version)
    echo -e "${GREEN}✓${NC} protoc found: $PROTOC_VERSION"
else
    echo -e "${RED}✗${NC} protoc not found"
    echo ""
    echo "Installing protoc..."

    # Detect OS
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "Detected Linux. Installing via apt..."
        sudo apt-get update
        sudo apt-get install -y protobuf-compiler
    elif [[ "$OSTYPE" == "darwin"* ]]; then
        echo "Detected macOS. Installing via Homebrew..."
        if command -v brew &> /dev/null; then
            brew install protobuf
        else
            echo -e "${RED}Error:${NC} Homebrew not found. Please install Homebrew first:"
            echo "  /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            exit 1
        fi
    else
        echo -e "${YELLOW}Warning:${NC} Unsupported OS. Please install protoc manually:"
        echo "  Ubuntu/Debian: sudo apt-get install protobuf-compiler"
        echo "  macOS: brew install protobuf"
        echo "  Or download from: https://github.com/protocolbuffers/protobuf/releases"
        exit 1
    fi

    # Verify installation
    if command -v protoc &> /dev/null; then
        PROTOC_VERSION=$(protoc --version)
        echo -e "${GREEN}✓${NC} protoc installed successfully: $PROTOC_VERSION"
    else
        echo -e "${RED}✗${NC} Failed to install protoc"
        exit 1
    fi
fi

echo ""
echo "Checking Rust toolchain..."
if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    echo -e "${GREEN}✓${NC} Cargo found: $CARGO_VERSION"
else
    echo -e "${RED}✗${NC} Cargo not found. Please install Rust:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo ""
echo "Checking project dependencies..."
echo "Running cargo check..."
if cargo check --quiet 2>&1 | grep -q "error"; then
    echo -e "${YELLOW}Warning:${NC} Some build issues detected. Running detailed check..."
    cargo check
else
    echo -e "${GREEN}✓${NC} All dependencies resolved"
fi

echo ""
echo "Verifying proto files..."
if [ -f "proto/memory_graph.proto" ]; then
    echo -e "${GREEN}✓${NC} proto/memory_graph.proto found"
else
    echo -e "${RED}✗${NC} proto/memory_graph.proto not found"
    exit 1
fi

echo ""
echo "Verifying build script..."
if [ -f "build.rs" ]; then
    echo -e "${GREEN}✓${NC} build.rs found"
else
    echo -e "${RED}✗${NC} build.rs not found"
    exit 1
fi

echo ""
echo "Verifying test file..."
if [ -f "tests/grpc_integration_test.rs" ]; then
    echo -e "${GREEN}✓${NC} tests/grpc_integration_test.rs found"

    # Count tests
    TEST_COUNT=$(grep -c "^#\[tokio::test\]" tests/grpc_integration_test.rs || true)
    echo "  Found $TEST_COUNT integration tests"
else
    echo -e "${RED}✗${NC} tests/grpc_integration_test.rs not found"
    exit 1
fi

echo ""
echo "Building project with gRPC support..."
if cargo build --quiet 2>&1 | grep -q "error\|failed"; then
    echo -e "${YELLOW}Warning:${NC} Build encountered issues. Running detailed build..."
    cargo build
    BUILD_STATUS=$?
else
    echo -e "${GREEN}✓${NC} Project built successfully"
    BUILD_STATUS=0
fi

echo ""
echo "=========================================="
echo "Setup Complete!"
echo "=========================================="
echo ""

if [ $BUILD_STATUS -eq 0 ]; then
    echo "You can now run the tests with:"
    echo "  cargo test --test grpc_integration_test"
    echo ""
    echo "Or run specific tests:"
    echo "  cargo test --test grpc_integration_test test_health_check"
    echo ""
    echo "For verbose output:"
    echo "  cargo test --test grpc_integration_test -- --nocapture"
else
    echo -e "${YELLOW}Note:${NC} Build completed with warnings/errors."
    echo "Please review the output above and fix any issues before running tests."
fi

echo ""
echo "For more information, see:"
echo "  docs/GRPC_INTEGRATION_TEST_REPORT.md"
