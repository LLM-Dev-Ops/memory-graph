#!/bin/bash
# ============================================================================
# LLM-Memory-Graph IAM Setup Script
# ============================================================================
# Creates service accounts and grants minimum required permissions
# following the principle of least privilege.
#
# Usage:
#   ./setup-iam.sh [PROJECT_ID] [REGION]
#
# Prerequisites:
#   - gcloud CLI authenticated
#   - Project owner or IAM admin permissions

set -euo pipefail

PROJECT_ID="${1:-$(gcloud config get-value project)}"
REGION="${2:-us-central1}"
SERVICE_ACCOUNT_NAME="llm-memory-graph-sa"
SERVICE_ACCOUNT_EMAIL="${SERVICE_ACCOUNT_NAME}@${PROJECT_ID}.iam.gserviceaccount.com"

echo "============================================================================"
echo "LLM-Memory-Graph IAM Setup"
echo "============================================================================"
echo "Project: ${PROJECT_ID}"
echo "Region: ${REGION}"
echo "Service Account: ${SERVICE_ACCOUNT_EMAIL}"
echo ""

# Enable required APIs
echo "Enabling required Google Cloud APIs..."
gcloud services enable \
    run.googleapis.com \
    cloudbuild.googleapis.com \
    secretmanager.googleapis.com \
    artifactregistry.googleapis.com \
    --project="${PROJECT_ID}"

# Create Artifact Registry repository
echo "Creating Artifact Registry repository..."
gcloud artifacts repositories create llm-memory-graph \
    --repository-format=docker \
    --location="${REGION}" \
    --description="LLM-Memory-Graph container images" \
    --project="${PROJECT_ID}" 2>/dev/null || echo "Repository already exists"

# Create service account
echo "Creating service account..."
gcloud iam service-accounts create "${SERVICE_ACCOUNT_NAME}" \
    --display-name="LLM-Memory-Graph Service Account" \
    --description="Service account for LLM-Memory-Graph Cloud Run service" \
    --project="${PROJECT_ID}" 2>/dev/null || echo "Service account already exists"

# Grant minimum required permissions
echo "Granting IAM permissions (least privilege)..."

# Cloud Run Invoker (to be invoked by other services)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/run.invoker" \
    --condition=None

# Secret Manager Secret Accessor (read secrets only)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None

# Logging Writer (emit logs)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/logging.logWriter" \
    --condition=None

# Monitoring Metric Writer (emit metrics)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/monitoring.metricWriter" \
    --condition=None

# Cloud Trace Agent (distributed tracing)
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${SERVICE_ACCOUNT_EMAIL}" \
    --role="roles/cloudtrace.agent" \
    --condition=None

# Grant Cloud Build permissions
echo "Setting up Cloud Build permissions..."
CLOUD_BUILD_SA=$(gcloud projects describe ${PROJECT_ID} --format='value(projectNumber)')@cloudbuild.gserviceaccount.com

# Allow Cloud Build to deploy to Cloud Run
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/run.admin" \
    --condition=None

# Allow Cloud Build to access secrets
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/secretmanager.secretAccessor" \
    --condition=None

# Allow Cloud Build to push to Artifact Registry
gcloud projects add-iam-policy-binding "${PROJECT_ID}" \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/artifactregistry.writer" \
    --condition=None

# Allow Cloud Build to act as the service account
gcloud iam service-accounts add-iam-policy-binding "${SERVICE_ACCOUNT_EMAIL}" \
    --member="serviceAccount:${CLOUD_BUILD_SA}" \
    --role="roles/iam.serviceAccountUser" \
    --project="${PROJECT_ID}"

# Create secrets (placeholders - actual values should be set manually)
echo "Creating secret placeholders..."
echo "PLACEHOLDER" | gcloud secrets create ruvector-api-key \
    --data-file=- \
    --replication-policy="automatic" \
    --project="${PROJECT_ID}" 2>/dev/null || echo "Secret ruvector-api-key already exists"

for ENV in dev staging prod; do
    echo "https://ruvector-service-${ENV}.example.com" | gcloud secrets create "ruvector-service-url-${ENV}" \
        --data-file=- \
        --replication-policy="automatic" \
        --project="${PROJECT_ID}" 2>/dev/null || echo "Secret ruvector-service-url-${ENV} already exists"

    echo "https://llm-observatory-${ENV}.example.com" | gcloud secrets create "observatory-endpoint-${ENV}" \
        --data-file=- \
        --replication-policy="automatic" \
        --project="${PROJECT_ID}" 2>/dev/null || echo "Secret observatory-endpoint-${ENV} already exists"
done

echo ""
echo "============================================================================"
echo "IAM Setup Complete"
echo "============================================================================"
echo ""
echo "Next steps:"
echo "1. Update secrets with actual values:"
echo "   gcloud secrets versions add ruvector-api-key --data-file=/path/to/key"
echo "   gcloud secrets versions add ruvector-service-url-dev --data-file=-"
echo "   gcloud secrets versions add observatory-endpoint-dev --data-file=-"
echo ""
echo "2. Deploy the service:"
echo "   gcloud builds submit --config=deploy/gcloud/cloudbuild.yaml \\"
echo "       --substitutions=_ENV=dev,_REGION=${REGION}"
echo ""
echo "Service Account: ${SERVICE_ACCOUNT_EMAIL}"
