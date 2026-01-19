//! Memory Retrieval Agent CLI commands
//!
//! This module provides CLI wrappers for invoking the Memory Retrieval Agent.
//! The agent is implemented in TypeScript and deployed as a Google Cloud Function.
//!
//! CLI Invocation Shapes:
//! - inspect: Retrieve and display memory subgraph
//! - retrieve: Execute query and return results
//! - replay: Re-execute a previous query by execution_ref

use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::output::OutputFormat;
use super::CommandContext;

/// Memory Retrieval Agent configuration
#[derive(Debug, Clone)]
pub struct MemoryRetrievalConfig {
    /// Agent service URL (defaults to local TypeScript agent)
    pub service_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for MemoryRetrievalConfig {
    fn default() -> Self {
        Self {
            service_url: std::env::var("MEMORY_RETRIEVAL_AGENT_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            timeout_secs: 30,
        }
    }
}

/// Query types supported by the Memory Retrieval Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryType {
    Subgraph,
    Nodes,
    Edges,
    Lineage,
    Context,
    Similarity,
}

impl std::str::FromStr for QueryType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "subgraph" => Ok(QueryType::Subgraph),
            "nodes" => Ok(QueryType::Nodes),
            "edges" => Ok(QueryType::Edges),
            "lineage" => Ok(QueryType::Lineage),
            "context" => Ok(QueryType::Context),
            "similarity" => Ok(QueryType::Similarity),
            _ => Err(anyhow!("Invalid query type: {}", s)),
        }
    }
}

/// Traversal options for graph queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraversalOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_edge_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_node_types: Option<Vec<String>>,
}

/// Input structure for the Memory Retrieval Agent
#[derive(Debug, Serialize)]
pub struct MemoryRetrievalInput {
    pub query_id: String,
    pub query_type: QueryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_nodes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_sessions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traversal_options: Option<TraversalOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_query: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_true")]
    pub include_metadata: bool,
}

fn default_limit() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

/// Retrieved node structure
#[derive(Debug, Deserialize)]
pub struct RetrievedNode {
    pub node_id: String,
    pub node_type: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub relevance_score: Option<f64>,
    #[serde(default)]
    pub depth: Option<u32>,
}

/// Retrieved edge structure
#[derive(Debug, Deserialize)]
pub struct RetrievedEdge {
    pub edge_id: String,
    pub edge_type: String,
    pub from_node_id: String,
    pub to_node_id: String,
    #[serde(default)]
    pub weight: Option<f64>,
}

/// Retrieved subgraph structure
#[derive(Debug, Deserialize)]
pub struct RetrievedSubgraph {
    pub nodes: Vec<RetrievedNode>,
    pub edges: Vec<RetrievedEdge>,
    #[serde(default)]
    pub anchor_node_ids: Option<Vec<String>>,
    #[serde(default)]
    pub truncated: Option<bool>,
}

/// Output structure from the Memory Retrieval Agent
#[derive(Debug, Deserialize)]
pub struct MemoryRetrievalOutput {
    pub query_id: String,
    pub query_type: String,
    pub subgraph: RetrievedSubgraph,
    pub total_nodes_retrieved: u32,
    pub total_edges_retrieved: u32,
    pub retrieval_timestamp: String,
    #[serde(default)]
    pub constraints_applied: Option<Vec<String>>,
}

/// Agent error structure
#[derive(Debug, Deserialize)]
pub struct AgentError {
    pub error_code: String,
    pub message: String,
    #[serde(default)]
    pub details: Option<serde_json::Value>,
    pub execution_ref: String,
    pub timestamp: String,
}

/// Response from the agent
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum AgentResponse {
    Success {
        success: bool,
        output: MemoryRetrievalOutput,
        decision_event: serde_json::Value,
    },
    Error {
        success: bool,
        error: AgentError,
    },
}

/// Handle memory-retrieval inspect command
///
/// Retrieves and displays a memory subgraph based on anchor nodes or sessions.
pub async fn handle_inspect(
    ctx: &CommandContext<'_>,
    anchor_nodes: Option<Vec<String>>,
    anchor_sessions: Option<Vec<String>>,
    query_type: Option<String>,
    max_depth: Option<u32>,
    limit: Option<u32>,
) -> Result<()> {
    let config = MemoryRetrievalConfig::default();

    let query_type = query_type
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(QueryType::Subgraph);

    let input = MemoryRetrievalInput {
        query_id: Uuid::new_v4().to_string(),
        query_type,
        anchor_nodes,
        anchor_sessions,
        traversal_options: max_depth.map(|d| TraversalOptions {
            max_depth: Some(d),
            direction: Some("both".to_string()),
            follow_edge_types: None,
            include_node_types: None,
        }),
        semantic_query: None,
        limit: limit.unwrap_or(100),
        offset: 0,
        include_metadata: true,
    };

    let response = invoke_agent(&config, &input).await?;

    match response {
        AgentResponse::Success { output, .. } => {
            display_output(ctx.format, &output)?;
        }
        AgentResponse::Error { error, .. } => {
            ctx.format.error(&format!(
                "Agent error [{}]: {}",
                error.error_code, error.message
            ));
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle memory-retrieval retrieve command
///
/// Executes a full query with all options.
pub async fn handle_retrieve(
    ctx: &CommandContext<'_>,
    query_type: String,
    anchor_nodes: Option<Vec<String>>,
    anchor_sessions: Option<Vec<String>>,
    semantic_query: Option<String>,
    max_depth: Option<u32>,
    direction: Option<String>,
    edge_types: Option<Vec<String>>,
    node_types: Option<Vec<String>>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<()> {
    let config = MemoryRetrievalConfig::default();

    let query_type: QueryType = query_type.parse()?;

    let traversal_options = if max_depth.is_some() || direction.is_some() ||
                               edge_types.is_some() || node_types.is_some() {
        Some(TraversalOptions {
            max_depth,
            direction,
            follow_edge_types: edge_types,
            include_node_types: node_types,
        })
    } else {
        None
    };

    let input = MemoryRetrievalInput {
        query_id: Uuid::new_v4().to_string(),
        query_type,
        anchor_nodes,
        anchor_sessions,
        traversal_options,
        semantic_query,
        limit: limit.unwrap_or(100),
        offset: offset.unwrap_or(0),
        include_metadata: true,
    };

    let response = invoke_agent(&config, &input).await?;

    match response {
        AgentResponse::Success { output, decision_event, .. } => {
            match ctx.format {
                OutputFormat::Json => {
                    let full_response = serde_json::json!({
                        "success": true,
                        "output": output,
                        "decision_event": decision_event,
                    });
                    println!("{}", serde_json::to_string_pretty(&full_response)?);
                }
                _ => {
                    display_output(ctx.format, &output)?;
                }
            }
        }
        AgentResponse::Error { error, .. } => {
            match ctx.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&error)?);
                }
                _ => {
                    ctx.format.error(&format!(
                        "Agent error [{}]: {}",
                        error.error_code, error.message
                    ));
                }
            }
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle memory-retrieval replay command
///
/// Re-displays results from a previous query by execution_ref.
pub async fn handle_replay(
    ctx: &CommandContext<'_>,
    execution_ref: String,
) -> Result<()> {
    let config = MemoryRetrievalConfig::default();

    // Fetch the previous DecisionEvent
    let url = format!(
        "{}/api/v1/decision-events/{}",
        config.service_url.replace(":8081", ":8080"), // RuVector service
        execution_ref
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .send()
        .await
        .context("Failed to connect to ruvector-service")?;

    if !response.status().is_success() {
        ctx.format.error(&format!(
            "DecisionEvent not found: {}",
            execution_ref
        ));
        std::process::exit(1);
    }

    let event: serde_json::Value = response.json().await
        .context("Failed to parse DecisionEvent")?;

    match ctx.format {
        OutputFormat::Json | OutputFormat::Yaml => {
            if matches!(ctx.format, OutputFormat::Json) {
                println!("{}", serde_json::to_string_pretty(&event)?);
            } else {
                println!("{}", serde_yaml::to_string(&event)?);
            }
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!(
                "{}",
                format!("Previous Execution: {}", execution_ref).bold().cyan()
            );
            println!("{}", "=".repeat(50).cyan());

            if let Some(outputs) = event.get("outputs") {
                if let Some(query_id) = outputs.get("query_id") {
                    println!("Query ID: {}", query_id);
                }
                if let Some(query_type) = outputs.get("query_type") {
                    println!("Query Type: {}", query_type);
                }
                if let Some(nodes) = outputs.get("total_nodes_retrieved") {
                    println!("Nodes Retrieved: {}", nodes);
                }
                if let Some(edges) = outputs.get("total_edges_retrieved") {
                    println!("Edges Retrieved: {}", edges);
                }
            }

            if let Some(timestamp) = event.get("timestamp") {
                println!("\nTimestamp: {}", timestamp);
            }
            if let Some(confidence) = event.get("confidence") {
                println!("Confidence: {}", confidence);
            }
        }
    }

    Ok(())
}

/// Handle memory-retrieval similarity command
///
/// Performs semantic similarity search.
pub async fn handle_similarity(
    ctx: &CommandContext<'_>,
    query: String,
    sessions: Option<Vec<String>>,
    node_types: Option<Vec<String>>,
    limit: Option<u32>,
) -> Result<()> {
    let config = MemoryRetrievalConfig::default();

    let input = MemoryRetrievalInput {
        query_id: Uuid::new_v4().to_string(),
        query_type: QueryType::Similarity,
        anchor_nodes: None,
        anchor_sessions: sessions,
        traversal_options: node_types.map(|nt| TraversalOptions {
            max_depth: Some(1),
            direction: Some("both".to_string()),
            follow_edge_types: None,
            include_node_types: Some(nt),
        }),
        semantic_query: Some(query),
        limit: limit.unwrap_or(10),
        offset: 0,
        include_metadata: true,
    };

    let response = invoke_agent(&config, &input).await?;

    match response {
        AgentResponse::Success { output, .. } => {
            display_similarity_output(ctx.format, &output)?;
        }
        AgentResponse::Error { error, .. } => {
            ctx.format.error(&format!(
                "Agent error [{}]: {}",
                error.error_code, error.message
            ));
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Invoke the Memory Retrieval Agent
async fn invoke_agent(
    config: &MemoryRetrievalConfig,
    input: &MemoryRetrievalInput,
) -> Result<AgentResponse> {
    let client = reqwest::Client::new();

    let response = client
        .post(&config.service_url)
        .json(input)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .send()
        .await
        .context("Failed to connect to Memory Retrieval Agent")?;

    let body = response.text().await
        .context("Failed to read response body")?;

    serde_json::from_str(&body)
        .context("Failed to parse agent response")
}

/// Display the output in the appropriate format
fn display_output(format: &OutputFormat, output: &MemoryRetrievalOutput) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(output)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(output)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!(
                "{}",
                format!("Memory Retrieval: {}", output.query_type).bold().green()
            );
            println!("{}", "=".repeat(50).green());
            println!("Query ID: {}", output.query_id);
            println!("Nodes Retrieved: {}", output.total_nodes_retrieved.to_string().cyan());
            println!("Edges Retrieved: {}", output.total_edges_retrieved.to_string().cyan());
            println!("Timestamp: {}", output.retrieval_timestamp);

            if let Some(constraints) = &output.constraints_applied {
                if !constraints.is_empty() {
                    println!("\n{}", "Constraints Applied:".bold());
                    for c in constraints {
                        println!("  - {}", c);
                    }
                }
            }

            if !output.subgraph.nodes.is_empty() {
                println!("\n{}", "Nodes:".bold());
                for node in &output.subgraph.nodes {
                    let score = node.relevance_score
                        .map(|s| format!(" (score: {:.3})", s))
                        .unwrap_or_default();
                    println!(
                        "  {} [{}]{}",
                        node.node_id.cyan(),
                        node.node_type,
                        score.yellow()
                    );
                }
            }

            if !output.subgraph.edges.is_empty() {
                println!("\n{}", "Edges:".bold());
                for edge in &output.subgraph.edges {
                    println!(
                        "  {} --[{}]--> {}",
                        edge.from_node_id,
                        edge.edge_type.magenta(),
                        edge.to_node_id
                    );
                }
            }

            if output.subgraph.truncated == Some(true) {
                println!("\n{}", "Results were truncated. Use --limit to retrieve more.".yellow());
            }
        }
    }

    Ok(())
}

/// Display similarity search output
fn display_similarity_output(format: &OutputFormat, output: &MemoryRetrievalOutput) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(output)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(output)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!(
                "{}",
                "Similarity Search Results".bold().green()
            );
            println!("{}", "=".repeat(50).green());
            println!("Results: {}", output.total_nodes_retrieved.to_string().cyan());

            if !output.subgraph.nodes.is_empty() {
                println!("\n{:5} {:36} {:15} {:8}", "Rank", "Node ID", "Type", "Score");
                println!("{}", "-".repeat(70));

                for (i, node) in output.subgraph.nodes.iter().enumerate() {
                    let score = node.relevance_score.unwrap_or(0.0);
                    let score_color = if score > 0.8 {
                        score.to_string().green()
                    } else if score > 0.5 {
                        score.to_string().yellow()
                    } else {
                        score.to_string().red()
                    };

                    println!(
                        "{:5} {:36} {:15} {:.3}",
                        (i + 1).to_string().cyan(),
                        node.node_id,
                        node.node_type,
                        score_color
                    );
                }
            }
        }
    }

    Ok(())
}
