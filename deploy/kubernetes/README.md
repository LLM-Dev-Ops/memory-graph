# LLM-Memory-Graph Kubernetes Deployment Guide

Complete production-grade Kubernetes deployment manifests for the LLM-Memory-Graph gRPC service.

## Overview

This deployment provides:
- **High Availability**: 3 replicas with anti-affinity rules
- **Auto-scaling**: HorizontalPodAutoscaler based on CPU, memory, and custom metrics
- **Monitoring**: Prometheus ServiceMonitor with comprehensive alerting
- **Security**: Non-root containers, read-only filesystem, RBAC, secrets management
- **Storage**: Persistent volumes for data and plugins
- **Health Checks**: gRPC-based liveness, readiness, and startup probes

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Kubernetes Cluster                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              Namespace: llm-memory-graph                  │  │
│  ├───────────────────────────────────────────────────────────┤  │
│  │                                                           │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │  │
│  │  │ Pod 1       │  │ Pod 2       │  │ Pod 3       │      │  │
│  │  │ (Zone A)    │  │ (Zone B)    │  │ (Zone C)    │      │  │
│  │  ├─────────────┤  ├─────────────┤  ├─────────────┤      │  │
│  │  │ gRPC :50051 │  │ gRPC :50051 │  │ gRPC :50051 │      │  │
│  │  │ Metrics:9090│  │ Metrics:9090│  │ Metrics:9090│      │  │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │  │
│  │         │                │                │             │  │
│  │         └────────────────┴────────────────┘             │  │
│  │                          │                              │  │
│  │                   ┌──────▼──────┐                       │  │
│  │                   │  Service    │                       │  │
│  │                   │ LoadBalancer│                       │  │
│  │                   └──────┬──────┘                       │  │
│  │                          │                              │  │
│  │  ┌───────────────────────▼───────────────────────────┐  │  │
│  │  │         HorizontalPodAutoscaler (3-10)           │  │  │
│  │  │  - CPU: 70%                                      │  │  │
│  │  │  - Memory: 80%                                   │  │  │
│  │  │  - Custom: grpc_requests_per_second              │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                         │  │
│  │  ┌──────────────────────────────────────────────────┐  │  │
│  │  │           Prometheus ServiceMonitor              │  │  │
│  │  │  - Scrape /metrics every 30s                     │  │  │
│  │  │  - PrometheusRule for alerts                     │  │  │
│  │  └──────────────────────────────────────────────────┘  │  │
│  │                                                         │  │
│  │  ┌──────────────────┐  ┌──────────────────┐           │  │
│  │  │ PVC: data        │  │ PVC: plugins     │           │  │
│  │  │ 100Gi            │  │ 10Gi             │           │  │
│  │  └──────────────────┘  └──────────────────┘           │  │
│  │                                                         │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Prerequisites

### Required

1. **Kubernetes Cluster**: v1.24+
   ```bash
   kubectl version --short
   ```

2. **kubectl**: Configured to access your cluster
   ```bash
   kubectl cluster-info
   ```

3. **Storage Class**: For persistent volumes
   ```bash
   kubectl get storageclass
   ```

### Optional

4. **Prometheus Operator**: For ServiceMonitor and alerting
   ```bash
   kubectl get crd servicemonitors.monitoring.coreos.com
   ```

5. **Metrics Server**: For resource-based HPA
   ```bash
   kubectl get deployment metrics-server -n kube-system
   ```

6. **Prometheus Adapter**: For custom metrics HPA
   ```bash
   kubectl get deployment prometheus-adapter -n monitoring
   ```

## Quick Start

### 1. Deploy All Resources

```bash
# Navigate to kubernetes directory
cd deploy/kubernetes

# Apply all manifests in order
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f pvc.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f hpa.yaml
kubectl apply -f servicemonitor.yaml

# Or apply all at once
kubectl apply -f .
```

### 2. Verify Deployment

```bash
# Check namespace
kubectl get namespace llm-memory-graph

# Check all resources
kubectl get all -n llm-memory-graph

# Check pods
kubectl get pods -n llm-memory-graph -o wide

# Check services
kubectl get svc -n llm-memory-graph

# Check PVCs
kubectl get pvc -n llm-memory-graph

# Check HPA
kubectl get hpa -n llm-memory-graph

# Check ServiceMonitor (if Prometheus Operator is installed)
kubectl get servicemonitor -n llm-memory-graph
```

### 3. Access the Service

```bash
# Get LoadBalancer external IP
kubectl get svc memory-graph -n llm-memory-graph

# Get service details
kubectl describe svc memory-graph -n llm-memory-graph

# Test gRPC health check (requires grpcurl)
EXTERNAL_IP=$(kubectl get svc memory-graph -n llm-memory-graph -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
grpcurl -plaintext ${EXTERNAL_IP}:50051 grpc.health.v1.Health/Check

# Test metrics endpoint
curl http://${EXTERNAL_IP}:9090/metrics
```

## Configuration

### Update Secrets

**IMPORTANT**: Replace placeholder secrets before deploying to production!

```bash
# Edit secret.yaml with actual values
kubectl edit secret memory-graph-secrets -n llm-memory-graph

# Or use kubectl create secret
kubectl create secret generic memory-graph-secrets \
  --from-literal=REGISTRY_API_KEY=your-actual-registry-key \
  --from-literal=VAULT_API_KEY=your-actual-vault-key \
  -n llm-memory-graph \
  --dry-run=client -o yaml | kubectl apply -f -
```

### Update ConfigMap

```bash
# Edit configuration
kubectl edit configmap memory-graph-config -n llm-memory-graph

# Restart pods to apply changes
kubectl rollout restart deployment memory-graph -n llm-memory-graph
```

### Scale Replicas

```bash
# Manual scaling
kubectl scale deployment memory-graph --replicas=5 -n llm-memory-graph

# Update HPA limits
kubectl edit hpa memory-graph-hpa -n llm-memory-graph
```

## Monitoring

### View Logs

```bash
# All pods
kubectl logs -f -l app=memory-graph -n llm-memory-graph

# Specific pod
kubectl logs -f memory-graph-<pod-id> -n llm-memory-graph

# Previous container (if pod restarted)
kubectl logs memory-graph-<pod-id> -n llm-memory-graph --previous

# Tail last 100 lines
kubectl logs --tail=100 -l app=memory-graph -n llm-memory-graph
```

### Check Metrics

```bash
# Get metrics from pods
kubectl top pods -n llm-memory-graph

# Get HPA status
kubectl get hpa memory-graph-hpa -n llm-memory-graph --watch

# Describe HPA for detailed metrics
kubectl describe hpa memory-graph-hpa -n llm-memory-graph
```

### View Events

```bash
# Namespace events
kubectl get events -n llm-memory-graph --sort-by='.lastTimestamp'

# Deployment events
kubectl describe deployment memory-graph -n llm-memory-graph

# Pod events
kubectl describe pod memory-graph-<pod-id> -n llm-memory-graph
```

## Troubleshooting

### Pods Not Starting

```bash
# Check pod status
kubectl get pods -n llm-memory-graph

# Describe pod
kubectl describe pod memory-graph-<pod-id> -n llm-memory-graph

# Check pod logs
kubectl logs memory-graph-<pod-id> -n llm-memory-graph

# Check init container logs
kubectl logs memory-graph-<pod-id> -n llm-memory-graph -c init-permissions

# Check events
kubectl get events -n llm-memory-graph --field-selector involvedObject.name=memory-graph-<pod-id>
```

### Service Not Accessible

```bash
# Check service
kubectl get svc memory-graph -n llm-memory-graph

# Check endpoints
kubectl get endpoints memory-graph -n llm-memory-graph

# Test from within cluster
kubectl run -it --rm debug --image=alpine --restart=Never -n llm-memory-graph -- sh
# Inside the pod:
apk add curl
curl http://memory-graph:9090/metrics

# Port-forward for local testing
kubectl port-forward svc/memory-graph 50051:50051 -n llm-memory-graph
```

### PVC Issues

```bash
# Check PVC status
kubectl get pvc -n llm-memory-graph

# Describe PVC
kubectl describe pvc memory-graph-data -n llm-memory-graph

# Check storage class
kubectl get storageclass

# Check PV
kubectl get pv
```

### HPA Not Scaling

```bash
# Check HPA status
kubectl describe hpa memory-graph-hpa -n llm-memory-graph

# Check metrics server
kubectl get --raw /apis/metrics.k8s.io/v1beta1/pods

# Check custom metrics (if using)
kubectl get --raw /apis/custom.metrics.k8s.io/v1beta1

# Check prometheus adapter (if using)
kubectl logs -n monitoring -l app=prometheus-adapter
```

## Maintenance

### Rolling Updates

```bash
# Update image
kubectl set image deployment/memory-graph \
  memory-graph=ghcr.io/globalbusinessadvisors/llm-memory-graph:v1.1.0 \
  -n llm-memory-graph

# Check rollout status
kubectl rollout status deployment/memory-graph -n llm-memory-graph

# Rollback if needed
kubectl rollout undo deployment/memory-graph -n llm-memory-graph

# View rollout history
kubectl rollout history deployment/memory-graph -n llm-memory-graph
```

### Backup Data

```bash
# Create snapshot of PVC (method varies by storage provider)
# For example, using kubectl cp:
kubectl exec -n llm-memory-graph memory-graph-<pod-id> -- tar czf /tmp/backup.tar.gz /data
kubectl cp llm-memory-graph/memory-graph-<pod-id>:/tmp/backup.tar.gz ./backup.tar.gz
```

### Disaster Recovery

```bash
# Delete deployment (keeps PVC)
kubectl delete deployment memory-graph -n llm-memory-graph

# Recreate deployment
kubectl apply -f deployment.yaml

# Or delete everything and start fresh
kubectl delete namespace llm-memory-graph
kubectl apply -f .
```

## Performance Tuning

### Resource Limits

Edit `deployment.yaml` to adjust resource requests/limits:

```yaml
resources:
  requests:
    memory: "4Gi"  # Increase for better performance
    cpu: "2000m"
  limits:
    memory: "8Gi"
    cpu: "4000m"
```

### HPA Tuning

Edit `hpa.yaml` to adjust scaling behavior:

```yaml
spec:
  minReplicas: 5      # Increase for higher baseline capacity
  maxReplicas: 20     # Increase for higher peak capacity
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 60  # Lower threshold = earlier scaling
```

### Storage Performance

Use faster storage class in `pvc.yaml`:

```yaml
spec:
  storageClassName: ssd  # or premium-rwo, gp3, etc.
```

## Security

### RBAC

The deployment includes minimal RBAC permissions. Review and adjust `deployment.yaml` Role/RoleBinding as needed.

### Network Policies

Consider adding NetworkPolicy for additional isolation:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: memory-graph-netpol
  namespace: llm-memory-graph
spec:
  podSelector:
    matchLabels:
      app: memory-graph
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: ingress-nginx
    ports:
    - protocol: TCP
      port: 50051
  - from:
    - namespaceSelector:
        matchLabels:
          name: monitoring
    ports:
    - protocol: TCP
      port: 9090
  egress:
  - to:
    - namespaceSelector:
        matchLabels:
          name: llm-services
    ports:
    - protocol: TCP
      port: 8080
    - protocol: TCP
      port: 9000
```

### TLS Configuration

For production, enable TLS by:

1. Create TLS secret:
```bash
kubectl create secret tls memory-graph-tls \
  --cert=path/to/tls.crt \
  --key=path/to/tls.key \
  -n llm-memory-graph
```

2. Mount secret in deployment and configure gRPC server for TLS

## Multi-Environment Deployment

### Namespaces

Create separate namespaces for different environments:

```bash
# Development
kubectl create namespace llm-memory-graph-dev

# Staging
kubectl create namespace llm-memory-graph-staging

# Production
kubectl create namespace llm-memory-graph-prod
```

### Kustomize

Use kustomize for environment-specific configurations:

```
deploy/kubernetes/
├── base/
│   ├── kustomization.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   └── ...
├── overlays/
│   ├── dev/
│   │   ├── kustomization.yaml
│   │   └── patches/
│   ├── staging/
│   │   ├── kustomization.yaml
│   │   └── patches/
│   └── prod/
│       ├── kustomization.yaml
│       └── patches/
```

Apply with:
```bash
kubectl apply -k deploy/kubernetes/overlays/prod
```

## Cost Optimization

### Right-sizing

Monitor actual resource usage and adjust requests/limits:

```bash
# Check actual usage
kubectl top pods -n llm-memory-graph

# Adjust based on metrics
kubectl set resources deployment memory-graph \
  --requests=cpu=500m,memory=1Gi \
  --limits=cpu=1000m,memory=2Gi \
  -n llm-memory-graph
```

### Spot Instances

Use node selectors/tolerations for spot instances:

```yaml
spec:
  template:
    spec:
      nodeSelector:
        node.kubernetes.io/instance-type: spot
      tolerations:
      - key: spot
        operator: Equal
        value: "true"
        effect: NoSchedule
```

### Storage Optimization

- Use lifecycle policies for old data
- Enable compression
- Use cheaper storage tiers for archives

## References

- [Kubernetes Documentation](https://kubernetes.io/docs/)
- [Prometheus Operator](https://github.com/prometheus-operator/prometheus-operator)
- [HPA Documentation](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/)
- [gRPC Health Check Protocol](https://github.com/grpc/grpc/blob/master/doc/health-checking.md)
- [Production Best Practices](https://kubernetes.io/docs/setup/best-practices/)

## Support

For issues or questions:
- GitHub: https://github.com/globalbusinessadvisors/llm-memory-graph
- Documentation: See `/docs` directory
- Implementation Plan: `/plans/PRODUCTION-PHASE-IMPLEMENTATION-PLAN.md`

---

**Version**: 1.0.0
**Last Updated**: 2025-11-07
**Maintained By**: LLM DevOps Team
