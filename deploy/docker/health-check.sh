#!/bin/bash
# Health check script for LLM-Memory-Graph deployment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=================================================="
echo "  LLM-Memory-Graph Health Check"
echo "=================================================="
echo ""

# Function to check service health
check_service() {
    local service=$1
    local check_command=$2

    echo -n "Checking $service... "

    if eval "$check_command" &> /dev/null; then
        echo -e "${GREEN}✓ Healthy${NC}"
        return 0
    else
        echo -e "${RED}✗ Unhealthy${NC}"
        return 1
    fi
}

# Function to check HTTP endpoint
check_http() {
    local name=$1
    local url=$2

    echo -n "Checking $name... "

    if curl -s -f "$url" > /dev/null; then
        echo -e "${GREEN}✓ Responding${NC}"
        return 0
    else
        echo -e "${RED}✗ Not responding${NC}"
        return 1
    fi
}

# Check if services are running
echo "Container Status:"
docker compose ps
echo ""

# Initialize health status
overall_health=0

# Check Memory Graph service
if docker compose ps | grep -q "memory-graph.*running"; then
    check_service "Memory Graph gRPC" "docker exec llm-memory-graph /usr/local/bin/grpc_health_probe -addr=:50051" || overall_health=1
    check_http "Memory Graph Metrics" "http://localhost:9090/metrics" || overall_health=1
else
    echo -e "${RED}✗ Memory Graph service is not running${NC}"
    overall_health=1
fi

# Check Prometheus
if docker compose ps | grep -q "prometheus.*running"; then
    check_http "Prometheus API" "http://localhost:9091/-/healthy" || overall_health=1
    check_http "Prometheus Targets" "http://localhost:9091/api/v1/targets" || overall_health=1
else
    echo -e "${RED}✗ Prometheus service is not running${NC}"
    overall_health=1
fi

# Check Grafana
if docker compose ps | grep -q "grafana.*running"; then
    check_http "Grafana API" "http://localhost:3000/api/health" || overall_health=1
else
    echo -e "${RED}✗ Grafana service is not running${NC}"
    overall_health=1
fi

echo ""
echo "=================================================="

if [ $overall_health -eq 0 ]; then
    echo -e "${GREEN}All services are healthy!${NC}"
    exit 0
else
    echo -e "${RED}Some services are unhealthy!${NC}"
    echo ""
    echo "Troubleshooting steps:"
    echo "  1. Check logs: docker compose logs -f"
    echo "  2. Check container status: docker compose ps"
    echo "  3. Restart services: docker compose restart"
    echo "  4. See README.md for more help"
    exit 1
fi
