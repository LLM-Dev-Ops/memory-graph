# Kubernetes Deployment Manifest Index

Complete listing of all Kubernetes deployment files for LLM-Memory-Graph gRPC service.

## Quick Navigation

| File | Type | Purpose | Size |
|------|------|---------|------|
| [namespace.yaml](#namespaceyaml) | Resource | Dedicated namespace | 232 B |
| [configmap.yaml](#configmapyaml) | Resource | Environment configuration | 768 B |
| [secret.yaml](#secretyaml) | Resource | API keys & credentials | 1.6 KB |
| [pvc.yaml](#pvcyaml) | Resource | Persistent storage | 1.3 KB |
| [deployment.yaml](#deploymentyaml) | Resource | Main deployment + RBAC | 6.3 KB |
| [service.yaml](#serviceyaml) | Resource | Network services | 2.2 KB |
| [hpa.yaml](#hpayaml) | Resource | Auto-scaling | 4.0 KB |
| [servicemonitor.yaml](#servicemonitoryaml) | Resource | Monitoring & alerts | 6.7 KB |
| [kustomization.yaml](#kustomizationyaml) | Config | Kustomize | 1.9 KB |
| [deploy.sh](#deploysh) | Script | Automated deployment | 5.8 KB |
| [validate.sh](#validatesh) | Script | Health validation | 9.2 KB |
| [cleanup.sh](#cleanupsh) | Script | Safe cleanup | 3.0 KB |
| [README.md](#readmemd) | Docs | Deployment guide | 17 KB |
| [DEPLOYMENT-REPORT.md](#deployment-reportmd) | Docs | Detailed report | 26 KB |
| [QUICK-START.md](#quick-startmd) | Docs | Quick reference | 3.0 KB |
| [ARCHITECTURE.md](#architecturemd) | Docs | Architecture diagrams | 39 KB |

**Total**: 16 files, 164 KB

---

## Resource Manifests

### namespace.yaml
**Type**: Kubernetes Namespace
**Resources**: 1
- Namespace: `llm-memory-graph`

**Purpose**: Isolated environment for all LLM-Memory-Graph resources

**Labels**:
- `name: llm-memory-graph`
- `environment: production`
- `managed-by: kubernetes`

---

### configmap.yaml
**Type**: Kubernetes ConfigMap
**Resources**: 1
- ConfigMap: `memory-graph-config`

**Purpose**: Environment configuration for gRPC service

**Configuration Items** (12):
1. GRPC_HOST: "0.0.0.0"
2. GRPC_PORT: "50051"
3. METRICS_PORT: "9090"
4. RUST_LOG: "info,llm_memory_graph=debug"
5. DB_PATH: "/data"
6. MAX_CONNECTIONS: "1000"
7. REQUEST_TIMEOUT_MS: "30000"
8. ENABLE_REFLECTION: "true"
9. ENABLE_HEALTH: "true"
10. REGISTRY_URL: LLM-Registry endpoint
11. VAULT_URL: Data-Vault endpoint
12. PLUGIN_DIRS: "/plugins"

---

### secret.yaml
**Type**: Kubernetes Secret
**Resources**: 2
- Secret: `memory-graph-secrets`
- Examples: Sealed Secrets, External Secrets Operator

**Purpose**: Secure storage for sensitive credentials

**Secrets**:
1. REGISTRY_API_KEY (required - update before production!)
2. VAULT_API_KEY (required - update before production!)
3. TLS certificates (optional)

**Security Note**: Includes examples for production secret management

---

### pvc.yaml
**Type**: Kubernetes PersistentVolumeClaim
**Resources**: 3
- PVC: `memory-graph-data` (100Gi, RWO)
- PVC: `memory-graph-plugins` (10Gi, ROX)
- Notes/examples for RWX configurations

**Purpose**: Persistent storage for data and plugins

**Storage**:
1. Data volume: 100Gi (database, session data)
2. Plugin volume: 10Gi (extensions, plugins)

**Notes**: Includes examples for multi-replica shared storage

---

### deployment.yaml
**Type**: Kubernetes Deployment
**Resources**: 4
- Deployment: `memory-graph` (3 replicas)
- ServiceAccount: `memory-graph`
- Role: `memory-graph`
- RoleBinding: `memory-graph`

**Purpose**: Core application deployment with RBAC

**Specifications**:
- Replicas: 3 (with anti-affinity)
- Image: `ghcr.io/globalbusinessadvisors/llm-memory-graph:latest`
- Resources:
  - Memory: 2Gi request, 4Gi limit
  - CPU: 1 core request, 2 core limit
  - Ephemeral storage: 10Gi request, 20Gi limit
- Health Probes: Liveness, Readiness, Startup (gRPC)
- Security: Non-root, read-only filesystem, drop ALL caps
- Volumes: data PVC, plugins PVC, tmp emptyDir

**High Availability**:
- Rolling update strategy (maxSurge: 1, maxUnavailable: 0)
- Pod anti-affinity rules
- Topology spread constraints
- Multi-zone distribution

---

### service.yaml
**Type**: Kubernetes Service
**Resources**: 4
- Service: `memory-graph` (LoadBalancer)
- Service: `memory-graph-headless` (ClusterIP None)
- Service: `memory-graph-internal` (ClusterIP)
- Optional: NodePort example (commented)

**Purpose**: Network access to gRPC service

**Services**:
1. **LoadBalancer**: External access
   - Ports: 50051 (gRPC), 9090 (metrics)
   - Session affinity: ClientIP (3h timeout)
   - Cloud provider annotations (AWS/Azure/GCP)

2. **Headless**: Direct pod access
   - For StatefulSet scenarios
   - publishNotReadyAddresses: true

3. **Internal**: In-cluster communication
   - ClusterIP only
   - No external access

---

### hpa.yaml
**Type**: HorizontalPodAutoscaler
**Resources**: 3
- HPA: `memory-graph-hpa`
- Examples: Prometheus Adapter config
- Optional: VerticalPodAutoscaler example

**Purpose**: Automatic scaling based on metrics

**Configuration**:
- Min replicas: 3
- Max replicas: 10

**Metrics** (5):
1. CPU: 70% target
2. Memory: 80% target
3. Custom: grpc_requests_per_second (1000 target)
4. Custom: grpc_request_duration_p95 (100ms target)
5. Custom: grpc_active_streams (100 target)

**Scaling Behavior**:
- Scale-up: Fast (100% or 2 pods/30s, 60s stabilization)
- Scale-down: Slow (50% or 1 pod/60s, 300s stabilization)

---

### servicemonitor.yaml
**Type**: Prometheus ServiceMonitor & PrometheusRule
**Resources**: 3
- ServiceMonitor: `memory-graph`
- PrometheusRule: `memory-graph-alerts` (10 alerts)
- Optional: PodMonitor example

**Purpose**: Prometheus metrics collection and alerting

**Monitoring**:
- Scrape interval: 30s
- Scrape timeout: 10s
- Target: /metrics:9090

**Alerts** (10):
1. MemoryGraphHighErrorRate (Warning)
2. MemoryGraphHighLatency (Warning)
3. MemoryGraphServiceDown (Critical)
4. MemoryGraphHighMemoryUsage (Warning)
5. MemoryGraphHighCPUUsage (Warning)
6. MemoryGraphPodRestarting (Warning)
7. MemoryGraphHighStorageUsage (Warning)
8. MemoryGraphPluginErrors (Warning)
9. MemoryGraphIntegrationFailure (Warning)
10. MemoryGraphLowReplicaCount (Critical)

---

### kustomization.yaml
**Type**: Kustomize Configuration
**Resources**: 1 (references 9 resource files)

**Purpose**: Environment-specific customization

**Features**:
- Common labels and annotations
- Resource inclusion
- Image tag overrides
- ConfigMap/Secret generators (examples)
- Patch support

**Usage**:
```bash
kubectl apply -k deploy/kubernetes/
```

---

## Operational Scripts

### deploy.sh
**Type**: Bash script (executable)
**Lines**: ~180

**Purpose**: Automated deployment with validation

**Features**:
- Checks prerequisites
- Creates resources in order
- Waits for PVCs to bind
- Waits for deployment rollout
- Displays status and endpoints
- Provides helpful commands

**Usage**:
```bash
./deploy.sh [namespace]
```

**Default namespace**: llm-memory-graph

---

### validate.sh
**Type**: Bash script (executable)
**Lines**: ~280

**Purpose**: Comprehensive deployment validation

**Checks**:
1. Prerequisites (kubectl, optional tools)
2. Cluster connection
3. Namespace existence
4. ConfigMap and Secret
5. Secret validation (warns on defaults)
6. PVC status and binding
7. Deployment status
8. Pod status and readiness
9. Service endpoints
10. LoadBalancer provisioning
11. HPA status and metrics server
12. ServiceMonitor (if Prometheus Operator installed)
13. Optional: gRPC connectivity
14. Optional: Metrics endpoint
15. Recent events

**Usage**:
```bash
./validate.sh [namespace]
```

**Exit codes**:
- 0: Healthy
- 1: Issues detected

---

### cleanup.sh
**Type**: Bash script (executable)
**Lines**: ~90

**Purpose**: Safe cleanup with confirmations

**Features**:
- Confirmation prompts
- Reverse-order deletion
- Optional PVC preservation
- Optional namespace preservation
- Status display

**Usage**:
```bash
./cleanup.sh [namespace]
```

**Safety**: Requires "yes" confirmation

---

## Documentation

### README.md
**Size**: 17 KB
**Lines**: ~650

**Sections**:
1. Overview and architecture
2. Prerequisites
3. Quick start (3 methods)
4. Verification steps
5. Configuration management
6. Monitoring setup
7. Troubleshooting guide
8. Maintenance procedures
9. Performance tuning
10. Security hardening
11. Multi-environment deployment
12. Cost optimization
13. References

**Audience**: Operations teams, DevOps engineers

---

### DEPLOYMENT-REPORT.md
**Size**: 26 KB
**Lines**: ~850

**Sections**:
1. Executive summary
2. Architecture overview
3. Detailed component specifications
4. Security hardening details
5. Performance optimization
6. Monitoring configuration
7. Operational procedures
8. Multi-environment strategy
9. Cost optimization
10. Testing checklist
11. Production readiness
12. Performance targets (SLOs)
13. References
14. Appendices

**Audience**: Technical leads, architects, management

---

### QUICK-START.md
**Size**: 3.0 KB
**Lines**: ~120

**Sections**:
1. Prerequisites check
2. 5-minute deployment (3 options)
3. Quick verification
4. Service access
5. Common tasks
6. Cleanup
7. Pre-production checklist

**Audience**: Quick deployment, developers

---

### ARCHITECTURE.md
**Size**: 39 KB
**Lines**: ~500

**Sections**:
1. System overview diagram
2. Pod architecture
3. Traffic flow
4. Scaling behavior
5. Resource distribution
6. High availability strategy
7. Security layers
8. Deployment workflow
9. Monitoring architecture

**Audience**: Architects, technical teams

---

## Deployment Methods

### Method 1: Automated Script (Recommended)
```bash
cd deploy/kubernetes
./deploy.sh llm-memory-graph
```

**Pros**: Automated, validated, ordered, status feedback

### Method 2: Manual kubectl
```bash
cd deploy/kubernetes
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f pvc.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f hpa.yaml
kubectl apply -f servicemonitor.yaml
```

**Pros**: Full control, step-by-step

### Method 3: Kustomize
```bash
kubectl apply -k deploy/kubernetes/
```

**Pros**: Environment-specific, GitOps-friendly

---

## Resource Summary

**Kubernetes Resources Created**: 22
- 1 Namespace
- 1 ConfigMap
- 1 Secret
- 2 PersistentVolumeClaims
- 1 Deployment
- 3 Services
- 1 HorizontalPodAutoscaler
- 1 ServiceMonitor
- 1 PrometheusRule
- 1 ServiceAccount
- 1 Role
- 1 RoleBinding
- Examples and templates

**Scripts**: 3
- deploy.sh (automated deployment)
- validate.sh (health checks)
- cleanup.sh (safe removal)

**Documentation**: 4
- README.md (comprehensive guide)
- DEPLOYMENT-REPORT.md (detailed report)
- QUICK-START.md (quick reference)
- ARCHITECTURE.md (architecture diagrams)

**Total Files**: 16
**Total Size**: 164 KB
**Total Lines**: ~3,500+

---

## Prerequisites

### Required
- Kubernetes cluster v1.24+
- kubectl configured
- Storage class available

### Optional
- Prometheus Operator (for ServiceMonitor)
- Metrics Server (for HPA)
- Prometheus Adapter (for custom metrics HPA)
- grpcurl (for testing)

---

## Quick Commands

### Deploy
```bash
./deploy.sh llm-memory-graph
```

### Validate
```bash
./validate.sh llm-memory-graph
```

### View Logs
```bash
kubectl logs -f -l app=memory-graph -n llm-memory-graph
```

### Check Status
```bash
kubectl get all -n llm-memory-graph
```

### Cleanup
```bash
./cleanup.sh llm-memory-graph
```

---

## Support

- **Documentation**: See README.md
- **Implementation Plan**: /plans/PRODUCTION-PHASE-IMPLEMENTATION-PLAN.md
- **Repository**: https://github.com/globalbusinessadvisors/llm-memory-graph

---

**Index Version**: 1.0.0
**Last Updated**: 2025-11-07
**Maintained By**: LLM DevOps Team
