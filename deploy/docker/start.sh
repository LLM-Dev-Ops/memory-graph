#!/bin/bash
# Quick start script for LLM-Memory-Graph Docker deployment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=================================================="
echo "  LLM-Memory-Graph Docker Deployment"
echo "=================================================="
echo ""

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}Error: Docker is not installed${NC}"
    echo "Please install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is installed
if ! docker compose version &> /dev/null; then
    echo -e "${RED}Error: Docker Compose is not installed${NC}"
    echo "Please install Docker Compose: https://docs.docker.com/compose/install/"
    exit 1
fi

echo -e "${GREEN}✓ Docker and Docker Compose are installed${NC}"
echo ""

# Check for .env file
if [ ! -f .env ]; then
    echo -e "${YELLOW}Warning: .env file not found${NC}"
    echo "Creating .env from .env.example..."
    cp .env.example .env
    echo -e "${GREEN}✓ Created .env file${NC}"
    echo -e "${YELLOW}Please edit .env file with your configuration before continuing${NC}"
    echo ""
    read -p "Press Enter to continue or Ctrl+C to exit..."
fi

# Create data directory
if [ ! -d "./data" ]; then
    echo "Creating data directory..."
    mkdir -p ./data
    echo -e "${GREEN}✓ Created data directory${NC}"
fi

echo ""
echo "Starting services..."
echo ""

# Pull latest images
docker compose pull

# Build the image
docker compose build

# Start services
docker compose up -d

echo ""
echo "Waiting for services to start..."
sleep 10

# Check service status
echo ""
echo "Service Status:"
docker compose ps

echo ""
echo "=================================================="
echo "  Deployment Complete!"
echo "=================================================="
echo ""
echo "Access your services at:"
echo -e "  ${GREEN}gRPC Service:${NC}     localhost:50051"
echo -e "  ${GREEN}Metrics:${NC}          http://localhost:9090/metrics"
echo -e "  ${GREEN}Prometheus:${NC}       http://localhost:9091"
echo -e "  ${GREEN}Grafana:${NC}          http://localhost:3000"
echo ""
echo "Default Grafana credentials:"
echo "  Username: admin"
echo "  Password: admin (change on first login)"
echo ""
echo "Useful commands:"
echo "  View logs:           docker compose logs -f"
echo "  Stop services:       docker compose stop"
echo "  Restart services:    docker compose restart"
echo "  Remove everything:   docker compose down -v"
echo ""
echo "For more information, see README.md"
echo "=================================================="
