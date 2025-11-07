# LLM-Memory-Graph Kubernetes Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         KUBERNETES CLUSTER                                   │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │ INGRESS / LOAD BALANCER                                                │ │
│  │                                                                         │ │
│  │  External Traffic                                                       │ │
│  │       │                                                                 │ │
│  │       ▼                                                                 │ │
│  │  ┌─────────────────────┐                                               │ │
│  │  │ LoadBalancer Service│                                               │ │
│  │  │  :50051 (gRPC)      │                                               │ │
│  │  │  :9090 (Metrics)    │                                               │ │
│  │  └──────────┬──────────┘                                               │ │
│  └─────────────┼──────────────────────────────────────────────────────────┘ │
│                │                                                             │
│  ┌─────────────▼──────────────────────────────────────────────────────────┐ │
│  │ NAMESPACE: llm-memory-graph                                            │ │
│  │                                                                         │ │
│  │  ┌────────────────────────────────────────────────────────────────┐   │ │
│  │  │ APPLICATION TIER                                                │   │ │
│  │  │                                                                 │   │ │
│  │  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │   │ │
│  │  │  │ Pod-1        │  │ Pod-2        │  │ Pod-3        │         │   │ │
│  │  │  │ (AZ-A)       │  │ (AZ-B)       │  │ (AZ-C)       │         │   │ │
│  │  │  ├──────────────┤  ├──────────────┤  ├──────────────┤         │   │ │
│  │  │  │ Container:   │  │ Container:   │  │ Container:   │         │   │ │
│  │  │  │ memory-graph │  │ memory-graph │  │ memory-graph │         │   │ │
│  │  │  │              │  │              │  │              │         │   │ │
│  │  │  │ gRPC :50051  │  │ gRPC :50051  │  │ gRPC :50051  │         │   │ │
│  │  │  │ Metrics:9090 │  │ Metrics:9090 │  │ Metrics:9090 │         │   │ │
│  │  │  │              │  │              │  │              │         │   │ │
│  │  │  │ Resources:   │  │ Resources:   │  │ Resources:   │         │   │ │
│  │  │  │ 2-4Gi RAM    │  │ 2-4Gi RAM    │  │ 2-4Gi RAM    │         │   │ │
│  │  │  │ 1-2 CPU      │  │ 1-2 CPU      │  │ 1-2 CPU      │         │   │ │
│  │  │  │              │  │              │  │              │         │   │ │
│  │  │  │ Probes:      │  │ Probes:      │  │ Probes:      │         │   │ │
│  │  │  │ ✓ Liveness   │  │ ✓ Liveness   │  │ ✓ Liveness   │         │   │ │
│  │  │  │ ✓ Readiness  │  │ ✓ Readiness  │  │ ✓ Readiness  │         │   │ │
│  │  │  │ ✓ Startup    │  │ ✓ Startup    │  │ ✓ Startup    │         │   │ │
│  │  │  │              │  │              │  │              │         │   │ │
│  │  │  │ Security:    │  │ Security:    │  │ Security:    │         │   │ │
│  │  │  │ • Non-root   │  │ • Non-root   │  │ • Non-root   │         │   │ │
│  │  │  │ • Read-only  │  │ • Read-only  │  │ • Read-only  │         │   │ │
│  │  │  │ • Drop caps  │  │ • Drop caps  │  │ • Drop caps  │         │   │ │
│  │  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘         │   │ │
│  │  │         │                 │                 │                 │   │ │
│  │  │         └─────────────────┴─────────────────┘                 │   │ │
│  │  │                           │                                   │   │ │
│  │  └───────────────────────────┼───────────────────────────────────┘   │ │
│  │                              │                                       │ │
│  │  ┌───────────────────────────▼───────────────────────────────────┐   │ │
│  │  │ AUTO-SCALING TIER                                             │   │ │
│  │  │                                                                │   │ │
│  │  │  ┌──────────────────────────────────────────────────────────┐ │   │ │
│  │  │  │ HorizontalPodAutoscaler                                  │ │   │ │
│  │  │  │                                                           │ │   │ │
│  │  │  │ Min: 3 replicas  ──────────────────  Max: 10 replicas    │ │   │ │
│  │  │  │                                                           │ │   │ │
│  │  │  │ Scale-up triggers:                                        │ │   │ │
│  │  │  │ • CPU > 70%                                              │ │   │ │
│  │  │  │ • Memory > 80%                                           │ │   │ │
│  │  │  │ • gRPC requests/sec > 1000                               │ │   │ │
│  │  │  │ • gRPC p95 latency > 100ms                               │ │   │ │
│  │  │  │ • Active streams > 100                                   │ │   │ │
│  │  │  │                                                           │ │   │ │
│  │  │  │ Scale behavior:                                           │ │   │ │
│  │  │  │ Up:   Fast (100% or 2 pods/30s)                          │ │   │ │
│  │  │  │ Down: Slow (50% or 1 pod/60s, 5min stabilization)        │ │   │ │
│  │  │  └──────────────────────────────────────────────────────────┘ │   │ │
│  │  └────────────────────────────────────────────────────────────────┘   │ │
│  │                                                                       │ │
│  │  ┌──────────────────────────────────────────────────────────────┐   │ │
│  │  │ STORAGE TIER                                                 │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────┐      ┌────────────────────┐          │   │ │
│  │  │  │ PVC: data          │      │ PVC: plugins       │          │   │ │
│  │  │  │                    │      │                    │          │   │ │
│  │  │  │ Size: 100Gi        │      │ Size: 10Gi         │          │   │ │
│  │  │  │ Mode: RWO          │      │ Mode: ROX          │          │   │ │
│  │  │  │ Class: standard    │      │ Class: standard    │          │   │ │
│  │  │  │                    │      │                    │          │   │ │
│  │  │  │ Purpose:           │      │ Purpose:           │          │   │ │
│  │  │  │ • Database         │      │ • Plugin storage   │          │   │ │
│  │  │  │ • Session data     │      │ • Extensions       │          │   │ │
│  │  │  └────────────────────┘      └────────────────────┘          │   │ │
│  │  └──────────────────────────────────────────────────────────────┘   │ │
│  │                                                                       │ │
│  │  ┌──────────────────────────────────────────────────────────────┐   │ │
│  │  │ CONFIGURATION TIER                                           │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────┐      ┌────────────────────┐          │   │ │
│  │  │  │ ConfigMap          │      │ Secret             │          │   │ │
│  │  │  │                    │      │                    │          │   │ │
│  │  │  │ • GRPC_HOST        │      │ • REGISTRY_API_KEY │          │   │ │
│  │  │  │ • GRPC_PORT        │      │ • VAULT_API_KEY    │          │   │ │
│  │  │  │ • METRICS_PORT     │      │ • TLS certificates │          │   │ │
│  │  │  │ • DB_PATH          │      │   (optional)       │          │   │ │
│  │  │  │ • RUST_LOG         │      │                    │          │   │ │
│  │  │  │ • REGISTRY_URL     │      │                    │          │   │ │
│  │  │  │ • VAULT_URL        │      │                    │          │   │ │
│  │  │  │ • PLUGIN_DIRS      │      │                    │          │   │ │
│  │  │  └────────────────────┘      └────────────────────┘          │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────────────┐          │   │ │
│  │  │  │ ServiceAccount: memory-graph                   │          │   │ │
│  │  │  │ Role: read ConfigMaps, Secrets, Pods           │          │   │ │
│  │  │  │ RoleBinding: Attached to ServiceAccount        │          │   │ │
│  │  │  └────────────────────────────────────────────────┘          │   │ │
│  │  └──────────────────────────────────────────────────────────────┘   │ │
│  │                                                                       │ │
│  │  ┌──────────────────────────────────────────────────────────────┐   │ │
│  │  │ MONITORING TIER                                              │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────────────────────┐  │   │ │
│  │  │  │ ServiceMonitor (Prometheus Operator)                   │  │   │ │
│  │  │  │                                                         │  │   │ │
│  │  │  │ • Scrape interval: 30s                                 │  │   │ │
│  │  │  │ • Target: /metrics:9090                                │  │   │ │
│  │  │  │ • Metrics collected:                                   │  │   │ │
│  │  │  │   - memory_graph_grpc_requests_total                   │  │   │ │
│  │  │  │   - memory_graph_grpc_request_duration_seconds         │  │   │ │
│  │  │  │   - memory_graph_grpc_active_streams                   │  │   │ │
│  │  │  │   - memory_graph_plugin_*                              │  │   │ │
│  │  │  │   - memory_graph_vault_*                               │  │   │ │
│  │  │  │   - memory_graph_registry_*                            │  │   │ │
│  │  │  └────────────────────────────────────────────────────────┘  │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────────────────────┐  │   │ │
│  │  │  │ PrometheusRule (10 Alerts)                             │  │   │ │
│  │  │  │                                                         │  │   │ │
│  │  │  │ Critical:                                               │  │   │ │
│  │  │  │ • MemoryGraphServiceDown                               │  │   │ │
│  │  │  │ • MemoryGraphLowReplicaCount                           │  │   │ │
│  │  │  │                                                         │  │   │ │
│  │  │  │ Warning:                                                │  │   │ │
│  │  │  │ • MemoryGraphHighErrorRate                             │  │   │ │
│  │  │  │ • MemoryGraphHighLatency                               │  │   │ │
│  │  │  │ • MemoryGraphHighMemoryUsage                           │  │   │ │
│  │  │  │ • MemoryGraphHighCPUUsage                              │  │   │ │
│  │  │  │ • MemoryGraphPodRestarting                             │  │   │ │
│  │  │  │ • MemoryGraphHighStorageUsage                          │  │   │ │
│  │  │  │ • MemoryGraphPluginErrors                              │  │   │ │
│  │  │  │ • MemoryGraphIntegrationFailure                        │  │   │ │
│  │  │  └────────────────────────────────────────────────────────┘  │   │ │
│  │  └──────────────────────────────────────────────────────────────┘   │ │
│  │                                                                       │ │
│  │  ┌──────────────────────────────────────────────────────────────┐   │ │
│  │  │ NETWORK TIER                                                 │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────┐                  │   │ │
│  │  │  │ Service: memory-graph (LoadBalancer)   │                  │   │ │
│  │  │  │ • External access                       │                  │   │ │
│  │  │  │ • Session affinity: ClientIP (3h)      │                  │   │ │
│  │  │  │ • Ports: 50051, 9090                   │                  │   │ │
│  │  │  └────────────────────────────────────────┘                  │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────┐                  │   │ │
│  │  │  │ Service: memory-graph-internal (ClusterIP)               │   │ │
│  │  │  │ • In-cluster access only                │                  │   │ │
│  │  │  │ • Used by other services                │                  │   │ │
│  │  │  └────────────────────────────────────────┘                  │   │ │
│  │  │                                                               │   │ │
│  │  │  ┌────────────────────────────────────────┐                  │   │ │
│  │  │  │ Service: memory-graph-headless         │                  │   │ │
│  │  │  │ • Direct pod access                     │                  │   │ │
│  │  │  │ • StatefulSet scenarios                 │                  │   │ │
│  │  │  └────────────────────────────────────────┘                  │   │ │
│  │  └──────────────────────────────────────────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
│                                                                           │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ EXTERNAL INTEGRATIONS                                            │   │
│  │                                                                   │   │
│  │  ┌──────────────────┐      ┌──────────────────┐                  │   │
│  │  │ LLM-Registry     │◄─────┤ HTTP Client      │                  │   │
│  │  │ :8080            │      │ (Session tracking,│                  │   │
│  │  │                  │      │ model metadata)   │                  │   │
│  │  └──────────────────┘      └──────────────────┘                  │   │
│  │                                                                   │   │
│  │  ┌──────────────────┐      ┌──────────────────┐                  │   │
│  │  │ Data-Vault       │◄─────┤ HTTP Client      │                  │   │
│  │  │ :9000            │      │ (Archival,        │                  │   │
│  │  │                  │      │ compliance)       │                  │   │
│  │  └──────────────────┘      └──────────────────┘                  │   │
│  │                                                                   │   │
│  │  ┌──────────────────┐      ┌──────────────────┐                  │   │
│  │  │ Prometheus       │◄─────┤ ServiceMonitor   │                  │   │
│  │  │ (Metrics)        │      │ (Scraping)        │                  │   │
│  │  └──────────────────┘      └──────────────────┘                  │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

## Component Details

### Pod Architecture

Each pod runs a single container with the LLM-Memory-Graph gRPC server:

```
┌─────────────────────────────────────────┐
│ Pod: memory-graph-xxxxx                 │
├─────────────────────────────────────────┤
│                                         │
│ ┌─────────────────────────────────────┐ │
│ │ Container: memory-graph             │ │
│ │                                     │ │
│ │ Image: ghcr.io/.../llm-memory-graph│ │
│ │ User: appuser (UID 1001)            │ │
│ │                                     │ │
│ │ Ports:                              │ │
│ │ • 50051 (gRPC)                      │ │
│ │ • 9090 (Metrics)                    │ │
│ │                                     │ │
│ │ Volumes:                            │ │
│ │ • /data (PVC: memory-graph-data)    │ │
│ │ • /plugins (PVC: memory-graph-plugins)│
│ │ • /tmp (emptyDir)                   │ │
│ │                                     │ │
│ │ Environment:                        │ │
│ │ • From ConfigMap                    │ │
│ │ • From Secret                       │ │
│ │ • Field refs (pod metadata)         │ │
│ │                                     │ │
│ │ Security:                           │ │
│ │ • runAsNonRoot: true                │ │
│ │ • readOnlyRootFilesystem: true      │ │
│ │ • allowPrivilegeEscalation: false   │ │
│ │ • capabilities: drop ALL            │ │
│ └─────────────────────────────────────┘ │
└─────────────────────────────────────────┘
```

## Traffic Flow

### External Client → gRPC Service

```
External Client
     │
     ▼
LoadBalancer (Cloud Provider)
     │
     ▼
Service: memory-graph (LoadBalancer)
     │
     ├─► Pod 1 (Zone A)
     ├─► Pod 2 (Zone B)
     └─► Pod 3 (Zone C)
          │
          ├─► Database (/data PVC)
          ├─► Plugins (/plugins PVC)
          └─► External Services
               ├─► LLM-Registry
               └─► Data-Vault
```

### Prometheus → Metrics Scraping

```
Prometheus
     │
     ▼
ServiceMonitor (CRD)
     │
     ▼
Service: memory-graph:9090
     │
     ├─► Pod 1 /metrics
     ├─► Pod 2 /metrics
     └─► Pod 3 /metrics
```

## Scaling Behavior

### Auto-scaling Logic

```
Metrics Collection (30s interval)
     │
     ▼
HPA Evaluates Metrics
     │
     ├─► CPU > 70%? ───────────┐
     ├─► Memory > 80%? ─────────┤
     ├─► RPS > 1000? ──────────►├─► Scale Decision
     ├─► Latency p95 > 100ms? ─┤
     └─► Active streams > 100? ─┘
          │
          ▼
     ┌────────────┐
     │ Scale Up?  │ ─── Yes ──► Add 2 pods (max)
     │            │             Stabilization: 60s
     └────────────┘
          │
          ▼
     ┌────────────┐
     │ Scale Down?│ ─── Yes ──► Remove 1 pod (max)
     │            │             Stabilization: 300s
     └────────────┘
```

## Resource Distribution

```
┌───────────────────────────────────────────────────────┐
│ Cluster Resources (per pod)                          │
├───────────────────────────────────────────────────────┤
│                                                       │
│ Memory:    ████████████░░░░░░░░  2Gi / 4Gi (request/limit) │
│ CPU:       ████████████░░░░░░░░  1 / 2 cores          │
│ Storage:   ██████████████████░░  100Gi data + 10Gi plugins │
│ Ephemeral: ████░░░░░░░░░░░░░░░░  10Gi / 20Gi          │
│                                                       │
│ Total for 3 replicas:                                 │
│ Memory:    6-12Gi                                     │
│ CPU:       3-6 cores                                  │
│ Storage:   100Gi (shared) + 10Gi (shared)            │
│                                                       │
└───────────────────────────────────────────────────────┘
```

## High Availability Strategy

```
┌─────────────────────────────────────────────────────────┐
│ HA Components                                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ 1. Multiple Replicas (3 minimum)                       │
│    └─► Spread across availability zones                │
│                                                         │
│ 2. Pod Anti-Affinity                                   │
│    └─► Different nodes for each pod                    │
│                                                         │
│ 3. Topology Spread Constraints                         │
│    └─► Even distribution across zones/nodes            │
│                                                         │
│ 4. Rolling Update Strategy                             │
│    ├─► maxSurge: 1 (one extra pod during update)       │
│    └─► maxUnavailable: 0 (zero downtime)               │
│                                                         │
│ 5. Health Probes                                       │
│    ├─► Liveness: Restart unhealthy pods                │
│    ├─► Readiness: Remove from load balancer            │
│    └─► Startup: Allow slow initialization              │
│                                                         │
│ 6. Graceful Shutdown                                   │
│    ├─► PreStop hook: 15s delay                         │
│    └─► Termination grace: 30s                          │
│                                                         │
│ 7. Service Load Balancing                              │
│    └─► Distribute across healthy pods only             │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## Security Layers

```
┌──────────────────────────────────────────────────────┐
│ Security Architecture                                │
├──────────────────────────────────────────────────────┤
│                                                      │
│ Layer 1: Network                                     │
│ ├─► LoadBalancer (cloud provider security)          │
│ ├─► Service (ClusterIP for internal only)           │
│ └─► Network Policy (optional, recommended)           │
│                                                      │
│ Layer 2: Authentication & Authorization              │
│ ├─► RBAC (ServiceAccount, Role, RoleBinding)        │
│ ├─► API keys in Secrets                             │
│ └─► Optional: mTLS, OAuth2, JWT                     │
│                                                      │
│ Layer 3: Pod Security                               │
│ ├─► Non-root user (UID 1001)                        │
│ ├─► Read-only root filesystem                       │
│ ├─► No privilege escalation                         │
│ ├─► Drop ALL capabilities                           │
│ └─► seccompProfile: RuntimeDefault                  │
│                                                      │
│ Layer 4: Data Security                              │
│ ├─► Secrets (base64 encoded)                        │
│ ├─► Optional: Sealed Secrets / Vault                │
│ ├─► PVC encryption (cloud provider)                 │
│ └─► TLS in transit (optional, recommended)          │
│                                                      │
│ Layer 5: Runtime Security                           │
│ ├─► Resource limits (prevent DoS)                   │
│ ├─► Health probes (detect compromise)               │
│ └─► Monitoring & alerting (detect anomalies)        │
│                                                      │
└──────────────────────────────────────────────────────┘
```

## Deployment Workflow

```
Developer/CI → kubectl apply → API Server → Scheduler
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │ Deployment Controller│
                         └──────────┬───────────┘
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │ ReplicaSet Controller│
                         └──────────┬───────────┘
                                    │
                         ┌──────────▼───────────┐
                         │ Create 3 Pods        │
                         │ (rolling update)     │
                         └──────────┬───────────┘
                                    │
           ┌────────────────────────┼────────────────────────┐
           │                        │                        │
           ▼                        ▼                        ▼
    ┌──────────┐            ┌──────────┐            ┌──────────┐
    │ Pod 1    │            │ Pod 2    │            │ Pod 3    │
    │ (Zone A) │            │ (Zone B) │            │ (Zone C) │
    └────┬─────┘            └────┬─────┘            └────┬─────┘
         │                       │                       │
         │ Startup Probe (60s max)                      │
         │ Readiness Probe (mark ready)                 │
         │ Liveness Probe (monitor health)              │
         │                       │                       │
         └───────────────────────┴───────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │ Service Endpoints      │
                    │ Add to load balancer   │
                    └────────────────────────┘
                                 │
                                 ▼
                    ┌────────────────────────┐
                    │ Ready to serve traffic │
                    └────────────────────────┘
```

## Monitoring Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Observability Stack                                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Application (Pod)                                       │
│     │                                                   │
│     ├─► Logs ──────────────────► Kubernetes API        │
│     │                                    │              │
│     │                                    ▼              │
│     │                             Log Aggregator        │
│     │                             (ELK/Loki)            │
│     │                                                   │
│     ├─► Metrics (/metrics:9090) ► ServiceMonitor       │
│     │                                    │              │
│     │                                    ▼              │
│     │                               Prometheus          │
│     │                                    │              │
│     │                                    ▼              │
│     │                               Grafana             │
│     │                                    │              │
│     │                                    ▼              │
│     │                               Alertmanager        │
│     │                                                   │
│     └─► Traces (optional) ────────► OpenTelemetry      │
│                                             │           │
│                                             ▼           │
│                                       Jaeger/Zipkin     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

---

**Architecture Version**: 1.0.0
**Last Updated**: 2025-11-07
**Maintained By**: LLM DevOps Team
