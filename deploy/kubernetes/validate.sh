#!/bin/bash
# Kubernetes Deployment Validation Script for LLM-Memory-Graph
# Usage: ./validate.sh [namespace]

set -e

NAMESPACE="${1:-llm-memory-graph}"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================"
echo "LLM-Memory-Graph Deployment Validation"
echo "Namespace: $NAMESPACE"
echo "================================================"
echo ""

# Function to check command exists
check_command() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}✗${NC} $1 is not installed"
        return 1
    else
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    fi
}

# Function to check Kubernetes resource
check_resource() {
    local resource_type=$1
    local resource_name=$2
    local namespace=$3

    if kubectl get "$resource_type" "$resource_name" -n "$namespace" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $resource_type/$resource_name exists"
        return 0
    else
        echo -e "${RED}✗${NC} $resource_type/$resource_name not found"
        return 1
    fi
}

# Check prerequisites
echo "Checking prerequisites..."
check_command kubectl
check_command helm || echo -e "${YELLOW}⚠${NC} helm not installed (optional)"
check_command grpcurl || echo -e "${YELLOW}⚠${NC} grpcurl not installed (optional)"
echo ""

# Check cluster connection
echo "Checking cluster connection..."
if kubectl cluster-info &> /dev/null; then
    echo -e "${GREEN}✓${NC} Connected to cluster"
    kubectl cluster-info | head -n 1
else
    echo -e "${RED}✗${NC} Cannot connect to cluster"
    exit 1
fi
echo ""

# Check namespace
echo "Checking namespace..."
if kubectl get namespace "$NAMESPACE" &> /dev/null; then
    echo -e "${GREEN}✓${NC} Namespace $NAMESPACE exists"
else
    echo -e "${RED}✗${NC} Namespace $NAMESPACE not found"
    echo "Create it with: kubectl apply -f namespace.yaml"
    exit 1
fi
echo ""

# Check ConfigMap and Secret
echo "Checking configuration..."
check_resource configmap memory-graph-config "$NAMESPACE"
check_resource secret memory-graph-secrets "$NAMESPACE"

# Validate secret values
echo ""
echo "Validating secrets..."
REGISTRY_KEY=$(kubectl get secret memory-graph-secrets -n "$NAMESPACE" -o jsonpath='{.data.REGISTRY_API_KEY}' | base64 -d)
VAULT_KEY=$(kubectl get secret memory-graph-secrets -n "$NAMESPACE" -o jsonpath='{.data.VAULT_API_KEY}' | base64 -d)

if [[ "$REGISTRY_KEY" == "replace-with-actual-registry-api-key" ]]; then
    echo -e "${YELLOW}⚠${NC} REGISTRY_API_KEY is using default value - update for production!"
else
    echo -e "${GREEN}✓${NC} REGISTRY_API_KEY is configured"
fi

if [[ "$VAULT_KEY" == "replace-with-actual-vault-api-key" ]]; then
    echo -e "${YELLOW}⚠${NC} VAULT_API_KEY is using default value - update for production!"
else
    echo -e "${GREEN}✓${NC} VAULT_API_KEY is configured"
fi
echo ""

# Check PVCs
echo "Checking storage..."
check_resource pvc memory-graph-data "$NAMESPACE"
check_resource pvc memory-graph-plugins "$NAMESPACE"

# Check PVC status
PVC_STATUS=$(kubectl get pvc memory-graph-data -n "$NAMESPACE" -o jsonpath='{.status.phase}')
if [[ "$PVC_STATUS" == "Bound" ]]; then
    echo -e "${GREEN}✓${NC} PVC memory-graph-data is bound"
else
    echo -e "${RED}✗${NC} PVC memory-graph-data is $PVC_STATUS"
fi
echo ""

# Check Deployment
echo "Checking deployment..."
check_resource deployment memory-graph "$NAMESPACE"

# Check deployment status
DESIRED=$(kubectl get deployment memory-graph -n "$NAMESPACE" -o jsonpath='{.spec.replicas}')
READY=$(kubectl get deployment memory-graph -n "$NAMESPACE" -o jsonpath='{.status.readyReplicas}')
AVAILABLE=$(kubectl get deployment memory-graph -n "$NAMESPACE" -o jsonpath='{.status.availableReplicas}')

echo "Deployment status:"
echo "  Desired replicas: $DESIRED"
echo "  Ready replicas: ${READY:-0}"
echo "  Available replicas: ${AVAILABLE:-0}"

if [[ "${READY:-0}" -eq "$DESIRED" ]]; then
    echo -e "${GREEN}✓${NC} All replicas are ready"
else
    echo -e "${YELLOW}⚠${NC} Not all replicas are ready"
fi
echo ""

# Check Pods
echo "Checking pods..."
PODS=$(kubectl get pods -n "$NAMESPACE" -l app=memory-graph -o jsonpath='{.items[*].metadata.name}')

if [[ -z "$PODS" ]]; then
    echo -e "${RED}✗${NC} No pods found"
else
    for POD in $PODS; do
        POD_STATUS=$(kubectl get pod "$POD" -n "$NAMESPACE" -o jsonpath='{.status.phase}')
        if [[ "$POD_STATUS" == "Running" ]]; then
            echo -e "${GREEN}✓${NC} Pod $POD is $POD_STATUS"
        else
            echo -e "${RED}✗${NC} Pod $POD is $POD_STATUS"
        fi

        # Check container status
        CONTAINER_READY=$(kubectl get pod "$POD" -n "$NAMESPACE" -o jsonpath='{.status.containerStatuses[0].ready}')
        if [[ "$CONTAINER_READY" == "true" ]]; then
            echo -e "  ${GREEN}✓${NC} Container is ready"
        else
            echo -e "  ${RED}✗${NC} Container is not ready"
            # Show container state
            kubectl get pod "$POD" -n "$NAMESPACE" -o jsonpath='{.status.containerStatuses[0].state}' | jq .
        fi
    done
fi
echo ""

# Check Services
echo "Checking services..."
check_resource service memory-graph "$NAMESPACE"
check_resource service memory-graph-headless "$NAMESPACE"
check_resource service memory-graph-internal "$NAMESPACE"

# Get LoadBalancer info
LB_IP=$(kubectl get svc memory-graph -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
LB_HOSTNAME=$(kubectl get svc memory-graph -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

if [[ -n "$LB_IP" ]]; then
    echo -e "${GREEN}✓${NC} LoadBalancer IP: $LB_IP"
elif [[ -n "$LB_HOSTNAME" ]]; then
    echo -e "${GREEN}✓${NC} LoadBalancer hostname: $LB_HOSTNAME"
else
    echo -e "${YELLOW}⚠${NC} LoadBalancer external address pending..."
fi
echo ""

# Check HPA
echo "Checking HorizontalPodAutoscaler..."
if check_resource hpa memory-graph-hpa "$NAMESPACE"; then
    HPA_STATUS=$(kubectl get hpa memory-graph-hpa -n "$NAMESPACE" -o jsonpath='{.status.currentReplicas}')
    echo "  Current replicas: $HPA_STATUS"

    # Check metrics server
    if kubectl get --raw /apis/metrics.k8s.io/v1beta1/nodes &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} Metrics server is available"
    else
        echo -e "  ${YELLOW}⚠${NC} Metrics server is not available - HPA may not work properly"
    fi
fi
echo ""

# Check ServiceMonitor
echo "Checking monitoring..."
if kubectl get crd servicemonitors.monitoring.coreos.com &> /dev/null; then
    if check_resource servicemonitor memory-graph "$NAMESPACE"; then
        echo -e "${GREEN}✓${NC} Prometheus monitoring is configured"
    fi
else
    echo -e "${YELLOW}⚠${NC} Prometheus Operator CRDs not installed - ServiceMonitor skipped"
fi
echo ""

# Check PrometheusRule
if kubectl get crd prometheusrules.monitoring.coreos.com &> /dev/null; then
    if check_resource prometheusrule memory-graph-alerts "$NAMESPACE"; then
        echo -e "${GREEN}✓${NC} Prometheus alerting is configured"
    fi
fi
echo ""

# Check endpoints
echo "Checking service endpoints..."
ENDPOINTS=$(kubectl get endpoints memory-graph -n "$NAMESPACE" -o jsonpath='{.subsets[*].addresses[*].ip}')
if [[ -n "$ENDPOINTS" ]]; then
    echo -e "${GREEN}✓${NC} Service has endpoints: $ENDPOINTS"
else
    echo -e "${RED}✗${NC} Service has no endpoints"
fi
echo ""

# Test connectivity (if grpcurl is available)
if command -v grpcurl &> /dev/null && [[ -n "$LB_IP" ]]; then
    echo "Testing gRPC connectivity..."
    if timeout 5 grpcurl -plaintext "${LB_IP}:50051" list &> /dev/null; then
        echo -e "${GREEN}✓${NC} gRPC service is accessible"

        # Test health check
        if grpcurl -plaintext "${LB_IP}:50051" grpc.health.v1.Health/Check 2>&1 | grep -q "SERVING"; then
            echo -e "${GREEN}✓${NC} Health check passed"
        else
            echo -e "${YELLOW}⚠${NC} Health check not responding"
        fi
    else
        echo -e "${YELLOW}⚠${NC} Cannot connect to gRPC service (may still be starting)"
    fi
    echo ""
fi

# Test metrics endpoint
if [[ -n "$LB_IP" ]] && command -v curl &> /dev/null; then
    echo "Testing metrics endpoint..."
    if timeout 5 curl -s "http://${LB_IP}:9090/metrics" | grep -q "memory_graph_"; then
        echo -e "${GREEN}✓${NC} Metrics endpoint is accessible"
    else
        echo -e "${YELLOW}⚠${NC} Cannot access metrics endpoint (may still be starting)"
    fi
    echo ""
fi

# Check recent events
echo "Recent events:"
kubectl get events -n "$NAMESPACE" --sort-by='.lastTimestamp' | tail -n 10
echo ""

# Summary
echo "================================================"
echo "Validation Summary"
echo "================================================"

# Count ready pods
READY_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=memory-graph -o jsonpath='{.items[*].status.containerStatuses[0].ready}' | grep -o "true" | wc -l)
TOTAL_PODS=$(kubectl get pods -n "$NAMESPACE" -l app=memory-graph --no-headers | wc -l)

echo "Pods: ${READY_PODS}/${TOTAL_PODS} ready"
echo "Replicas: ${READY:-0}/${DESIRED} desired"

if [[ "${READY:-0}" -eq "$DESIRED" ]] && [[ "$READY_PODS" -eq "$TOTAL_PODS" ]]; then
    echo -e "${GREEN}✓${NC} Deployment is healthy"
    exit 0
else
    echo -e "${YELLOW}⚠${NC} Deployment may have issues - check logs and events"
    exit 1
fi
