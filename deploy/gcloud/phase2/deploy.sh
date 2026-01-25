#!/bin/bash
# =============================================================================
# Phase 2 - Operational Intelligence (Layer 1) Deployment Script
# LLM-Memory-Graph Cloud Run Deployment
# =============================================================================
#
# Usage:
#   ./deploy.sh [ENV] [PROJECT_ID] [REGION]
#
# Arguments:
#   ENV        - Environment: dev | staging | prod (default: dev)
#   PROJECT_ID - Google Cloud Project ID (required)
#   REGION     - Google Cloud Region (default: us-central1)
#
# Prerequisites:
#   - gcloud CLI installed and authenticated
#   - Docker installed (for local builds)
#   - Secrets configured in Google Secret Manager:
#     - ruvector-api-key
#     - ruvector-service-url-{ENV}
#     - observatory-endpoint-{ENV}
#
# =============================================================================

set -e

# Configuration
ENV="${1:-dev}"
PROJECT_ID="${2:-}"
REGION="${3:-us-central1}"
SERVICE_NAME="llm-memory-graph"
IMAGE_NAME="llm-memory-graph"

# Phase 2 identity
AGENT_NAME="llm-memory-graph"
AGENT_DOMAIN="memory"
AGENT_PHASE="phase2"
AGENT_LAYER="layer1"

# Performance budgets
MAX_TOKENS="1000"
MAX_LATENCY_MS="2000"
MAX_CALLS_PER_RUN="3"

# Validate arguments
if [ -z "$PROJECT_ID" ]; then
    echo "ERROR: PROJECT_ID is required"
    echo "Usage: ./deploy.sh [ENV] [PROJECT_ID] [REGION]"
    exit 1
fi

# Validate environment
if [[ ! "$ENV" =~ ^(dev|staging|prod)$ ]]; then
    echo "ERROR: ENV must be dev, staging, or prod"
    exit 1
fi

echo "============================================================"
echo "Phase 2 - Operational Intelligence (Layer 1) Deployment"
echo "============================================================"
echo "Environment:    $ENV"
echo "Project:        $PROJECT_ID"
echo "Region:         $REGION"
echo "Service:        $SERVICE_NAME"
echo "Agent Phase:    $AGENT_PHASE"
echo "Agent Layer:    $AGENT_LAYER"
echo "============================================================"

# Set project
gcloud config set project "$PROJECT_ID"

# Image URL
IMAGE_URL="${REGION}-docker.pkg.dev/${PROJECT_ID}/${IMAGE_NAME}/${IMAGE_NAME}:latest"

# Build and push image
echo ""
echo "[1/3] Building and pushing Docker image..."
cd "$(dirname "$0")/.."

# Build image
docker build -t "$IMAGE_URL" -f Dockerfile .

# Push to Artifact Registry
docker push "$IMAGE_URL"

echo "Image pushed: $IMAGE_URL"

# Deploy to Cloud Run
echo ""
echo "[2/3] Deploying to Cloud Run..."

gcloud run deploy "$SERVICE_NAME" \
    --image "$IMAGE_URL" \
    --region "$REGION" \
    --platform managed \
    --allow-unauthenticated \
    --memory 2Gi \
    --cpu 2 \
    --min-instances 1 \
    --max-instances 100 \
    --concurrency 80 \
    --timeout 300 \
    --set-env-vars "SERVICE_NAME=${SERVICE_NAME}" \
    --set-env-vars "PLATFORM_ENV=${ENV}" \
    --set-env-vars "PORT=8080" \
    --set-env-vars "NODE_ENV=production" \
    --set-env-vars "LOG_LEVEL=warn" \
    --set-env-vars "AGENT_NAME=${AGENT_NAME}" \
    --set-env-vars "AGENT_DOMAIN=${AGENT_DOMAIN}" \
    --set-env-vars "AGENT_PHASE=${AGENT_PHASE}" \
    --set-env-vars "AGENT_LAYER=${AGENT_LAYER}" \
    --set-env-vars "MAX_TOKENS=${MAX_TOKENS}" \
    --set-env-vars "MAX_LATENCY_MS=${MAX_LATENCY_MS}" \
    --set-env-vars "MAX_CALLS_PER_RUN=${MAX_CALLS_PER_RUN}" \
    --set-secrets "RUVECTOR_API_KEY=ruvector-api-key:latest" \
    --set-secrets "RUVECTOR_SERVICE_URL=ruvector-service-url-${ENV}:latest" \
    --set-secrets "TELEMETRY_ENDPOINT=observatory-endpoint-${ENV}:latest" \
    --service-account "${SERVICE_NAME}-sa@${PROJECT_ID}.iam.gserviceaccount.com"

# Verify deployment
echo ""
echo "[3/3] Verifying deployment..."

SERVICE_URL=$(gcloud run services describe "$SERVICE_NAME" \
    --region "$REGION" \
    --format 'value(status.url)')

echo "Service URL: $SERVICE_URL"

# Health check
echo "Running health check..."
HEALTH_RESPONSE=$(curl -s "${SERVICE_URL}/health")
echo "$HEALTH_RESPONSE" | jq .

# Verify Phase 2 requirements
echo ""
echo "============================================================"
echo "Phase 2 Deployment Verification"
echo "============================================================"

# Check for required fields in health response
PHASE=$(echo "$HEALTH_RESPONSE" | jq -r '.phase // "unknown"')
LAYER=$(echo "$HEALTH_RESPONSE" | jq -r '.layer // "unknown"')
RUVECTOR_STATUS=$(echo "$HEALTH_RESPONSE" | jq -r '.dependencies["ruvector-service"].status // "unknown"')

echo "Phase:          $PHASE"
echo "Layer:          $LAYER"
echo "Ruvector:       $RUVECTOR_STATUS"

if [ "$PHASE" != "phase2" ]; then
    echo "WARNING: Agent phase is not 'phase2'"
fi

if [ "$LAYER" != "layer1" ]; then
    echo "WARNING: Agent layer is not 'layer1'"
fi

if [ "$RUVECTOR_STATUS" != "healthy" ]; then
    echo "ERROR: Ruvector is not healthy - deployment may fail at runtime"
fi

echo ""
echo "============================================================"
echo "Deployment Complete"
echo "============================================================"
echo "Service URL:    $SERVICE_URL"
echo "Health:         ${SERVICE_URL}/health"
echo "Topology:       ${SERVICE_URL}/topology"
echo "============================================================"
