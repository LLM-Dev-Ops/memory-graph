#!/bin/bash
# Cleanup script for LLM-Memory-Graph Kubernetes deployment
# Usage: ./cleanup.sh [namespace]

set -e

NAMESPACE="${1:-llm-memory-graph}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "================================================"
echo "LLM-Memory-Graph Cleanup"
echo "Namespace: $NAMESPACE"
echo "================================================"
echo ""

# Confirm deletion
echo -e "${RED}WARNING: This will delete all resources in namespace $NAMESPACE${NC}"
echo -e "${YELLOW}This action cannot be undone!${NC}"
echo ""
read -p "Are you sure you want to continue? (type 'yes' to confirm): " CONFIRM

if [[ "$CONFIRM" != "yes" ]]; then
    echo "Cleanup cancelled"
    exit 0
fi
echo ""

# Check if namespace exists
if ! kubectl get namespace "$NAMESPACE" &> /dev/null; then
    echo "Namespace $NAMESPACE does not exist"
    exit 0
fi

# Delete resources in reverse order
echo "Deleting resources..."
echo ""

# ServiceMonitor
if kubectl get crd servicemonitors.monitoring.coreos.com &> /dev/null; then
    echo "Deleting ServiceMonitor..."
    kubectl delete -f "$SCRIPT_DIR/servicemonitor.yaml" --ignore-not-found=true
fi

# HPA
echo "Deleting HPA..."
kubectl delete -f "$SCRIPT_DIR/hpa.yaml" --ignore-not-found=true

# Service
echo "Deleting Services..."
kubectl delete -f "$SCRIPT_DIR/service.yaml" --ignore-not-found=true

# Deployment
echo "Deleting Deployment..."
kubectl delete -f "$SCRIPT_DIR/deployment.yaml" --ignore-not-found=true

# Wait for pods to terminate
echo "Waiting for pods to terminate..."
kubectl wait --for=delete pod -l app=memory-graph -n "$NAMESPACE" --timeout=60s || true

# PVCs
echo ""
echo -e "${YELLOW}PVCs contain data. Do you want to delete them?${NC}"
read -p "Delete PVCs? (type 'yes' to confirm): " DELETE_PVC

if [[ "$DELETE_PVC" == "yes" ]]; then
    echo "Deleting PVCs..."
    kubectl delete -f "$SCRIPT_DIR/pvc.yaml" --ignore-not-found=true
else
    echo "Keeping PVCs (you can delete them later)"
fi

# Secret and ConfigMap
echo ""
echo "Deleting Secret and ConfigMap..."
kubectl delete -f "$SCRIPT_DIR/secret.yaml" --ignore-not-found=true
kubectl delete -f "$SCRIPT_DIR/configmap.yaml" --ignore-not-found=true

# Namespace
echo ""
echo -e "${YELLOW}Do you want to delete the namespace $NAMESPACE?${NC}"
read -p "Delete namespace? (type 'yes' to confirm): " DELETE_NS

if [[ "$DELETE_NS" == "yes" ]]; then
    echo "Deleting namespace..."
    kubectl delete -f "$SCRIPT_DIR/namespace.yaml" --ignore-not-found=true
    echo "Waiting for namespace to be deleted..."
    kubectl wait --for=delete namespace "$NAMESPACE" --timeout=120s || true
else
    echo "Keeping namespace"
fi

echo ""
echo "Cleanup complete!"
echo ""

# Show remaining resources (if namespace still exists)
if kubectl get namespace "$NAMESPACE" &> /dev/null; then
    echo "Remaining resources in namespace $NAMESPACE:"
    kubectl get all -n "$NAMESPACE"
fi
