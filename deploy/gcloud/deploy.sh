#!/bin/bash
# ============================================================================
# LLM-Memory-Graph Deployment Script
# ============================================================================
# Deploys the unified LLM-Memory-Graph service to Google Cloud Run.
#
# Usage:
#   ./deploy.sh [ENV] [REGION]
#
# Arguments:
#   ENV    - Environment: dev, staging, prod (default: dev)
#   REGION - GCP region (default: us-central1)
#
# Examples:
#   ./deploy.sh                    # Deploy to dev
#   ./deploy.sh staging            # Deploy to staging
#   ./deploy.sh prod us-east1      # Deploy to prod in us-east1

set -euo pipefail

# Configuration
ENV="${1:-dev}"
REGION="${2:-us-central1}"
PROJECT_ID="${PROJECT_ID:-$(gcloud config get-value project)}"
SERVICE_NAME="llm-memory-graph"
IMAGE_TAG="${IMAGE_TAG:-$(git rev-parse --short HEAD 2>/dev/null || echo 'latest')}"

echo "============================================================================"
echo "LLM-Memory-Graph Deployment"
echo "============================================================================"
echo "Project:     ${PROJECT_ID}"
echo "Environment: ${ENV}"
echo "Region:      ${REGION}"
echo "Service:     ${SERVICE_NAME}"
echo "Image Tag:   ${IMAGE_TAG}"
echo ""

# Validate environment
if [[ ! "${ENV}" =~ ^(dev|staging|prod)$ ]]; then
    echo "ERROR: Invalid environment '${ENV}'. Must be dev, staging, or prod."
    exit 1
fi

# Confirm production deployment
if [[ "${ENV}" == "prod" ]]; then
    echo "⚠️  WARNING: You are deploying to PRODUCTION!"
    read -p "Are you sure? (type 'yes' to confirm): " CONFIRM
    if [[ "${CONFIRM}" != "yes" ]]; then
        echo "Deployment cancelled."
        exit 0
    fi
fi

# Navigate to repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}/../.."

echo "Step 1: Verifying prerequisites..."

# Check for required files
for FILE in deploy/gcloud/Dockerfile deploy/gcloud/cloudbuild.yaml deploy/gcloud/service-router.js; do
    if [[ ! -f "${FILE}" ]]; then
        echo "ERROR: Missing required file: ${FILE}"
        exit 1
    fi
done

# Check secrets exist
echo "Verifying secrets..."
for SECRET in "ruvector-api-key" "ruvector-service-url-${ENV}" "observatory-endpoint-${ENV}"; do
    if ! gcloud secrets describe "${SECRET}" --project="${PROJECT_ID}" &>/dev/null; then
        echo "ERROR: Missing secret: ${SECRET}"
        echo "Run: ./deploy/gcloud/setup-iam.sh to create secrets"
        exit 1
    fi
done

echo "Step 2: Submitting Cloud Build..."

gcloud builds submit \
    --config=deploy/gcloud/cloudbuild.yaml \
    --substitutions="_ENV=${ENV},_REGION=${REGION},_IMAGE_TAG=${IMAGE_TAG}" \
    --project="${PROJECT_ID}"

echo ""
echo "Step 3: Verifying deployment..."

# Get service URL
SERVICE_URL=$(gcloud run services describe "${SERVICE_NAME}-${ENV}" \
    --region="${REGION}" \
    --project="${PROJECT_ID}" \
    --format='value(status.url)')

echo "Service URL: ${SERVICE_URL}"

# Health check
echo "Running health check..."
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "${SERVICE_URL}/health" || echo "000")

if [[ "${HTTP_STATUS}" == "200" ]]; then
    echo "✅ Health check passed"
else
    echo "❌ Health check failed (HTTP ${HTTP_STATUS})"
    exit 1
fi

# Check topology
echo "Fetching service topology..."
curl -s "${SERVICE_URL}/topology" | jq '.'

echo ""
echo "============================================================================"
echo "Deployment Complete"
echo "============================================================================"
echo ""
echo "Service URL: ${SERVICE_URL}"
echo ""
echo "Agent Endpoints:"
echo "  ${SERVICE_URL}/conversation-memory"
echo "  ${SERVICE_URL}/prompt-lineage"
echo "  ${SERVICE_URL}/decision-memory"
echo "  ${SERVICE_URL}/knowledge-graph-builder"
echo "  ${SERVICE_URL}/memory-retrieval"
echo "  ${SERVICE_URL}/long-term-pattern"
echo ""
echo "CLI Commands:"
echo "  memory-graph agent conversation-memory health --url ${SERVICE_URL}"
echo "  memory-graph agent prompt-lineage inspect <ref> --url ${SERVICE_URL}"
echo ""
echo "Next Steps:"
echo "  1. Verify agents: curl ${SERVICE_URL}/topology"
echo "  2. Test CLI: memory-graph --help"
echo "  3. Monitor: https://console.cloud.google.com/run?project=${PROJECT_ID}"
