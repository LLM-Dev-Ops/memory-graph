//! Decision Memory Agent CLI commands
//!
//! This module provides CLI commands for interacting with the Decision Memory Agent:
//! - `decision capture` - Capture a new decision memory
//! - `decision inspect` - Query and inspect decision events
//! - `decision retrieve` - Retrieve a specific decision event
//! - `decision replay` - Replay a decision capture with modifications
//!
//! # Contract Compliance
//! - Exposes CLI-invokable endpoints (inspect / retrieve / replay)
//! - Returns deterministic, machine-readable output
//! - All persistence via ruvector-service client calls

use super::CommandContext;
use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Decision Memory Agent configuration
#[derive(Debug, Clone)]
pub struct DecisionAgentConfig {
    /// Base URL for the Decision Memory Agent service
    pub base_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for DecisionAgentConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("DECISION_MEMORY_AGENT_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            timeout_secs: 30,
        }
    }
}

/// Decision capture input for CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionCaptureInput {
    /// Decision identifier
    pub decision_id: Option<Uuid>,
    /// Decision type/category
    pub decision_type: String,
    /// Session ID
    pub session_id: Uuid,
    /// Predecessor decision ID (for chaining)
    pub predecessor_id: Option<Uuid>,
    /// Agent ID that made the decision
    pub agent_id: Option<String>,
    /// Model used
    pub model_id: Option<String>,
    /// Temperature setting
    pub temperature: Option<f64>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Outcome result type
    pub outcome_result: Option<String>,
    /// Artifact content hashes (for pre-stored artifacts)
    pub artifact_hashes: Vec<String>,
}

/// Decision query filters for CLI
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionQueryFilters {
    /// Filter by session ID
    pub session_id: Option<Uuid>,
    /// Filter by decision ID
    pub decision_id: Option<Uuid>,
    /// Filter by start timestamp
    pub from_timestamp: Option<String>,
    /// Filter by end timestamp
    pub to_timestamp: Option<String>,
    /// Limit results
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Decision event output from the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEventOutput {
    pub agent_id: String,
    pub agent_version: String,
    pub decision_type: String,
    pub execution_ref: Uuid,
    pub decision_id: Uuid,
    pub session_id: Uuid,
    pub nodes_created: usize,
    pub edges_created: usize,
    pub artifacts_stored: usize,
    pub confidence: f64,
    pub capture_timestamp: String,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: Option<T>,
    pub execution_ref: Option<Uuid>,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

/// Decision Memory Agent CLI client
pub struct DecisionAgentClient {
    config: DecisionAgentConfig,
    client: Client,
}

impl DecisionAgentClient {
    /// Create a new client
    pub fn new(config: DecisionAgentConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { config, client })
    }

    /// Create a client from environment
    pub fn from_env() -> Result<Self> {
        Self::new(DecisionAgentConfig::default())
    }

    /// Capture a decision
    pub async fn capture(&self, input: &DecisionCaptureInput) -> Result<DecisionEventOutput> {
        let url = format!("{}/api/v1/capture", self.config.base_url);

        // Build the request payload matching the agent contract
        let decision_id = input.decision_id.unwrap_or_else(Uuid::new_v4);
        let payload = serde_json::json!({
            "input": {
                "decision_id": decision_id,
                "decision_type": input.decision_type,
                "context": {
                    "session_id": input.session_id,
                    "agent_id": input.agent_id,
                    "predecessor_decision_id": input.predecessor_id,
                    "model_id": input.model_id,
                    "temperature": input.temperature,
                },
                "reasoning_artifacts": input.artifact_hashes.iter().enumerate().map(|(i, hash)| {
                    serde_json::json!({
                        "artifact_id": Uuid::new_v4(),
                        "artifact_type": "context_snapshot",
                        "content_hash": hash,
                        "created_at": Utc::now().to_rfc3339(),
                    })
                }).collect::<Vec<_>>(),
                "outcome": input.outcome_result.as_ref().map(|result| {
                    serde_json::json!({
                        "outcome_id": Uuid::new_v4(),
                        "decision_ref": decision_id,
                        "result_type": result,
                        "recorded_at": Utc::now().to_rfc3339(),
                    })
                }),
                "tags": input.tags,
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send capture request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Capture failed: {}", error_text);
        }

        let api_response: ApiResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error_code) = api_response.error_code {
            anyhow::bail!(
                "Agent error [{}]: {}",
                error_code,
                api_response.message.unwrap_or_default()
            );
        }

        let data = api_response.data.context("No data in response")?;

        Ok(DecisionEventOutput {
            agent_id: "decision-memory-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            decision_type: input.decision_type.clone(),
            execution_ref: api_response.execution_ref.unwrap_or_else(Uuid::new_v4),
            decision_id,
            session_id: input.session_id,
            nodes_created: data
                .get("nodes_created")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
            edges_created: data
                .get("edges_created")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
            artifacts_stored: data
                .get("artifacts_stored")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            confidence: data
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            capture_timestamp: data
                .get("capture_timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    /// Retrieve a decision event
    pub async fn retrieve(&self, execution_ref: Uuid) -> Result<serde_json::Value> {
        let url = format!("{}/api/v1/retrieve", self.config.base_url);

        let payload = serde_json::json!({
            "execution_ref": execution_ref,
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send retrieve request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Retrieve failed: {}", error_text);
        }

        let api_response: ApiResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error_code) = api_response.error_code {
            anyhow::bail!(
                "Agent error [{}]: {}",
                error_code,
                api_response.message.unwrap_or_default()
            );
        }

        api_response.data.context("No data in response")
    }

    /// Inspect (query) decision events
    pub async fn inspect(&self, filters: &DecisionQueryFilters) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/v1/inspect", self.config.base_url);

        let payload = serde_json::json!({
            "query": {
                "session_id": filters.session_id,
                "decision_id": filters.decision_id,
                "from_timestamp": filters.from_timestamp,
                "to_timestamp": filters.to_timestamp,
                "limit": filters.limit,
                "offset": filters.offset,
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send inspect request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Inspect failed: {}", error_text);
        }

        let api_response: ApiResponse<Vec<serde_json::Value>> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error_code) = api_response.error_code {
            anyhow::bail!(
                "Agent error [{}]: {}",
                error_code,
                api_response.message.unwrap_or_default()
            );
        }

        Ok(api_response.data.unwrap_or_default())
    }

    /// Replay a decision capture
    pub async fn replay(
        &self,
        execution_ref: Uuid,
        modifications: Option<serde_json::Value>,
    ) -> Result<DecisionEventOutput> {
        let url = format!("{}/api/v1/replay", self.config.base_url);

        let payload = serde_json::json!({
            "execution_ref": execution_ref,
            "modifications": modifications,
        });

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send replay request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Replay failed: {}", error_text);
        }

        let api_response: ApiResponse<serde_json::Value> = response
            .json()
            .await
            .context("Failed to parse response")?;

        if let Some(error_code) = api_response.error_code {
            anyhow::bail!(
                "Agent error [{}]: {}",
                error_code,
                api_response.message.unwrap_or_default()
            );
        }

        let data = api_response.data.context("No data in response")?;

        Ok(DecisionEventOutput {
            agent_id: "decision-memory-agent".to_string(),
            agent_version: "1.0.0".to_string(),
            decision_type: "decision_memory_capture".to_string(),
            execution_ref: api_response.execution_ref.unwrap_or_else(Uuid::new_v4),
            decision_id: data
                .get("decision_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4),
            session_id: Uuid::nil(),
            nodes_created: data
                .get("nodes_created")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
            edges_created: data
                .get("edges_created")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
            artifacts_stored: data
                .get("artifacts_stored")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            confidence: data
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0),
            capture_timestamp: data
                .get("capture_timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }
}

/// Handle decision capture command
pub async fn handle_decision_capture(
    ctx: &CommandContext<'_>,
    decision_type: String,
    session_id: String,
    agent_id: Option<String>,
    model_id: Option<String>,
    tags: Vec<String>,
    outcome_result: Option<String>,
) -> Result<()> {
    let session_uuid =
        Uuid::parse_str(&session_id).context("Invalid session ID (must be UUID format)")?;

    let client = DecisionAgentClient::from_env()?;

    let input = DecisionCaptureInput {
        decision_id: None,
        decision_type,
        session_id: session_uuid,
        predecessor_id: None,
        agent_id,
        model_id,
        temperature: None,
        tags,
        outcome_result,
        artifact_hashes: vec![],
    };

    let result = client.capture(&input).await?;

    ctx.format.output(&serde_json::json!({
        "status": "captured",
        "execution_ref": result.execution_ref,
        "decision_id": result.decision_id,
        "session_id": result.session_id,
        "nodes_created": result.nodes_created,
        "edges_created": result.edges_created,
        "artifacts_stored": result.artifacts_stored,
        "confidence": result.confidence,
        "capture_timestamp": result.capture_timestamp,
    }));

    Ok(())
}

/// Handle decision inspect command
pub async fn handle_decision_inspect(
    ctx: &CommandContext<'_>,
    session_id: Option<String>,
    decision_id: Option<String>,
    from_timestamp: Option<String>,
    to_timestamp: Option<String>,
    limit: Option<u32>,
) -> Result<()> {
    let client = DecisionAgentClient::from_env()?;

    let filters = DecisionQueryFilters {
        session_id: session_id.and_then(|s| Uuid::parse_str(&s).ok()),
        decision_id: decision_id.and_then(|s| Uuid::parse_str(&s).ok()),
        from_timestamp,
        to_timestamp,
        limit,
        offset: None,
    };

    let results = client.inspect(&filters).await?;

    ctx.format.output(&serde_json::json!({
        "status": "inspected",
        "count": results.len(),
        "decisions": results,
    }));

    Ok(())
}

/// Handle decision retrieve command
pub async fn handle_decision_retrieve(
    ctx: &CommandContext<'_>,
    execution_ref: String,
) -> Result<()> {
    let execution_uuid =
        Uuid::parse_str(&execution_ref).context("Invalid execution_ref (must be UUID format)")?;

    let client = DecisionAgentClient::from_env()?;

    let result = client.retrieve(execution_uuid).await?;

    ctx.format.output(&serde_json::json!({
        "status": "retrieved",
        "execution_ref": execution_ref,
        "decision": result,
    }));

    Ok(())
}

/// Handle decision replay command
pub async fn handle_decision_replay(
    ctx: &CommandContext<'_>,
    execution_ref: String,
    modifications_json: Option<String>,
) -> Result<()> {
    let execution_uuid =
        Uuid::parse_str(&execution_ref).context("Invalid execution_ref (must be UUID format)")?;

    let modifications = modifications_json
        .map(|json| serde_json::from_str(&json))
        .transpose()
        .context("Invalid modifications JSON")?;

    let client = DecisionAgentClient::from_env()?;

    let result = client.replay(execution_uuid, modifications).await?;

    ctx.format.output(&serde_json::json!({
        "status": "replayed",
        "original_execution_ref": execution_ref,
        "new_execution_ref": result.execution_ref,
        "decision_id": result.decision_id,
        "nodes_created": result.nodes_created,
        "edges_created": result.edges_created,
        "artifacts_stored": result.artifacts_stored,
        "confidence": result.confidence,
    }));

    Ok(())
}
