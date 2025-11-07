#!/bin/bash
# Validation script for Docker deployment configuration

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo "=================================================="
echo "  Docker Deployment Configuration Validator"
echo "=================================================="
echo ""

# Track validation status
validation_errors=0

# Function to validate file exists
check_file() {
    local file=$1
    echo -n "Checking $file... "
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓ Found${NC}"
        return 0
    else
        echo -e "${RED}✗ Missing${NC}"
        validation_errors=$((validation_errors + 1))
        return 1
    fi
}

# Function to validate docker-compose syntax
validate_compose() {
    echo -n "Validating docker-compose.yml syntax... "
    if docker compose config > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Valid${NC}"
        return 0
    else
        echo -e "${RED}✗ Invalid${NC}"
        echo "Error details:"
        docker compose config 2>&1 | head -10
        validation_errors=$((validation_errors + 1))
        return 1
    fi
}

# Function to check for required ports
check_ports() {
    echo ""
    echo -e "${BLUE}Checking port availability:${NC}"

    local ports=(50051 9090 9091 3000)
    local port_issues=0

    for port in "${ports[@]}"; do
        echo -n "  Port $port... "
        if lsof -i :$port > /dev/null 2>&1; then
            echo -e "${YELLOW}⚠ In use${NC}"
            port_issues=$((port_issues + 1))
        else
            echo -e "${GREEN}✓ Available${NC}"
        fi
    done

    if [ $port_issues -gt 0 ]; then
        echo ""
        echo -e "${YELLOW}Warning: Some ports are in use. You may need to:${NC}"
        echo "  1. Stop conflicting services"
        echo "  2. Change ports in .env file"
        echo "  3. Use different port mappings"
    fi

    return 0
}

# Function to validate environment file
check_env() {
    echo -n "Checking environment configuration... "
    if [ -f ".env" ]; then
        echo -e "${GREEN}✓ .env exists${NC}"

        # Check for required variables (optional check)
        if grep -q "GRAFANA_ADMIN_PASSWORD=admin" .env; then
            echo -e "  ${YELLOW}⚠ Warning: Using default Grafana password${NC}"
        fi
    else
        echo -e "${YELLOW}⚠ .env not found (will use .env.example)${NC}"
    fi
}

# Function to check Docker and Docker Compose
check_docker() {
    echo ""
    echo -e "${BLUE}Checking Docker installation:${NC}"

    echo -n "  Docker daemon... "
    if docker info > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Running${NC}"
        echo "    Version: $(docker --version | cut -d' ' -f3)"
    else
        echo -e "${RED}✗ Not running${NC}"
        validation_errors=$((validation_errors + 1))
        return 1
    fi

    echo -n "  Docker Compose... "
    if docker compose version > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Installed${NC}"
        echo "    Version: $(docker compose version --short)"
    else
        echo -e "${RED}✗ Not installed${NC}"
        validation_errors=$((validation_errors + 1))
        return 1
    fi
}

# Main validation
echo -e "${BLUE}Validating configuration files:${NC}"

# Check all required files
check_file "Dockerfile"
check_file "docker-compose.yml"
check_file "prometheus.yml"
check_file "grafana-datasources.yml"
check_file "grafana-dashboards.yml"
check_file ".dockerignore"
check_file ".env.example"
check_file "README.md"

echo ""

# Validate docker-compose syntax
validate_compose

# Check environment
echo ""
check_env

# Check Docker installation
check_docker

# Check ports
check_ports

# Check directory structure
echo ""
echo -e "${BLUE}Checking directory structure:${NC}"

echo -n "  Data directory... "
if [ -d "./data" ]; then
    echo -e "${GREEN}✓ Exists${NC}"
elif [ -d "../../data" ]; then
    echo -e "${GREEN}✓ Exists (root level)${NC}"
else
    echo -e "${YELLOW}⚠ Not found (will be created)${NC}"
fi

echo -n "  Grafana dashboards... "
if [ -d "../../grafana" ]; then
    dashboard_count=$(find ../../grafana -name "*.json" | wc -l)
    echo -e "${GREEN}✓ Found ($dashboard_count dashboards)${NC}"
else
    echo -e "${YELLOW}⚠ Not found${NC}"
fi

# Check build context
echo ""
echo -e "${BLUE}Checking build context:${NC}"

echo -n "  Cargo.toml... "
if [ -f "../../Cargo.toml" ]; then
    echo -e "${GREEN}✓ Found${NC}"
else
    echo -e "${RED}✗ Missing${NC}"
    validation_errors=$((validation_errors + 1))
fi

echo -n "  src/ directory... "
if [ -d "../../src" ]; then
    echo -e "${GREEN}✓ Found${NC}"
else
    echo -e "${RED}✗ Missing${NC}"
    validation_errors=$((validation_errors + 1))
fi

echo -n "  proto/ directory... "
if [ -d "../../proto" ]; then
    proto_count=$(find ../../proto -name "*.proto" | wc -l)
    echo -e "${GREEN}✓ Found ($proto_count proto files)${NC}"
else
    echo -e "${RED}✗ Missing${NC}"
    validation_errors=$((validation_errors + 1))
fi

# Final summary
echo ""
echo "=================================================="

if [ $validation_errors -eq 0 ]; then
    echo -e "${GREEN}✓ All validations passed!${NC}"
    echo ""
    echo "You can now deploy with:"
    echo "  ./start.sh"
    echo ""
    echo "Or manually:"
    echo "  docker compose up -d"
    exit 0
else
    echo -e "${RED}✗ Validation failed with $validation_errors error(s)${NC}"
    echo ""
    echo "Please fix the errors above before deploying."
    echo "See README.md for more information."
    exit 1
fi
