#!/bin/bash
# Quick deployment script for LLM-Memory-Graph on Kubernetes
# Usage: ./deploy.sh [namespace]

set -e

NAMESPACE="${1:-llm-memory-graph}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================"
echo "LLM-Memory-Graph Kubernetes Deployment"
echo "Namespace: $NAMESPACE"
echo "================================================"
echo ""

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    echo "ERROR: kubectl is not installed"
    exit 1
fi

# Check cluster connection
if ! kubectl cluster-info &> /dev/null; then
    echo "ERROR: Cannot connect to Kubernetes cluster"
    exit 1
fi

echo -e "${BLUE}Connected to cluster:${NC}"
kubectl cluster-info | head -n 1
echo ""

# Step 1: Create namespace
echo -e "${BLUE}Step 1: Creating namespace...${NC}"
if kubectl apply -f "$SCRIPT_DIR/namespace.yaml"; then
    echo -e "${GREEN}✓${NC} Namespace created/updated"
else
    echo "ERROR: Failed to create namespace"
    exit 1
fi
echo ""

# Step 2: Create ConfigMap
echo -e "${BLUE}Step 2: Creating ConfigMap...${NC}"
if kubectl apply -f "$SCRIPT_DIR/configmap.yaml"; then
    echo -e "${GREEN}✓${NC} ConfigMap created/updated"
else
    echo "ERROR: Failed to create ConfigMap"
    exit 1
fi
echo ""

# Step 3: Create Secret
echo -e "${BLUE}Step 3: Creating Secret...${NC}"
echo -e "${YELLOW}⚠ WARNING: Update secret.yaml with actual API keys before production!${NC}"
if kubectl apply -f "$SCRIPT_DIR/secret.yaml"; then
    echo -e "${GREEN}✓${NC} Secret created/updated"
else
    echo "ERROR: Failed to create Secret"
    exit 1
fi
echo ""

# Step 4: Create PVCs
echo -e "${BLUE}Step 4: Creating Persistent Volume Claims...${NC}"
if kubectl apply -f "$SCRIPT_DIR/pvc.yaml"; then
    echo -e "${GREEN}✓${NC} PVCs created/updated"

    # Wait for PVCs to be bound
    echo "Waiting for PVCs to be bound..."
    kubectl wait --for=condition=Bound pvc/memory-graph-data -n "$NAMESPACE" --timeout=60s || true
    kubectl wait --for=condition=Bound pvc/memory-graph-plugins -n "$NAMESPACE" --timeout=60s || true
else
    echo "ERROR: Failed to create PVCs"
    exit 1
fi
echo ""

# Step 5: Create Deployment
echo -e "${BLUE}Step 5: Creating Deployment...${NC}"
if kubectl apply -f "$SCRIPT_DIR/deployment.yaml"; then
    echo -e "${GREEN}✓${NC} Deployment created/updated"

    # Wait for rollout
    echo "Waiting for deployment rollout..."
    kubectl rollout status deployment/memory-graph -n "$NAMESPACE" --timeout=5m || true
else
    echo "ERROR: Failed to create Deployment"
    exit 1
fi
echo ""

# Step 6: Create Service
echo -e "${BLUE}Step 6: Creating Services...${NC}"
if kubectl apply -f "$SCRIPT_DIR/service.yaml"; then
    echo -e "${GREEN}✓${NC} Services created/updated"
else
    echo "ERROR: Failed to create Services"
    exit 1
fi
echo ""

# Step 7: Create HPA
echo -e "${BLUE}Step 7: Creating HorizontalPodAutoscaler...${NC}"
if kubectl apply -f "$SCRIPT_DIR/hpa.yaml"; then
    echo -e "${GREEN}✓${NC} HPA created/updated"
else
    echo -e "${YELLOW}⚠${NC} Failed to create HPA (metrics-server may not be installed)"
fi
echo ""

# Step 8: Create ServiceMonitor (optional)
echo -e "${BLUE}Step 8: Creating ServiceMonitor...${NC}"
if kubectl get crd servicemonitors.monitoring.coreos.com &> /dev/null; then
    if kubectl apply -f "$SCRIPT_DIR/servicemonitor.yaml"; then
        echo -e "${GREEN}✓${NC} ServiceMonitor created/updated"
    else
        echo -e "${YELLOW}⚠${NC} Failed to create ServiceMonitor"
    fi
else
    echo -e "${YELLOW}⚠${NC} Prometheus Operator not installed - skipping ServiceMonitor"
fi
echo ""

# Display deployment status
echo "================================================"
echo "Deployment Status"
echo "================================================"
echo ""

echo -e "${BLUE}Pods:${NC}"
kubectl get pods -n "$NAMESPACE" -l app=memory-graph -o wide
echo ""

echo -e "${BLUE}Services:${NC}"
kubectl get svc -n "$NAMESPACE"
echo ""

echo -e "${BLUE}PVCs:${NC}"
kubectl get pvc -n "$NAMESPACE"
echo ""

echo -e "${BLUE}HPA:${NC}"
kubectl get hpa -n "$NAMESPACE" || echo "HPA not available"
echo ""

# Get LoadBalancer info
echo -e "${BLUE}Service Endpoints:${NC}"
LB_IP=$(kubectl get svc memory-graph -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
LB_HOSTNAME=$(kubectl get svc memory-graph -n "$NAMESPACE" -o jsonpath='{.status.loadBalancer.ingress[0].hostname}')

if [[ -n "$LB_IP" ]]; then
    echo "gRPC Service: ${LB_IP}:50051"
    echo "Metrics: http://${LB_IP}:9090/metrics"
elif [[ -n "$LB_HOSTNAME" ]]; then
    echo "gRPC Service: ${LB_HOSTNAME}:50051"
    echo "Metrics: http://${LB_HOSTNAME}:9090/metrics"
else
    echo -e "${YELLOW}LoadBalancer external address pending...${NC}"
    echo "Check with: kubectl get svc memory-graph -n $NAMESPACE --watch"
fi
echo ""

# Helpful commands
echo "================================================"
echo "Useful Commands"
echo "================================================"
echo ""
echo "View logs:"
echo "  kubectl logs -f -l app=memory-graph -n $NAMESPACE"
echo ""
echo "Check pod status:"
echo "  kubectl get pods -n $NAMESPACE"
echo ""
echo "Check events:"
echo "  kubectl get events -n $NAMESPACE --sort-by='.lastTimestamp'"
echo ""
echo "Scale deployment:"
echo "  kubectl scale deployment memory-graph --replicas=5 -n $NAMESPACE"
echo ""
echo "Update deployment:"
echo "  kubectl rollout restart deployment memory-graph -n $NAMESPACE"
echo ""
echo "Delete deployment:"
echo "  kubectl delete -f $SCRIPT_DIR/"
echo ""
echo "Run validation:"
echo "  $SCRIPT_DIR/validate.sh $NAMESPACE"
echo ""

echo -e "${GREEN}✓${NC} Deployment complete!"
echo ""
echo "Note: It may take a few minutes for the LoadBalancer to provision"
echo "      and for all pods to become ready."
