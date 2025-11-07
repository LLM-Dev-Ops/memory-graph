# LLM-Memory-Graph Kubernetes - Quick Start

## Prerequisites Check

```bash
# Check kubectl
kubectl version --short

# Check cluster access
kubectl cluster-info

# Check storage classes
kubectl get storageclass
```

## 5-Minute Deployment

### Option 1: Automated Script (Recommended)

```bash
cd deploy/kubernetes
./deploy.sh llm-memory-graph
```

### Option 2: Manual Deployment

```bash
cd deploy/kubernetes

# 1. Create namespace
kubectl apply -f namespace.yaml

# 2. Deploy configuration
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml

# 3. Create storage
kubectl apply -f pvc.yaml

# 4. Deploy application
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml

# 5. Enable auto-scaling
kubectl apply -f hpa.yaml

# 6. Enable monitoring (optional)
kubectl apply -f servicemonitor.yaml
```

### Option 3: Kustomize

```bash
kubectl apply -k deploy/kubernetes/
```

## Verification

```bash
# Run validation script
./validate.sh llm-memory-graph

# OR manually check

# Check pods
kubectl get pods -n llm-memory-graph

# Check services
kubectl get svc -n llm-memory-graph

# Get service endpoint
kubectl get svc memory-graph -n llm-memory-graph
```

## Access the Service

```bash
# Get LoadBalancer IP/hostname
export LB_IP=$(kubectl get svc memory-graph -n llm-memory-graph -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
export LB_HOST=$(kubectl get svc memory-graph -n llm-memory-graph -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

# Test gRPC (requires grpcurl)
grpcurl -plaintext ${LB_IP:-$LB_HOST}:50051 grpc.health.v1.Health/Check

# Test metrics
curl http://${LB_IP:-$LB_HOST}:9090/metrics
```

## Common Tasks

### View Logs
```bash
kubectl logs -f -l app=memory-graph -n llm-memory-graph
```

### Scale Manually
```bash
kubectl scale deployment memory-graph --replicas=5 -n llm-memory-graph
```

### Update Image
```bash
kubectl set image deployment/memory-graph \
  memory-graph=ghcr.io/globalbusinessadvisors/llm-memory-graph:v1.1.0 \
  -n llm-memory-graph
```

### Restart Pods
```bash
kubectl rollout restart deployment memory-graph -n llm-memory-graph
```

### Check HPA
```bash
kubectl get hpa -n llm-memory-graph --watch
```

## Cleanup

```bash
./cleanup.sh llm-memory-graph
```

## IMPORTANT: Before Production

1. **Update Secrets**:
   ```bash
   kubectl edit secret memory-graph-secrets -n llm-memory-graph
   ```
   Replace placeholder API keys with actual values!

2. **Verify Storage Class**:
   Ensure your cluster has the required storage class (default: `standard`)

3. **Configure LoadBalancer**:
   Check cloud provider annotations in `service.yaml`

4. **Enable Monitoring**:
   Ensure Prometheus Operator is installed for ServiceMonitor

5. **Review Resource Limits**:
   Adjust in `deployment.yaml` based on your needs

## Need Help?

- Full Documentation: [README.md](README.md)
- Deployment Report: [DEPLOYMENT-REPORT.md](DEPLOYMENT-REPORT.md)
- Implementation Plan: `/plans/PRODUCTION-PHASE-IMPLEMENTATION-PLAN.md`
