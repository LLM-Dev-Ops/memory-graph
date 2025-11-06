# LLM-Memory-Graph Deployment Guide

## Table of Contents
1. [Prerequisites](#prerequisites)
2. [Deployment Modes](#deployment-modes)
3. [Configuration](#configuration)
4. [Installation Steps](#installation-steps)
5. [Operational Procedures](#operational-procedures)
6. [Monitoring & Maintenance](#monitoring--maintenance)
7. [Troubleshooting](#troubleshooting)

---

## 1. PREREQUISITES

### 1.1 Infrastructure Requirements

#### Minimum Requirements (Development)
```yaml
compute:
  nodes: 1
  cpu_per_node: 8 cores
  memory_per_node: 32 GB
  storage: 500 GB SSD

networking:
  bandwidth: 1 Gbps
  latency: < 10ms inter-node
```

#### Production Requirements
```yaml
compute:
  nodes: 12+ (across availability zones)
  breakdown:
    api_servers: 5 nodes (8 cores, 32 GB each)
    graph_db: 3 nodes (16 cores, 128 GB each)
    vector_store: managed service or 3 nodes
    cache: 3 nodes (8 cores, 64 GB each)
    analytics: 3 nodes (16 cores, 64 GB each)

storage:
  graph_db: 2 TB NVMe SSD (per node)
  cache: 200 GB RAM (distributed)
  time_series: 5 TB SSD
  archive: Unlimited (S3/GCS)

networking:
  bandwidth: 10 Gbps
  latency: < 1ms inter-node (same AZ)
  load_balancer: Layer 7 with SSL termination
```

### 1.2 Software Requirements

```yaml
kubernetes:
  version: "1.28+"
  cluster_type: "managed (EKS/GKE/AKS) or self-hosted"

container_runtime:
  - docker: "24.0+"
  - containerd: "1.7+"

databases:
  neo4j: "5.12+"
  redis: "7.2+"
  influxdb: "2.7+"
  elasticsearch: "8.10+"

messaging:
  kafka: "3.5+"
  schema_registry: "7.5+"

observability:
  prometheus: "2.47+"
  grafana: "10.1+"
  jaeger: "1.50+"
  elasticsearch: "8.10+" (for logs)

service_mesh:
  istio: "1.19+" (optional but recommended)
```

### 1.3 Cloud Provider Credentials

```bash
# AWS
export AWS_ACCESS_KEY_ID="..."
export AWS_SECRET_ACCESS_KEY="..."
export AWS_REGION="us-west-2"

# GCP
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
export GCP_PROJECT_ID="..."
export GCP_REGION="us-central1"

# Azure
export AZURE_SUBSCRIPTION_ID="..."
export AZURE_TENANT_ID="..."
export AZURE_CLIENT_ID="..."
export AZURE_CLIENT_SECRET="..."
```

---

## 2. DEPLOYMENT MODES

### 2.1 Mode Comparison

```
┌─────────────────┬──────────────┬──────────────┬──────────────┬──────────────┐
│    Feature      │  Embedded    │  Standalone  │    Plugin    │    Hybrid    │
├─────────────────┼──────────────┼──────────────┼──────────────┼──────────────┤
│ Complexity      │   Low        │    High      │    Medium    │     High     │
│ Scalability     │   Limited    │    High      │    Medium    │     High     │
│ Latency         │   < 5ms      │   < 50ms     │   < 100ms    │   Variable   │
│ Multi-tenancy   │   No         │    Yes       │    Limited   │     Yes      │
│ Ops Overhead    │   Minimal    │    High      │    Low       │     High     │
│ Use Case        │   Dev/Test   │  Production  │  Integration │  Edge+Cloud  │
└─────────────────┴──────────────┴──────────────┴──────────────┴──────────────┘
```

### 2.2 Embedded Mode Deployment

**Installation:**
```bash
# NPM
npm install @llm-devops/memory-graph-embedded

# Python
pip install llm-memory-graph-embedded

# Go
go get github.com/llm-devops/memory-graph-embedded
```

**Usage Example (Node.js):**
```javascript
import MemoryGraph from '@llm-devops/memory-graph-embedded';

const graph = new MemoryGraph({
  storage: {
    type: 'sqlite',
    path: './data/graph.db'
  },
  vector: {
    type: 'faiss',
    dimension: 1536,
    index: 'IVF1024,Flat'
  },
  cache: {
    type: 'lru',
    maxSize: 10000
  }
});

// Initialize
await graph.init();

// Record prompt
const promptId = await graph.recordPrompt({
  text: "Explain quantum computing",
  sessionId: "sess_123",
  userId: "user_456",
  modelId: "gpt-4"
});

// Query
const lineage = await graph.getLineage(promptId, {
  direction: 'downstream',
  depth: 3
});

console.log(lineage);
```

### 2.3 Standalone Service Deployment (Kubernetes)

**Directory Structure:**
```
llm-memory-graph/
├── helm/
│   ├── Chart.yaml
│   ├── values.yaml
│   ├── values-prod.yaml
│   ├── templates/
│   │   ├── deployment-ingestion.yaml
│   │   ├── deployment-query.yaml
│   │   ├── deployment-analytics.yaml
│   │   ├── service.yaml
│   │   ├── ingress.yaml
│   │   ├── configmap.yaml
│   │   ├── secret.yaml
│   │   ├── hpa.yaml
│   │   └── pdb.yaml
│   └── charts/
│       ├── neo4j/
│       ├── redis/
│       ├── kafka/
│       └── elasticsearch/
└── kustomize/
    ├── base/
    └── overlays/
        ├── dev/
        ├── staging/
        └── production/
```

**Helm Chart Values (values-prod.yaml):**
```yaml
# Global settings
global:
  environment: production
  region: us-west-2
  domain: memory-graph.example.com

# Ingestion Service
ingestion:
  replicaCount: 3
  image:
    repository: llm-devops/memory-graph-ingestion
    tag: "1.0.0"
    pullPolicy: IfNotPresent

  resources:
    requests:
      cpu: "2"
      memory: "4Gi"
    limits:
      cpu: "4"
      memory: "8Gi"

  autoscaling:
    enabled: true
    minReplicas: 3
    maxReplicas: 20
    targetCPUUtilizationPercentage: 70
    targetMemoryUtilizationPercentage: 80

  env:
    - name: KAFKA_BROKERS
      value: "kafka-1:9092,kafka-2:9092,kafka-3:9092"
    - name: KAFKA_CONSUMER_GROUP
      value: "memory-graph-ingestion"
    - name: NEO4J_URI
      value: "bolt://neo4j:7687"
    - name: REDIS_URL
      value: "redis://redis-cluster:6379"
    - name: LOG_LEVEL
      value: "info"

# Query Service
query:
  replicaCount: 5
  image:
    repository: llm-devops/memory-graph-query
    tag: "1.0.0"

  resources:
    requests:
      cpu: "1"
      memory: "2Gi"
    limits:
      cpu: "2"
      memory: "4Gi"

  autoscaling:
    enabled: true
    minReplicas: 5
    maxReplicas: 50
    targetCPUUtilizationPercentage: 70
    customMetrics:
      - type: Pods
        pods:
          metric:
            name: http_request_duration_p99
          target:
            type: AverageValue
            averageValue: "200m"

  service:
    type: ClusterIP
    port: 8080
    annotations:
      prometheus.io/scrape: "true"
      prometheus.io/port: "9090"

# Neo4j
neo4j:
  enabled: true
  core:
    numberOfServers: 3
  readReplica:
    numberOfServers: 2

  resources:
    requests:
      cpu: "8"
      memory: "64Gi"
    limits:
      cpu: "16"
      memory: "128Gi"

  persistentVolume:
    size: 2Ti
    storageClass: "fast-ssd"

  config:
    dbms.memory.heap.initial_size: "32g"
    dbms.memory.heap.max_size: "32g"
    dbms.memory.pagecache.size: "64g"

# Redis
redis:
  enabled: true
  cluster:
    enabled: true
    slaveCount: 3

  master:
    persistence:
      size: 100Gi

  resources:
    requests:
      cpu: "4"
      memory: "32Gi"
    limits:
      cpu: "8"
      memory: "64Gi"

# InfluxDB
influxdb:
  enabled: true
  replicaCount: 3

  persistence:
    size: 5Ti

  resources:
    requests:
      cpu: "4"
      memory: "16Gi"
    limits:
      cpu: "8"
      memory: "32Gi"

# Elasticsearch
elasticsearch:
  enabled: true
  replicas: 3
  minimumMasterNodes: 2

  volumeClaimTemplate:
    resources:
      requests:
        storage: 1Ti

  resources:
    requests:
      cpu: "4"
      memory: "16Gi"
    limits:
      cpu: "8"
      memory: "32Gi"

# Ingress
ingress:
  enabled: true
  className: nginx
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"

  hosts:
    - host: api.memory-graph.example.com
      paths:
        - path: /
          pathType: Prefix
          backend:
            service:
              name: query-service
              port: 8080

  tls:
    - secretName: memory-graph-tls
      hosts:
        - api.memory-graph.example.com

# Monitoring
monitoring:
  prometheus:
    enabled: true
    retention: 30d

  grafana:
    enabled: true
    adminPassword: "changeme"

  jaeger:
    enabled: true
```

**Deployment Commands:**
```bash
# Add Helm repository
helm repo add llm-devops https://charts.llm-devops.io
helm repo update

# Install with custom values
helm install memory-graph llm-devops/memory-graph \
  --namespace llm-memory-graph \
  --create-namespace \
  --values values-prod.yaml \
  --wait \
  --timeout 15m

# Verify deployment
kubectl get pods -n llm-memory-graph
kubectl get svc -n llm-memory-graph

# Check ingestion service logs
kubectl logs -n llm-memory-graph -l app=ingestion -f

# Check query service logs
kubectl logs -n llm-memory-graph -l app=query -f
```

### 2.4 Plugin Mode Deployment

**LangChain Integration:**
```python
from langchain.callbacks import MemoryGraphCallback
from langchain.llms import OpenAI

# Initialize callback
callback = MemoryGraphCallback(
    endpoint="https://api.memory-graph.example.com",
    api_key="sk-...",
    sampling_rate=0.1  # Record 10% of requests
)

# Use with LangChain
llm = OpenAI(temperature=0.7, callbacks=[callback])

# All interactions are automatically tracked
response = llm.predict("Explain quantum computing")
```

**OpenAI SDK Middleware:**
```javascript
import OpenAI from 'openai';
import { memoryGraphMiddleware } from '@llm-devops/memory-graph-plugin';

const openai = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY
});

// Wrap with middleware
const trackedOpenAI = memoryGraphMiddleware(openai, {
  endpoint: 'https://api.memory-graph.example.com',
  apiKey: process.env.MEMORY_GRAPH_API_KEY,
  sessionId: 'my-session-123'
});

// Use as normal
const completion = await trackedOpenAI.chat.completions.create({
  model: 'gpt-4',
  messages: [{ role: 'user', content: 'Hello!' }]
});

// Interaction is automatically recorded
```

### 2.5 Hybrid Mode Deployment

**Edge Agent Configuration:**
```yaml
# edge-agent-config.yaml
edge:
  mode: embedded
  storage:
    type: sqlite
    path: /data/edge-graph.db
    max_size: 10GB

  sync:
    enabled: true
    central_endpoint: "https://central.memory-graph.example.com"
    interval: 5m
    batch_size: 1000
    retry_attempts: 3

  cache:
    enabled: true
    max_entries: 100000
    ttl: 24h

central:
  endpoint: "https://central.memory-graph.example.com"
  api_key: "${MEMORY_GRAPH_API_KEY}"
  region: "us-west-2"
```

**Edge Agent Deployment (Docker Compose):**
```yaml
version: '3.8'

services:
  edge-agent:
    image: llm-devops/memory-graph-edge:1.0.0
    container_name: memory-graph-edge
    volumes:
      - ./data:/data
      - ./config:/config
    environment:
      - CONFIG_PATH=/config/edge-agent-config.yaml
      - MEMORY_GRAPH_API_KEY=${MEMORY_GRAPH_API_KEY}
    ports:
      - "8080:8080"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

---

## 3. CONFIGURATION

### 3.1 Environment Variables

```bash
# Core Configuration
export MEMORY_GRAPH_ENV=production
export MEMORY_GRAPH_REGION=us-west-2
export LOG_LEVEL=info
export LOG_FORMAT=json

# Database Configuration
export NEO4J_URI=bolt://neo4j:7687
export NEO4J_USER=neo4j
export NEO4J_PASSWORD=${NEO4J_PASSWORD}
export NEO4J_DATABASE=graph
export NEO4J_MAX_CONNECTION_POOL_SIZE=50

export REDIS_URL=redis://redis-cluster:6379
export REDIS_PASSWORD=${REDIS_PASSWORD}
export REDIS_DB=0
export REDIS_MAX_CONNECTIONS=100

export INFLUXDB_URL=https://influxdb:8086
export INFLUXDB_TOKEN=${INFLUXDB_TOKEN}
export INFLUXDB_ORG=llm-devops
export INFLUXDB_BUCKET=memory-graph-metrics

export ELASTICSEARCH_URL=https://elasticsearch:9200
export ELASTICSEARCH_USERNAME=elastic
export ELASTICSEARCH_PASSWORD=${ELASTICSEARCH_PASSWORD}

# Kafka Configuration
export KAFKA_BROKERS=kafka-1:9092,kafka-2:9092,kafka-3:9092
export KAFKA_CONSUMER_GROUP=memory-graph-ingestion
export KAFKA_SASL_MECHANISM=SCRAM-SHA-512
export KAFKA_SASL_USERNAME=memory-graph
export KAFKA_SASL_PASSWORD=${KAFKA_SASL_PASSWORD}

# Vector Store Configuration
export PINECONE_API_KEY=${PINECONE_API_KEY}
export PINECONE_ENVIRONMENT=us-west1-gcp
export PINECONE_INDEX=llm-memory-prompts

# External Services
export LLM_REGISTRY_ENDPOINT=grpc://llm-registry:50051
export LLM_DATA_VAULT_ENDPOINT=https://data-vault.example.com
export LLM_DATA_VAULT_API_KEY=${DATA_VAULT_API_KEY}

# Security
export JWT_SECRET=${JWT_SECRET}
export ENCRYPTION_KEY=${ENCRYPTION_KEY}
export TLS_CERT_PATH=/certs/tls.crt
export TLS_KEY_PATH=/certs/tls.key

# Performance Tuning
export MAX_REQUEST_SIZE=10MB
export REQUEST_TIMEOUT=30s
export WORKER_THREADS=8
export QUEUE_SIZE=10000

# Feature Flags
export ENABLE_PII_DETECTION=true
export ENABLE_EMBEDDING_GENERATION=true
export ENABLE_SIMILARITY_SEARCH=true
export ENABLE_REAL_TIME_ANALYTICS=true
```

### 3.2 Configuration File (config.yaml)

```yaml
# config.yaml
server:
  host: 0.0.0.0
  port: 8080
  readTimeout: 30s
  writeTimeout: 30s
  idleTimeout: 120s
  maxHeaderBytes: 1048576

database:
  neo4j:
    uri: ${NEO4J_URI}
    user: ${NEO4J_USER}
    password: ${NEO4J_PASSWORD}
    database: graph
    maxConnectionPoolSize: 50
    connectionAcquisitionTimeout: 60s
    maxTransactionRetryTime: 30s
    logging: info

  redis:
    url: ${REDIS_URL}
    password: ${REDIS_PASSWORD}
    db: 0
    maxConnections: 100
    minIdleConnections: 10
    dialTimeout: 5s
    readTimeout: 3s
    writeTimeout: 3s
    poolTimeout: 4s

  influxdb:
    url: ${INFLUXDB_URL}
    token: ${INFLUXDB_TOKEN}
    org: llm-devops
    bucket: memory-graph-metrics
    batchSize: 5000
    flushInterval: 1s

  elasticsearch:
    url: ${ELASTICSEARCH_URL}
    username: ${ELASTICSEARCH_USERNAME}
    password: ${ELASTICSEARCH_PASSWORD}
    sniff: true
    healthcheck: true
    retryOnStatus: [502, 503, 504]
    maxRetries: 3

kafka:
  brokers: ${KAFKA_BROKERS}
  consumerGroup: ${KAFKA_CONSUMER_GROUP}
  topics:
    prompts: llm.prompts.v1
    responses: llm.responses.v1
    sessions: llm.sessions.v1

  consumer:
    autoOffsetReset: earliest
    enableAutoCommit: false
    maxPollRecords: 500
    maxPollIntervalMs: 300000
    sessionTimeoutMs: 30000

  producer:
    compressionType: snappy
    acks: all
    retries: 3
    maxInFlightRequestsPerConnection: 5
    enableIdempotence: true

vectorStore:
  provider: pinecone
  apiKey: ${PINECONE_API_KEY}
  environment: ${PINECONE_ENVIRONMENT}
  index: llm-memory-prompts
  dimension: 1536
  metric: cosine
  namespace: default

enrichment:
  piiDetection:
    enabled: true
    engines:
      - regex
      - ner
    redact: false
    encrypt: true

  embedding:
    enabled: true
    provider: openai
    model: text-embedding-ada-002
    batchSize: 100
    timeout: 30s
    cache:
      enabled: true
      ttl: 24h

  metadata:
    enabled: true
    registryEndpoint: ${LLM_REGISTRY_ENDPOINT}
    cacheTTL: 5m

cache:
  l1:
    enabled: true
    maxSize: 10000
    ttl: 1m

  l2:
    enabled: true
    ttl: 15m
    adaptiveTTL: true

rateLimit:
  enabled: true
  global:
    rps: 10000
    burst: 20000

  perUser:
    rps: 100
    burst: 200

  perTenant:
    rps: 1000
    burst: 2000

security:
  tls:
    enabled: true
    certFile: ${TLS_CERT_PATH}
    keyFile: ${TLS_KEY_PATH}

  authentication:
    type: jwt
    secret: ${JWT_SECRET}
    expiryTime: 15m
    refreshExpiryTime: 7d

  authorization:
    type: rbac
    roles:
      - admin
      - analyst
      - user

  audit:
    enabled: true
    logAllAccess: true
    retentionDays: 2555  # 7 years

observability:
  metrics:
    enabled: true
    port: 9090
    path: /metrics

  tracing:
    enabled: true
    provider: jaeger
    endpoint: http://jaeger:14268/api/traces
    samplingRate: 0.1

  logging:
    level: ${LOG_LEVEL}
    format: ${LOG_FORMAT}
    output: stdout
```

---

## 4. INSTALLATION STEPS

### 4.1 Pre-Installation Checklist

```bash
#!/bin/bash
# pre-install-check.sh

echo "Checking prerequisites..."

# Check Kubernetes
if ! command -v kubectl &> /dev/null; then
    echo "ERROR: kubectl not found"
    exit 1
fi

KUBE_VERSION=$(kubectl version --client --short | awk '{print $3}')
echo "✓ kubectl version: $KUBE_VERSION"

# Check Helm
if ! command -v helm &> /dev/null; then
    echo "ERROR: helm not found"
    exit 1
fi

HELM_VERSION=$(helm version --short)
echo "✓ helm version: $HELM_VERSION"

# Check cluster connectivity
if ! kubectl cluster-info &> /dev/null; then
    echo "ERROR: Cannot connect to Kubernetes cluster"
    exit 1
fi
echo "✓ Kubernetes cluster accessible"

# Check cluster capacity
NODES=$(kubectl get nodes --no-headers | wc -l)
TOTAL_CPU=$(kubectl get nodes -o json | jq '[.items[].status.capacity.cpu | tonumber] | add')
TOTAL_MEM=$(kubectl get nodes -o json | jq '[.items[].status.capacity.memory | gsub("Ki";"") | tonumber] | add / 1048576')

echo "✓ Cluster capacity: $NODES nodes, ${TOTAL_CPU} CPUs, ${TOTAL_MEM}Gi memory"

if [ "$TOTAL_CPU" -lt 64 ]; then
    echo "WARNING: Cluster has less than 64 CPUs (recommended minimum)"
fi

if [ $(echo "$TOTAL_MEM < 256" | bc) -eq 1 ]; then
    echo "WARNING: Cluster has less than 256Gi memory (recommended minimum)"
fi

# Check storage classes
STORAGE_CLASSES=$(kubectl get storageclass --no-headers | wc -l)
if [ "$STORAGE_CLASSES" -eq 0 ]; then
    echo "ERROR: No storage classes found"
    exit 1
fi
echo "✓ Storage classes available: $STORAGE_CLASSES"

# Check required secrets
if kubectl get secret llm-memory-graph-secrets -n llm-memory-graph &> /dev/null; then
    echo "✓ Secrets configured"
else
    echo "WARNING: Secrets not found. Run create-secrets.sh"
fi

echo ""
echo "Pre-installation check complete!"
```

### 4.2 Create Secrets

```bash
#!/bin/bash
# create-secrets.sh

NAMESPACE=llm-memory-graph

# Create namespace
kubectl create namespace $NAMESPACE --dry-run=client -o yaml | kubectl apply -f -

# Generate random passwords if not set
export NEO4J_PASSWORD=${NEO4J_PASSWORD:-$(openssl rand -base64 32)}
export REDIS_PASSWORD=${REDIS_PASSWORD:-$(openssl rand -base64 32)}
export INFLUXDB_TOKEN=${INFLUXDB_TOKEN:-$(openssl rand -base64 32)}
export ELASTICSEARCH_PASSWORD=${ELASTICSEARCH_PASSWORD:-$(openssl rand -base64 32)}
export KAFKA_SASL_PASSWORD=${KAFKA_SASL_PASSWORD:-$(openssl rand -base64 32)}
export JWT_SECRET=${JWT_SECRET:-$(openssl rand -base64 64)}
export ENCRYPTION_KEY=${ENCRYPTION_KEY:-$(openssl rand -base64 32)}

# Create Kubernetes secret
kubectl create secret generic llm-memory-graph-secrets \
  --namespace=$NAMESPACE \
  --from-literal=neo4j-password=$NEO4J_PASSWORD \
  --from-literal=redis-password=$REDIS_PASSWORD \
  --from-literal=influxdb-token=$INFLUXDB_TOKEN \
  --from-literal=elasticsearch-password=$ELASTICSEARCH_PASSWORD \
  --from-literal=kafka-sasl-password=$KAFKA_SASL_PASSWORD \
  --from-literal=jwt-secret=$JWT_SECRET \
  --from-literal=encryption-key=$ENCRYPTION_KEY \
  --from-literal=pinecone-api-key=${PINECONE_API_KEY} \
  --from-literal=data-vault-api-key=${DATA_VAULT_API_KEY} \
  --dry-run=client -o yaml | kubectl apply -f -

echo "Secrets created successfully!"
echo ""
echo "IMPORTANT: Save these credentials securely:"
echo "NEO4J_PASSWORD=$NEO4J_PASSWORD"
echo "REDIS_PASSWORD=$REDIS_PASSWORD"
echo "INFLUXDB_TOKEN=$INFLUXDB_TOKEN"
echo "ELASTICSEARCH_PASSWORD=$ELASTICSEARCH_PASSWORD"
echo "JWT_SECRET=$JWT_SECRET"
```

### 4.3 Install Dependencies

```bash
#!/bin/bash
# install-dependencies.sh

NAMESPACE=llm-memory-graph

# Add Helm repositories
helm repo add bitnami https://charts.bitnami.com/bitnami
helm repo add neo4j https://neo4j.com/helm
helm repo add elastic https://helm.elastic.co
helm repo add prometheus-community https://prometheus-community.github.io/helm-charts
helm repo update

# Install Neo4j
helm install neo4j neo4j/neo4j \
  --namespace=$NAMESPACE \
  --set neo4j.password=$NEO4J_PASSWORD \
  --set core.numberOfServers=3 \
  --set readReplica.numberOfServers=2 \
  --set core.persistentVolume.size=2Ti \
  --wait

# Install Redis Cluster
helm install redis bitnami/redis-cluster \
  --namespace=$NAMESPACE \
  --set password=$REDIS_PASSWORD \
  --set cluster.nodes=6 \
  --set persistence.size=100Gi \
  --wait

# Install InfluxDB
helm install influxdb bitnami/influxdb \
  --namespace=$NAMESPACE \
  --set auth.admin.token=$INFLUXDB_TOKEN \
  --set persistence.size=5Ti \
  --wait

# Install Elasticsearch
helm install elasticsearch elastic/elasticsearch \
  --namespace=$NAMESPACE \
  --set replicas=3 \
  --set minimumMasterNodes=2 \
  --set volumeClaimTemplate.resources.requests.storage=1Ti \
  --wait

# Install Kafka (if not using managed service)
helm install kafka bitnami/kafka \
  --namespace=$NAMESPACE \
  --set replicaCount=3 \
  --set persistence.size=1Ti \
  --wait

# Install Prometheus Stack
helm install prometheus prometheus-community/kube-prometheus-stack \
  --namespace=$NAMESPACE \
  --set prometheus.prometheusSpec.retention=30d \
  --set prometheus.prometheusSpec.storageSpec.volumeClaimTemplate.spec.resources.requests.storage=500Gi \
  --wait

echo "Dependencies installed successfully!"
```

### 4.4 Install LLM-Memory-Graph

```bash
#!/bin/bash
# install-memory-graph.sh

NAMESPACE=llm-memory-graph
RELEASE_NAME=memory-graph
CHART_VERSION=1.0.0

# Install with production values
helm install $RELEASE_NAME llm-devops/memory-graph \
  --namespace=$NAMESPACE \
  --version=$CHART_VERSION \
  --values=values-prod.yaml \
  --wait \
  --timeout=15m

# Verify deployment
kubectl rollout status deployment/ingestion-service -n $NAMESPACE
kubectl rollout status deployment/query-service -n $NAMESPACE
kubectl rollout status deployment/analytics-service -n $NAMESPACE

# Get ingress URL
INGRESS_URL=$(kubectl get ingress -n $NAMESPACE -o jsonpath='{.items[0].spec.rules[0].host}')

echo ""
echo "Installation complete!"
echo "API URL: https://$INGRESS_URL"
echo ""
echo "Next steps:"
echo "1. Configure DNS to point to the ingress"
echo "2. Run health checks"
echo "3. Configure monitoring dashboards"
```

### 4.5 Post-Installation Verification

```bash
#!/bin/bash
# verify-installation.sh

NAMESPACE=llm-memory-graph
API_URL=${API_URL:-http://localhost:8080}

echo "Verifying installation..."

# Check all pods are running
echo "Checking pod status..."
kubectl get pods -n $NAMESPACE

PENDING_PODS=$(kubectl get pods -n $NAMESPACE --field-selector=status.phase!=Running --no-headers 2>/dev/null | wc -l)
if [ "$PENDING_PODS" -gt 0 ]; then
    echo "ERROR: $PENDING_PODS pods are not running"
    exit 1
fi
echo "✓ All pods running"

# Health check
echo "Checking API health..."
HEALTH_STATUS=$(curl -s -o /dev/null -w "%{http_code}" $API_URL/health)
if [ "$HEALTH_STATUS" -ne 200 ]; then
    echo "ERROR: Health check failed (HTTP $HEALTH_STATUS)"
    exit 1
fi
echo "✓ API health check passed"

# Test prompt ingestion
echo "Testing prompt ingestion..."
RESPONSE=$(curl -s -X POST $API_URL/api/v1/prompts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_TOKEN" \
  -d '{
    "text": "Hello, world!",
    "sessionId": "test-session",
    "userId": "test-user",
    "modelId": "gpt-4"
  }')

PROMPT_ID=$(echo $RESPONSE | jq -r '.id')
if [ "$PROMPT_ID" == "null" ]; then
    echo "ERROR: Failed to ingest prompt"
    exit 1
fi
echo "✓ Prompt ingestion successful (ID: $PROMPT_ID)"

# Test query
echo "Testing query..."
QUERY_RESPONSE=$(curl -s $API_URL/api/v1/prompts/$PROMPT_ID \
  -H "Authorization: Bearer $API_TOKEN")

if [ "$(echo $QUERY_RESPONSE | jq -r '.id')" != "$PROMPT_ID" ]; then
    echo "ERROR: Query failed"
    exit 1
fi
echo "✓ Query successful"

echo ""
echo "Installation verification complete!"
```

---

## 5. OPERATIONAL PROCEDURES

### 5.1 Backup and Restore

**Backup Script:**
```bash
#!/bin/bash
# backup.sh

NAMESPACE=llm-memory-graph
BACKUP_DIR=/backups/$(date +%Y-%m-%d)
mkdir -p $BACKUP_DIR

# Backup Neo4j
kubectl exec -n $NAMESPACE neo4j-0 -- neo4j-admin dump \
  --database=graph \
  --to=/tmp/graph-backup.dump

kubectl cp $NAMESPACE/neo4j-0:/tmp/graph-backup.dump \
  $BACKUP_DIR/neo4j-graph.dump

# Backup Redis
kubectl exec -n $NAMESPACE redis-0 -- redis-cli --rdb /tmp/dump.rdb
kubectl cp $NAMESPACE/redis-0:/tmp/dump.rdb \
  $BACKUP_DIR/redis-dump.rdb

# Backup InfluxDB
kubectl exec -n $NAMESPACE influxdb-0 -- influx backup /tmp/backup
kubectl cp $NAMESPACE/influxdb-0:/tmp/backup \
  $BACKUP_DIR/influxdb-backup

# Upload to S3
aws s3 sync $BACKUP_DIR s3://llm-memory-graph-backups/$(date +%Y-%m-%d)/

echo "Backup complete: $BACKUP_DIR"
```

**Restore Script:**
```bash
#!/bin/bash
# restore.sh

BACKUP_DATE=$1
if [ -z "$BACKUP_DATE" ]; then
    echo "Usage: $0 <backup-date>"
    exit 1
fi

NAMESPACE=llm-memory-graph
BACKUP_DIR=/backups/$BACKUP_DATE

# Download from S3
aws s3 sync s3://llm-memory-graph-backups/$BACKUP_DATE/ $BACKUP_DIR/

# Restore Neo4j
kubectl cp $BACKUP_DIR/neo4j-graph.dump \
  $NAMESPACE/neo4j-0:/tmp/graph-backup.dump

kubectl exec -n $NAMESPACE neo4j-0 -- neo4j-admin load \
  --from=/tmp/graph-backup.dump \
  --database=graph \
  --force

# Restore Redis
kubectl cp $BACKUP_DIR/redis-dump.rdb \
  $NAMESPACE/redis-0:/tmp/dump.rdb

kubectl exec -n $NAMESPACE redis-0 -- redis-cli \
  --rdb /tmp/dump.rdb

# Restart services
kubectl rollout restart deployment/ingestion-service -n $NAMESPACE
kubectl rollout restart deployment/query-service -n $NAMESPACE

echo "Restore complete from: $BACKUP_DIR"
```

### 5.2 Scaling

**Manual Scaling:**
```bash
# Scale ingestion service
kubectl scale deployment ingestion-service \
  --replicas=10 \
  -n llm-memory-graph

# Scale query service
kubectl scale deployment query-service \
  --replicas=20 \
  -n llm-memory-graph
```

**Auto-Scaling Configuration:**
```yaml
# hpa-custom.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: query-service-hpa
  namespace: llm-memory-graph
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: query-service
  minReplicas: 5
  maxReplicas: 50
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Pods
    pods:
      metric:
        name: http_request_duration_p99
      target:
        type: AverageValue
        averageValue: "200m"
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 50
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 60
      policies:
      - type: Percent
        value: 100
        periodSeconds: 30
```

### 5.3 Updates and Rollbacks

**Rolling Update:**
```bash
# Update to new version
helm upgrade memory-graph llm-devops/memory-graph \
  --namespace=llm-memory-graph \
  --version=1.1.0 \
  --values=values-prod.yaml \
  --wait

# Monitor rollout
kubectl rollout status deployment/query-service -n llm-memory-graph

# Check rollout history
kubectl rollout history deployment/query-service -n llm-memory-graph
```

**Rollback:**
```bash
# Rollback to previous version
helm rollback memory-graph -n llm-memory-graph

# Or rollback to specific revision
helm rollback memory-graph 3 -n llm-memory-graph
```

---

## 6. MONITORING & MAINTENANCE

### 6.1 Health Checks

```bash
# API health endpoint
curl https://api.memory-graph.example.com/health

# Expected response:
{
  "status": "healthy",
  "version": "1.0.0",
  "checks": {
    "neo4j": "healthy",
    "redis": "healthy",
    "kafka": "healthy",
    "influxdb": "healthy",
    "elasticsearch": "healthy"
  }
}
```

### 6.2 Key Metrics to Monitor

```yaml
alerts:
  - name: HighErrorRate
    expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.01
    severity: critical
    description: "Error rate > 1%"

  - name: HighLatency
    expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 0.5
    severity: warning
    description: "P99 latency > 500ms"

  - name: HighMemoryUsage
    expr: container_memory_usage_bytes / container_spec_memory_limit_bytes > 0.9
    severity: warning
    description: "Memory usage > 90%"

  - name: Neo4jDown
    expr: up{job="neo4j"} == 0
    severity: critical
    description: "Neo4j is down"
```

### 6.3 Log Aggregation

```bash
# View logs from all ingestion pods
kubectl logs -n llm-memory-graph -l app=ingestion --tail=100 -f

# View logs from specific pod
kubectl logs -n llm-memory-graph ingestion-service-abc123-xyz

# Search logs in Elasticsearch
curl -X POST "https://elasticsearch:9200/logs-*/_search" \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "bool": {
        "must": [
          {"match": {"level": "error"}},
          {"range": {"@timestamp": {"gte": "now-1h"}}}
        ]
      }
    },
    "sort": [{"@timestamp": "desc"}],
    "size": 100
  }'
```

---

## 7. TROUBLESHOOTING

### 7.1 Common Issues

**Issue: Pods stuck in Pending state**
```bash
# Check pod events
kubectl describe pod <pod-name> -n llm-memory-graph

# Common causes:
# 1. Insufficient resources
kubectl get nodes -o json | jq '.items[] | {name:.metadata.name, allocatable:.status.allocatable}'

# 2. PVC not bound
kubectl get pvc -n llm-memory-graph

# 3. Image pull errors
kubectl get events -n llm-memory-graph --sort-by='.lastTimestamp'
```

**Issue: High latency**
```bash
# Check cache hit rate
kubectl exec -n llm-memory-graph redis-0 -- redis-cli INFO stats | grep cache_hit_rate

# Check database query performance
kubectl exec -n llm-memory-graph neo4j-0 -- cypher-shell \
  "CALL dbms.listQueries() YIELD query, elapsedTimeMillis WHERE elapsedTimeMillis > 1000 RETURN *"

# Check resource utilization
kubectl top pods -n llm-memory-graph
```

**Issue: Data inconsistency**
```bash
# Run reconciliation job
kubectl create job --from=cronjob/reconciliation reconciliation-manual -n llm-memory-graph

# Check job status
kubectl get jobs -n llm-memory-graph
kubectl logs job/reconciliation-manual -n llm-memory-graph
```

### 7.2 Emergency Procedures

**Circuit Breaker Activation:**
```bash
# Manually activate circuit breaker
kubectl exec -n llm-memory-graph query-service-0 -- \
  curl -X POST http://localhost:8080/admin/circuit-breaker/activate

# Check status
kubectl exec -n llm-memory-graph query-service-0 -- \
  curl http://localhost:8080/admin/circuit-breaker/status
```

**Rate Limit Adjustment:**
```bash
# Temporarily increase rate limits
kubectl patch configmap app-config -n llm-memory-graph \
  --patch '{"data":{"RATE_LIMIT_RPS":"20000"}}'

# Restart pods to apply
kubectl rollout restart deployment/query-service -n llm-memory-graph
```

### 7.3 Support Contacts

```
Critical Issues (24/7):
  PagerDuty: https://llm-devops.pagerduty.com
  On-call phone: +1-555-0100

Non-Critical Issues:
  Email: support@llm-devops.io
  Slack: #llm-memory-graph-support

Documentation:
  https://docs.llm-devops.io/memory-graph
```

---

This deployment guide provides comprehensive instructions for deploying and operating LLM-Memory-Graph across different modes and environments.
