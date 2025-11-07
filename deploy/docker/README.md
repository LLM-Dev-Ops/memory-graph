# Docker Deployment Guide for LLM-Memory-Graph

Complete Docker deployment configuration for the LLM-Memory-Graph gRPC service with integrated Prometheus monitoring and Grafana visualization.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Services](#services)
- [Security](#security)
- [Production Deployment](#production-deployment)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)
- [Advanced Topics](#advanced-topics)

## Overview

This deployment includes:

- **LLM-Memory-Graph gRPC Server**: Main application service
- **Prometheus**: Metrics collection and storage
- **Grafana**: Visualization and dashboards

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Docker Network                         │
│                  (172.28.0.0/16)                         │
│                                                           │
│  ┌──────────────────┐      ┌──────────────────┐         │
│  │  Memory Graph    │      │   Prometheus     │         │
│  │  gRPC Service    │─────▶│  Metrics Store   │         │
│  │  Port: 50051     │      │  Port: 9091      │         │
│  │  Metrics: 9090   │      └──────────────────┘         │
│  └──────────────────┘               │                    │
│                                     │                    │
│                                     ▼                    │
│                          ┌──────────────────┐            │
│                          │     Grafana      │            │
│                          │  Visualization   │            │
│                          │   Port: 3000     │            │
│                          └──────────────────┘            │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

## Prerequisites

### Required Software

- **Docker**: Version 20.10.0 or later
- **Docker Compose**: Version 2.0.0 or later

### System Requirements

- **CPU**: 2+ cores recommended
- **Memory**: 4GB minimum, 8GB recommended
- **Disk Space**: 20GB+ for data and images
- **OS**: Linux, macOS, or Windows with WSL2

### Verify Installation

```bash
docker --version
docker compose version
```

## Quick Start

### 1. Clone Repository

```bash
git clone https://github.com/globalbusinessadvisors/llm-memory-graph.git
cd llm-memory-graph/deploy/docker
```

### 2. Create Environment File

```bash
cp .env.example .env
# Edit .env with your configuration
nano .env
```

### 3. Create Data Directory

```bash
mkdir -p ./data
```

### 4. Start Services

```bash
docker compose up -d
```

### 5. Verify Services

```bash
# Check service status
docker compose ps

# View logs
docker compose logs -f memory-graph

# Test gRPC health
docker exec llm-memory-graph /usr/local/bin/grpc_health_probe -addr=:50051
```

### 6. Access Services

- **Grafana**: http://localhost:3000 (admin/admin)
- **Prometheus**: http://localhost:9091
- **Metrics**: http://localhost:9090/metrics

## Configuration

### Environment Variables

Create a `.env` file in the `deploy/docker` directory:

```bash
# Core Configuration
GRPC_PORT=50051
METRICS_PORT=9090
RUST_LOG=info

# Prometheus Configuration
PROMETHEUS_PORT=9091
PROMETHEUS_RETENTION=30d

# Grafana Configuration
GRAFANA_PORT=3000
GRAFANA_ADMIN_USER=admin
GRAFANA_ADMIN_PASSWORD=your-secure-password-here

# Data Storage
DATA_DIR=./data

# Optional: LLM-Registry Integration
REGISTRY_URL=http://llm-registry:8080
REGISTRY_API_KEY=your-registry-key

# Optional: Data-Vault Integration
VAULT_URL=http://data-vault:9000
VAULT_API_KEY=your-vault-key
```

### Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `Dockerfile` | Multi-stage build configuration | `/deploy/docker/` |
| `docker-compose.yml` | Service orchestration | `/deploy/docker/` |
| `prometheus.yml` | Prometheus scrape config | `/deploy/docker/` |
| `grafana-datasources.yml` | Grafana datasource setup | `/deploy/docker/` |
| `grafana-dashboards.yml` | Dashboard provisioning | `/deploy/docker/` |
| `.dockerignore` | Build context exclusions | `/deploy/docker/` |

## Services

### Memory Graph gRPC Service

**Container**: `llm-memory-graph`

**Ports**:
- `50051`: gRPC service endpoint
- `9090`: Prometheus metrics endpoint

**Volumes**:
- `memory-graph-data:/data` - Persistent storage
- `./plugins:/plugins:ro` - Plugin directory (read-only)

**Health Check**:
```bash
docker exec llm-memory-graph /usr/local/bin/grpc_health_probe -addr=:50051
```

### Prometheus

**Container**: `llm-memory-graph-prometheus`

**Ports**:
- `9091`: Prometheus UI and API

**Data Retention**: 30 days (configurable)

**Access**:
```bash
curl http://localhost:9091/api/v1/status/config
```

### Grafana

**Container**: `llm-memory-graph-grafana`

**Ports**:
- `3000`: Web interface

**Default Credentials**:
- Username: `admin`
- Password: `admin` (change on first login)

**Pre-loaded Dashboards**:
- Memory Graph Overview
- Additional dashboards from `/grafana` directory

## Security

### Production Security Checklist

- [ ] Change default Grafana password
- [ ] Use strong passwords for all services
- [ ] Enable TLS for gRPC (see [TLS Configuration](#tls-configuration))
- [ ] Restrict network access with firewall rules
- [ ] Use secrets management (Docker Secrets or Vault)
- [ ] Regular security updates
- [ ] Enable authentication for Prometheus
- [ ] Use read-only volumes where possible
- [ ] Run services as non-root users (already configured)
- [ ] Implement rate limiting
- [ ] Enable audit logging

### TLS Configuration

To enable TLS for gRPC:

1. Generate certificates:
```bash
mkdir -p ./certs
# Generate your certificates
```

2. Mount certificates in `docker-compose.yml`:
```yaml
volumes:
  - ./certs:/certs:ro
environment:
  - TLS_CERT_PATH=/certs/server.crt
  - TLS_KEY_PATH=/certs/server.key
```

### API Key Management

For production, use Docker secrets:

```bash
echo "your-registry-key" | docker secret create registry_api_key -
echo "your-vault-key" | docker secret create vault_api_key -
```

Update `docker-compose.yml`:
```yaml
secrets:
  - registry_api_key
  - vault_api_key
```

## Production Deployment

### Resource Limits

Update `docker-compose.yml` for your production environment:

```yaml
deploy:
  resources:
    limits:
      cpus: '4.0'
      memory: 8G
    reservations:
      cpus: '2.0'
      memory: 4G
```

### High Availability

For HA deployment:

1. **Database Replication**: Use distributed Sled or external storage
2. **Load Balancing**: Deploy multiple instances behind a load balancer
3. **Health Checks**: Configure aggressive health checks
4. **Backup Strategy**: Regular backups of data volumes

### Scaling

Scale the service:

```bash
docker compose up -d --scale memory-graph=3
```

Note: Requires load balancer configuration.

### Backup and Restore

**Backup**:
```bash
# Stop services
docker compose stop memory-graph

# Backup data
tar czf backup-$(date +%Y%m%d).tar.gz ./data

# Restart services
docker compose start memory-graph
```

**Restore**:
```bash
# Stop services
docker compose down

# Restore data
tar xzf backup-YYYYMMDD.tar.gz

# Start services
docker compose up -d
```

## Monitoring

### Metrics Endpoints

- **Application Metrics**: http://localhost:9090/metrics
- **Prometheus Metrics**: http://localhost:9091/metrics
- **Grafana Metrics**: http://localhost:3000/metrics

### Key Metrics to Monitor

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `memory_graph_write_latency_seconds_p95` | Write latency | > 50ms |
| `memory_graph_read_latency_seconds_p95` | Read latency | > 50ms |
| `memory_graph_active_sessions` | Active sessions | N/A |
| `memory_graph_nodes_created_total` | Total nodes | N/A |
| `memory_graph_edges_created_total` | Total edges | N/A |

### Grafana Dashboards

Access Grafana at http://localhost:3000 and navigate to:

1. **LLM Systems** folder
2. Select **Memory Graph Overview** dashboard

### Log Viewing

```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f memory-graph

# Last 100 lines
docker compose logs --tail=100 memory-graph

# Follow with timestamps
docker compose logs -f -t memory-graph
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs
docker compose logs memory-graph

# Verify configuration
docker compose config

# Check ports
sudo netstat -tulpn | grep -E '50051|9090|3000'
```

### Connection Issues

```bash
# Test gRPC connection
docker exec llm-memory-graph /usr/local/bin/grpc_health_probe -addr=:50051

# Check network
docker network inspect deploy_llm-network

# Verify service discovery
docker compose exec memory-graph ping prometheus
```

### Performance Issues

1. Check resource usage:
```bash
docker stats
```

2. Increase memory limits in `docker-compose.yml`

3. Review application logs for bottlenecks

4. Check Prometheus/Grafana for metrics

### Data Persistence Issues

```bash
# Check volume status
docker volume ls
docker volume inspect deploy_memory-graph-data

# Verify permissions
docker compose exec memory-graph ls -la /data
```

### Common Errors

| Error | Solution |
|-------|----------|
| `port already allocated` | Change port in `.env` file |
| `permission denied` | Check file/directory permissions |
| `container unhealthy` | Check logs and health check configuration |
| `no space left` | Clean up Docker resources: `docker system prune` |

## Advanced Topics

### Custom Plugins

Mount custom plugins:

```bash
mkdir -p ./plugins
# Copy your plugin files
```

Plugins are automatically loaded from `/plugins` directory.

### External Services Integration

To integrate with LLM-Registry and Data-Vault:

1. Update `.env` with service URLs
2. Ensure services are on same network
3. Configure API keys

### Logging Configuration

Enable structured logging:

```yaml
environment:
  - RUST_LOG=memory_graph=debug,tower=info
```

### Performance Tuning

**Sled Database**:
```yaml
environment:
  - SLED_CACHE_CAPACITY=1073741824  # 1GB
  - SLED_MODE=HighThroughput
```

**gRPC Server**:
```yaml
environment:
  - GRPC_MAX_CONNECTIONS=10000
  - GRPC_KEEPALIVE_TIME_MS=60000
```

### CI/CD Integration

**Build Image**:
```bash
docker build -t llm-memory-graph:latest -f deploy/docker/Dockerfile .
```

**Push to Registry**:
```bash
docker tag llm-memory-graph:latest your-registry/llm-memory-graph:latest
docker push your-registry/llm-memory-graph:latest
```

### Kubernetes Migration

For Kubernetes deployment, see:
- `/deploy/kubernetes/` directory
- Helm charts in `/deploy/helm/`

## Maintenance

### Update Services

```bash
# Pull latest images
docker compose pull

# Restart services
docker compose up -d
```

### Clean Up

```bash
# Remove stopped containers
docker compose down

# Remove volumes (WARNING: deletes data)
docker compose down -v

# Clean up unused resources
docker system prune -a
```

### Health Checks

Regular health check script:

```bash
#!/bin/bash
# health-check.sh

services=("memory-graph" "prometheus" "grafana")

for service in "${services[@]}"; do
  if docker compose ps | grep -q "$service.*running"; then
    echo "✓ $service is running"
  else
    echo "✗ $service is not running"
    exit 1
  fi
done
```

## Support and Resources

### Documentation

- **Main README**: `/README.md`
- **Implementation Plan**: `/plans/PRODUCTION-PHASE-IMPLEMENTATION-PLAN.md`
- **Grafana Dashboards**: `/grafana/README.md`
- **API Documentation**: Generated from protobuf definitions

### Getting Help

- **GitHub Issues**: https://github.com/globalbusinessadvisors/llm-memory-graph/issues
- **Discussions**: https://github.com/globalbusinessadvisors/llm-memory-graph/discussions

### Contributing

Contributions are welcome! Please see the main repository README for guidelines.

## License

MIT OR Apache-2.0

---

**Last Updated**: 2025-11-07
**Version**: 1.0.0
**Maintained by**: LLM DevOps Team
