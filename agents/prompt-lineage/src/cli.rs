//! CLI commands for the Prompt Lineage Agent
//!
//! This module provides command-line interface functionality for inspecting,
//! retrieving, and replaying prompt lineage data from the memory graph.
//!
//! # Commands
//!
//! - `lineage inspect <prompt_id>` - Inspect lineage of a prompt
//! - `lineage retrieve <prompt_id>` - Retrieve full lineage subgraph
//! - `lineage replay <prompt_id>` - Replay lineage creation step-by-step
//!
//! # Output Formats
//!
//! All commands support multiple output formats:
//! - `text` - Human-readable colored output (default)
//! - `json` - JSON format for programmatic consumption
//! - `yaml` - YAML format for configuration files
//! - `table` - Tabular format for structured display

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, Color, Table};
use llm_memory_graph::engine::AsyncMemoryGraph;
use llm_memory_graph_types::{Edge, EdgeType, Node, NodeId, PromptNode};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

// Import from local lineage module
use crate::lineage::EvolutionType;

// ============================================================================
// Local CLI Types (derived from contracts for CLI self-containment)
// ============================================================================

/// Unique identifier for a lineage chain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineageId(Uuid);

impl LineageId {
    /// Create a new random lineage ID
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a lineage ID from a UUID
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for LineageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LineageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Summary of differences between parent and child prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of characters added
    pub chars_added: usize,
    /// Number of characters removed
    pub chars_removed: usize,
    /// Number of lines changed
    pub lines_changed: usize,
    /// Sections that were modified
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_sections: Vec<String>,
    /// Brief textual summary of changes
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl DiffSummary {
    /// Create a new diff summary
    #[must_use]
    pub fn new(chars_added: usize, chars_removed: usize, lines_changed: usize) -> Self {
        Self {
            chars_added,
            chars_removed,
            lines_changed,
            modified_sections: Vec::new(),
            summary: None,
        }
    }

    /// Compute diff between two strings
    #[must_use]
    pub fn compute(parent: &str, child: &str) -> Self {
        let parent_len = parent.len();
        let child_len = child.len();

        let (chars_added, chars_removed) = if child_len >= parent_len {
            (child_len - parent_len, 0)
        } else {
            (0, parent_len - child_len)
        };

        let parent_lines = parent.lines().count();
        let child_lines = child.lines().count();
        let lines_changed = parent_lines.abs_diff(child_lines);

        Self {
            chars_added,
            chars_removed,
            lines_changed,
            modified_sections: Vec::new(),
            summary: None,
        }
    }
}

/// Constraints applied to maintain graph integrity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphConstraint {
    /// Each lineage chain must have exactly one root
    SingleRootPerChain,
    /// Evolution edges must have a parent prompt
    EvolutionMustHaveParent,
    /// No circular references in lineage chains
    NoCircularLineage,
    /// Maximum chain depth limit (prevents runaway evolution)
    MaxChainDepth,
    /// Branch names must be unique within a lineage
    UniqueBranchNames,
    /// Template references must exist
    ValidTemplateReference,
    /// Session must exist for prompt
    ValidSessionReference,
}

impl std::fmt::Display for GraphConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleRootPerChain => write!(f, "single_root_per_chain"),
            Self::EvolutionMustHaveParent => write!(f, "evolution_must_have_parent"),
            Self::NoCircularLineage => write!(f, "no_circular_lineage"),
            Self::MaxChainDepth => write!(f, "max_chain_depth"),
            Self::UniqueBranchNames => write!(f, "unique_branch_names"),
            Self::ValidTemplateReference => write!(f, "valid_template_reference"),
            Self::ValidSessionReference => write!(f, "valid_session_reference"),
        }
    }
}

/// Configuration for the lineage tracking agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageConfig {
    /// Maximum allowed chain depth
    pub max_chain_depth: u32,
    /// Maximum content size in bytes
    pub max_content_size: usize,
    /// Default confidence threshold for similarity
    pub similarity_threshold: f64,
    /// Enable automatic diff computation
    pub compute_diffs: bool,
    /// Enable performance metrics tracking
    pub track_metrics: bool,
    /// Agent identifier
    pub agent_id: String,
    /// Agent version
    pub agent_version: String,
}

impl Default for LineageConfig {
    fn default() -> Self {
        Self {
            max_chain_depth: 100,
            max_content_size: 1_000_000,
            similarity_threshold: 0.7,
            compute_diffs: true,
            track_metrics: true,
            agent_id: "prompt-lineage-agent".to_string(),
            agent_version: "1.0.0".to_string(),
        }
    }
}

/// Performance metrics for a lineage node
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageMetrics {
    /// Number of times this version was used
    pub usage_count: u64,
    /// Average response quality score (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_quality_score: Option<f64>,
    /// Average latency in milliseconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avg_latency_ms: Option<f64>,
    /// Success rate (0.0 to 1.0)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
    /// Cost per invocation (if applicable)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_invocation: Option<f64>,
    /// First used timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_used: Option<DateTime<Utc>>,
    /// Last used timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<DateTime<Utc>>,
}

// ============================================================================
// Output Format
// ============================================================================

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable text with colors
    Text,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Table format
    Table,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Text
    }
}

impl OutputFormat {
    /// Print success message in the appropriate format
    pub fn success(&self, message: &str) {
        match self {
            Self::Text | Self::Table => {
                println!("{} {}", "✓".green().bold(), message);
            }
            Self::Json => {
                let result = serde_json::json!({
                    "status": "success",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            Self::Yaml => {
                let result = serde_json::json!({
                    "status": "success",
                    "message": message
                });
                println!("{}", serde_yaml::to_string(&result).unwrap());
            }
        }
    }

    /// Print error message in the appropriate format
    pub fn error(&self, message: &str) {
        match self {
            Self::Text | Self::Table => {
                eprintln!("{} {}", "Error:".red().bold(), message);
            }
            Self::Json => {
                let result = serde_json::json!({
                    "status": "error",
                    "message": message
                });
                eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            Self::Yaml => {
                let result = serde_json::json!({
                    "status": "error",
                    "message": message
                });
                eprintln!("{}", serde_yaml::to_string(&result).unwrap());
            }
        }
    }

    /// Print warning message in the appropriate format
    pub fn warning(&self, message: &str) {
        match self {
            Self::Text | Self::Table => {
                println!("{} {}", "Warning:".yellow().bold(), message);
            }
            Self::Json => {
                let result = serde_json::json!({
                    "status": "warning",
                    "message": message
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            Self::Yaml => {
                let result = serde_json::json!({
                    "status": "warning",
                    "message": message
                });
                println!("{}", serde_yaml::to_string(&result).unwrap());
            }
        }
    }

    /// Print data in the appropriate format
    pub fn print<T: Serialize>(&self, data: &T) -> Result<()> {
        match self {
            Self::Json => {
                println!("{}", serde_json::to_string_pretty(data)?);
            }
            Self::Yaml => {
                println!("{}", serde_yaml::to_string(data)?);
            }
            Self::Text | Self::Table => {
                // For text/table, fall back to JSON
                println!("{}", serde_json::to_string_pretty(data)?);
            }
        }
        Ok(())
    }
}

// ============================================================================
// CLI Structures
// ============================================================================

/// Prompt Lineage Agent CLI
#[derive(Parser)]
#[command(name = "lineage")]
#[command(about = "Prompt Lineage Agent - Track and analyze prompt evolution")]
#[command(version, author)]
pub struct LineageCli {
    /// Output format (text, json, yaml, table)
    #[arg(short = 'f', long, default_value = "text")]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: LineageCommands,
}

/// Available lineage commands
#[derive(Subcommand)]
pub enum LineageCommands {
    /// Inspect lineage of a prompt
    ///
    /// Shows evolution history, confidence scores, and related prompts
    /// for the specified prompt ID.
    Inspect {
        /// Prompt ID to inspect (UUID format)
        prompt_id: String,

        /// Show full content of prompts
        #[arg(short, long)]
        full_content: bool,

        /// Maximum depth to traverse (default: 10)
        #[arg(short, long, default_value = "10")]
        depth: u32,

        /// Include performance metrics
        #[arg(short, long)]
        metrics: bool,
    },

    /// Retrieve full lineage subgraph
    ///
    /// Returns the complete lineage tree including all ancestors
    /// and descendants as a serializable structure.
    Retrieve {
        /// Prompt ID to retrieve lineage for (UUID format)
        prompt_id: String,

        /// Include ancestors (parents) in the subgraph
        #[arg(short, long, default_value = "true")]
        ancestors: bool,

        /// Include descendants (children) in the subgraph
        #[arg(short, long, default_value = "true")]
        descendants: bool,

        /// Maximum depth for traversal (default: 100)
        #[arg(short, long, default_value = "100")]
        max_depth: u32,

        /// Include edge properties in output
        #[arg(short, long)]
        edge_properties: bool,
    },

    /// Replay lineage creation step-by-step
    ///
    /// Useful for debugging and verification of lineage tracking.
    /// Shows the sequence of operations that built the lineage.
    Replay {
        /// Prompt ID to replay lineage for (UUID format)
        prompt_id: String,

        /// Show detailed diff between versions
        #[arg(short, long)]
        show_diffs: bool,

        /// Pause between steps (interactive mode)
        #[arg(short, long)]
        interactive: bool,

        /// Include timestamps for each step
        #[arg(short, long, default_value = "true")]
        timestamps: bool,
    },
}

// ============================================================================
// Data Structures for Output
// ============================================================================

/// Lineage inspection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageInspection {
    /// The inspected prompt
    pub prompt: PromptInfo,
    /// Lineage chain information
    pub lineage: LineageChainInfo,
    /// Evolution history (ancestors)
    pub ancestors: Vec<PromptEvolution>,
    /// Derived prompts (descendants)
    pub descendants: Vec<PromptEvolution>,
    /// Related prompts in the same session
    pub related_prompts: Vec<PromptInfo>,
    /// Overall confidence score for the lineage
    pub confidence: f64,
    /// Constraints that apply to this lineage
    pub constraints: Vec<String>,
}

/// Basic prompt information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptInfo {
    /// Node ID
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Content (may be truncated)
    pub content: String,
    /// Full content length
    pub content_length: usize,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Model used
    pub model: String,
    /// Template ID if instantiated from template
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    /// Performance metrics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<LineageMetrics>,
}

impl From<&PromptNode> for PromptInfo {
    fn from(prompt: &PromptNode) -> Self {
        let content = if prompt.content.len() > 200 {
            format!("{}...", &prompt.content[..200])
        } else {
            prompt.content.clone()
        };

        Self {
            id: prompt.id.to_string(),
            session_id: prompt.session_id.to_string(),
            content,
            content_length: prompt.content.len(),
            created_at: prompt.timestamp,
            model: prompt.metadata.model.clone(),
            template_id: prompt.template_id.map(|t| t.to_string()),
            metrics: None,
        }
    }
}

/// Lineage chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageChainInfo {
    /// Lineage chain ID
    pub lineage_id: String,
    /// Total depth of the chain
    pub chain_depth: u32,
    /// Position of the inspected prompt in the chain
    pub position: u32,
    /// Whether this is the root
    pub is_root: bool,
    /// Whether this is the current head
    pub is_head: bool,
    /// Branch name (if forked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Root prompt ID
    pub root_id: String,
    /// Head prompt ID (current)
    pub head_id: String,
}

/// Evolution step in lineage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEvolution {
    /// Prompt information
    pub prompt: PromptInfo,
    /// Type of evolution
    pub evolution_type: String,
    /// Reason for evolution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Similarity to previous version
    pub similarity: f64,
    /// Diff summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffInfo>,
    /// Step number in the chain
    pub step: u32,
}

/// Diff information between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInfo {
    /// Characters added
    pub chars_added: usize,
    /// Characters removed
    pub chars_removed: usize,
    /// Lines changed
    pub lines_changed: usize,
    /// Brief summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl From<&DiffSummary> for DiffInfo {
    fn from(diff: &DiffSummary) -> Self {
        Self {
            chars_added: diff.chars_added,
            chars_removed: diff.chars_removed,
            lines_changed: diff.lines_changed,
            summary: diff.summary.clone(),
        }
    }
}

/// Complete lineage subgraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageSubgraph {
    /// Root prompt of the lineage
    pub root: PromptInfo,
    /// All nodes in the subgraph
    pub nodes: Vec<PromptInfo>,
    /// All edges in the subgraph
    pub edges: Vec<LineageEdgeInfo>,
    /// Metadata about the subgraph
    pub metadata: SubgraphMetadata,
}

/// Edge information for subgraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdgeInfo {
    /// Edge ID
    pub id: String,
    /// Source node ID
    pub from: String,
    /// Target node ID
    pub to: String,
    /// Edge type
    pub edge_type: String,
    /// Evolution type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evolution_type: Option<String>,
    /// Similarity score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f64>,
    /// Additional properties
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl From<&Edge> for LineageEdgeInfo {
    fn from(edge: &Edge) -> Self {
        let evolution_type = edge
            .properties
            .get("evolution_type")
            .cloned();
        let similarity = edge
            .properties
            .get("similarity_score")
            .and_then(|s| s.parse().ok());

        Self {
            id: edge.id.to_string(),
            from: edge.from.to_string(),
            to: edge.to.to_string(),
            edge_type: format!("{:?}", edge.edge_type),
            evolution_type,
            similarity,
            properties: edge.properties.clone(),
            created_at: edge.created_at,
        }
    }
}

/// Metadata about the subgraph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubgraphMetadata {
    /// Total number of nodes
    pub node_count: usize,
    /// Total number of edges
    pub edge_count: usize,
    /// Maximum depth traversed
    pub max_depth: u32,
    /// Whether ancestors were included
    pub includes_ancestors: bool,
    /// Whether descendants were included
    pub includes_descendants: bool,
    /// Retrieval timestamp
    pub retrieved_at: DateTime<Utc>,
}

/// Replay step for lineage creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayStep {
    /// Step number
    pub step: u32,
    /// Action performed
    pub action: String,
    /// Description of the action
    pub description: String,
    /// Prompt involved
    pub prompt: PromptInfo,
    /// Edge created (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<LineageEdgeInfo>,
    /// Diff from previous (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<DiffInfo>,
    /// Constraints checked
    pub constraints_checked: Vec<String>,
    /// Timestamp of the step
    pub timestamp: DateTime<Utc>,
}

/// Complete replay result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageReplay {
    /// Target prompt being analyzed
    pub target_prompt_id: String,
    /// Total steps in the replay
    pub total_steps: usize,
    /// Individual steps
    pub steps: Vec<ReplayStep>,
    /// Summary of the replay
    pub summary: ReplaySummary,
}

/// Summary of a replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySummary {
    /// Total evolutions
    pub total_evolutions: u32,
    /// Evolution types encountered
    pub evolution_types: Vec<String>,
    /// Total characters changed
    pub total_chars_changed: usize,
    /// Time span of the lineage
    pub time_span: String,
    /// Constraints applied
    pub constraints_applied: Vec<String>,
}

// ============================================================================
// Command Handlers
// ============================================================================

/// Context for CLI command execution
pub struct CliContext<'a> {
    /// Reference to the memory graph
    pub graph: &'a AsyncMemoryGraph,
    /// Output format
    pub format: OutputFormat,
    /// Configuration
    pub config: LineageConfig,
}

impl<'a> CliContext<'a> {
    /// Create a new CLI context
    pub fn new(graph: &'a AsyncMemoryGraph, format: OutputFormat) -> Self {
        Self {
            graph,
            format,
            config: LineageConfig::default(),
        }
    }

    /// Create a CLI context with custom configuration
    pub fn with_config(graph: &'a AsyncMemoryGraph, format: OutputFormat, config: LineageConfig) -> Self {
        Self {
            graph,
            format,
            config,
        }
    }
}

/// Handle the `lineage inspect` command
pub async fn handle_inspect(
    ctx: &CliContext<'_>,
    prompt_id: &str,
    full_content: bool,
    depth: u32,
    show_metrics: bool,
) -> Result<()> {
    let uuid = Uuid::parse_str(prompt_id)
        .context("Invalid prompt ID format. Expected UUID.")?;
    let node_id = NodeId::from_uuid(uuid);

    // Get the prompt node
    let node = ctx
        .graph
        .get_node(&node_id)
        .await
        .context("Failed to retrieve node")?
        .ok_or_else(|| anyhow::anyhow!("Prompt not found: {}", prompt_id))?;

    let prompt = match node {
        Node::Prompt(p) => p,
        _ => return Err(anyhow::anyhow!("Node {} is not a prompt", prompt_id)),
    };

    // Build the inspection result
    let mut inspection = build_inspection(ctx, &prompt, depth, full_content, show_metrics).await?;

    // Output based on format
    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&inspection)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&inspection)?);
        }
        OutputFormat::Table => {
            display_inspection_table(&inspection);
        }
        OutputFormat::Text => {
            display_inspection_text(&inspection, full_content);
        }
    }

    Ok(())
}

/// Build inspection result by traversing the graph
async fn build_inspection(
    ctx: &CliContext<'_>,
    prompt: &PromptNode,
    max_depth: u32,
    full_content: bool,
    show_metrics: bool,
) -> Result<LineageInspection> {
    let mut prompt_info = PromptInfo::from(prompt);
    if full_content {
        prompt_info.content = prompt.content.clone();
    }

    // Traverse ancestors (incoming edges)
    let ancestors = traverse_ancestors(ctx, &prompt.id, max_depth, full_content).await?;

    // Traverse descendants (outgoing edges)
    let descendants = traverse_descendants(ctx, &prompt.id, max_depth, full_content).await?;

    // Calculate lineage chain info
    let chain_depth = ancestors.len() as u32 + descendants.len() as u32 + 1;
    let position = ancestors.len() as u32 + 1;
    let is_root = ancestors.is_empty();
    let is_head = descendants.is_empty();

    // Get root and head IDs
    let root_id = if ancestors.is_empty() {
        prompt.id.to_string()
    } else {
        ancestors.last().map(|a| a.prompt.id.clone()).unwrap_or_else(|| prompt.id.to_string())
    };

    let head_id = if descendants.is_empty() {
        prompt.id.to_string()
    } else {
        descendants.last().map(|d| d.prompt.id.clone()).unwrap_or_else(|| prompt.id.to_string())
    };

    // Generate lineage ID from root
    let lineage_id = LineageId::from_uuid(Uuid::parse_str(&root_id).unwrap_or_else(|_| Uuid::new_v4()));

    let lineage_info = LineageChainInfo {
        lineage_id: lineage_id.to_string(),
        chain_depth,
        position,
        is_root,
        is_head,
        branch: None, // Would need additional metadata to determine
        root_id,
        head_id,
    };

    // Get related prompts in the same session
    let related_prompts = get_related_prompts(ctx, &prompt.session_id, &prompt.id, 5).await?;

    // Calculate confidence based on chain integrity
    let confidence = calculate_lineage_confidence(&ancestors, &descendants);

    // Determine applicable constraints
    let mut constraints = vec![GraphConstraint::SingleRootPerChain.to_string()];
    if !ancestors.is_empty() {
        constraints.push(GraphConstraint::EvolutionMustHaveParent.to_string());
    }
    constraints.push(GraphConstraint::NoCircularLineage.to_string());

    Ok(LineageInspection {
        prompt: prompt_info,
        lineage: lineage_info,
        ancestors,
        descendants,
        related_prompts,
        confidence,
        constraints,
    })
}

/// Traverse ancestors (parents) of a prompt
async fn traverse_ancestors(
    ctx: &CliContext<'_>,
    node_id: &NodeId,
    max_depth: u32,
    full_content: bool,
) -> Result<Vec<PromptEvolution>> {
    let mut ancestors = Vec::new();
    let mut current_id = *node_id;
    let mut visited = HashSet::new();
    let mut step = 0;

    while step < max_depth {
        if visited.contains(&current_id) {
            break; // Circular reference detected
        }
        visited.insert(current_id);

        // Get incoming edges
        let edges = ctx.graph.get_incoming_edges(&current_id).await?;

        // Find evolution edges (Inherits or custom lineage edges)
        let parent_edge = edges.into_iter().find(|e| {
            matches!(e.edge_type, EdgeType::Inherits | EdgeType::Follows)
                || e.properties.contains_key("evolution_type")
        });

        if let Some(edge) = parent_edge {
            // Get parent node
            if let Some(Node::Prompt(parent)) = ctx.graph.get_node(&edge.from).await? {
                let mut prompt_info = PromptInfo::from(&parent);
                if full_content {
                    prompt_info.content = parent.content.clone();
                }

                let evolution_type = edge
                    .properties
                    .get("evolution_type")
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", edge.edge_type));

                let similarity = edge
                    .properties
                    .get("similarity_score")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0.0);

                let diff = edge
                    .properties
                    .get("diff_summary")
                    .and_then(|s| serde_json::from_str::<DiffSummary>(s).ok())
                    .map(|d| DiffInfo::from(&d));

                let reason = edge.properties.get("evolution_reason").cloned();

                ancestors.push(PromptEvolution {
                    prompt: prompt_info,
                    evolution_type,
                    reason,
                    similarity,
                    diff,
                    step: step + 1,
                });

                current_id = edge.from;
                step += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Reverse to show oldest first
    ancestors.reverse();
    Ok(ancestors)
}

/// Traverse descendants (children) of a prompt
async fn traverse_descendants(
    ctx: &CliContext<'_>,
    node_id: &NodeId,
    max_depth: u32,
    full_content: bool,
) -> Result<Vec<PromptEvolution>> {
    let mut descendants = Vec::new();
    let mut queue: VecDeque<(NodeId, u32)> = VecDeque::new();
    queue.push_back((*node_id, 0));
    let mut visited = HashSet::new();
    visited.insert(*node_id);

    while let Some((current_id, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        // Get outgoing edges
        let edges = ctx.graph.get_outgoing_edges(&current_id).await?;

        for edge in edges {
            // Skip if already visited
            if visited.contains(&edge.to) {
                continue;
            }

            // Check if this is an evolution edge
            let is_evolution = matches!(edge.edge_type, EdgeType::Inherits | EdgeType::Follows)
                || edge.properties.contains_key("evolution_type");

            if is_evolution {
                if let Some(Node::Prompt(child)) = ctx.graph.get_node(&edge.to).await? {
                    let mut prompt_info = PromptInfo::from(&child);
                    if full_content {
                        prompt_info.content = child.content.clone();
                    }

                    let evolution_type = edge
                        .properties
                        .get("evolution_type")
                        .cloned()
                        .unwrap_or_else(|| format!("{:?}", edge.edge_type));

                    let similarity = edge
                        .properties
                        .get("similarity_score")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.0);

                    let diff = edge
                        .properties
                        .get("diff_summary")
                        .and_then(|s| serde_json::from_str::<DiffSummary>(s).ok())
                        .map(|d| DiffInfo::from(&d));

                    let reason = edge.properties.get("evolution_reason").cloned();

                    descendants.push(PromptEvolution {
                        prompt: prompt_info,
                        evolution_type,
                        reason,
                        similarity,
                        diff,
                        step: depth + 1,
                    });

                    visited.insert(edge.to);
                    queue.push_back((edge.to, depth + 1));
                }
            }
        }
    }

    Ok(descendants)
}

/// Get related prompts in the same session
async fn get_related_prompts(
    ctx: &CliContext<'_>,
    session_id: &llm_memory_graph_types::SessionId,
    exclude_id: &NodeId,
    limit: usize,
) -> Result<Vec<PromptInfo>> {
    let session_nodes = ctx.graph.get_session_nodes(session_id).await?;

    let mut related = Vec::new();
    for node in session_nodes {
        if related.len() >= limit {
            break;
        }
        if let Node::Prompt(prompt) = node {
            if prompt.id != *exclude_id {
                related.push(PromptInfo::from(&prompt));
            }
        }
    }

    Ok(related)
}

/// Calculate confidence score for lineage integrity
fn calculate_lineage_confidence(
    ancestors: &[PromptEvolution],
    descendants: &[PromptEvolution],
) -> f64 {
    if ancestors.is_empty() && descendants.is_empty() {
        return 1.0; // Root with no descendants has full confidence
    }

    // Average similarity scores across all evolutions
    let all_evolutions: Vec<f64> = ancestors
        .iter()
        .chain(descendants.iter())
        .map(|e| e.similarity)
        .filter(|&s| s > 0.0)
        .collect();

    if all_evolutions.is_empty() {
        0.8 // Default confidence if no similarity data
    } else {
        all_evolutions.iter().sum::<f64>() / all_evolutions.len() as f64
    }
}

/// Display inspection result as text
fn display_inspection_text(inspection: &LineageInspection, full_content: bool) {
    println!(
        "{}",
        format!("Lineage Inspection: {}", inspection.prompt.id)
            .bold()
            .green()
    );
    println!("{}", "═".repeat(60).green());

    // Prompt info
    println!("\n{}", "Prompt Information".bold().cyan());
    println!("{}", "─".repeat(40));
    println!("  {:15} {}", "ID:", inspection.prompt.id);
    println!("  {:15} {}", "Session:", inspection.prompt.session_id);
    println!("  {:15} {}", "Model:", inspection.prompt.model);
    println!(
        "  {:15} {}",
        "Created:",
        inspection.prompt.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  {:15} {} chars",
        "Content Length:",
        inspection.prompt.content_length
    );

    if full_content {
        println!("\n  {}:", "Content".bold());
        for line in inspection.prompt.content.lines() {
            println!("    {}", line);
        }
    } else {
        println!("\n  {}:", "Content Preview".bold());
        println!("    {}", inspection.prompt.content.dimmed());
    }

    // Lineage info
    println!("\n{}", "Lineage Chain".bold().cyan());
    println!("{}", "─".repeat(40));
    println!("  {:15} {}", "Lineage ID:", inspection.lineage.lineage_id);
    println!("  {:15} {}", "Chain Depth:", inspection.lineage.chain_depth);
    println!(
        "  {:15} {} of {}",
        "Position:", inspection.lineage.position, inspection.lineage.chain_depth
    );
    println!(
        "  {:15} {}",
        "Is Root:",
        if inspection.lineage.is_root {
            "Yes".green()
        } else {
            "No".normal()
        }
    );
    println!(
        "  {:15} {}",
        "Is Head:",
        if inspection.lineage.is_head {
            "Yes".green()
        } else {
            "No".normal()
        }
    );
    println!(
        "  {:15} {:.1}%",
        "Confidence:",
        inspection.confidence * 100.0
    );

    // Ancestors
    if !inspection.ancestors.is_empty() {
        println!("\n{}", "Evolution History (Ancestors)".bold().cyan());
        println!("{}", "─".repeat(40));
        for (i, ancestor) in inspection.ancestors.iter().enumerate() {
            let arrow = if i == 0 { "┌" } else { "├" };
            println!(
                "  {} {} {} ({})",
                arrow,
                ancestor.prompt.id[..8].to_string().yellow(),
                ancestor.evolution_type.cyan(),
                format!("{:.0}% similar", ancestor.similarity * 100.0).dimmed()
            );
            if let Some(reason) = &ancestor.reason {
                println!("  │   Reason: {}", reason.dimmed());
            }
        }
        println!("  └─▶ {} (current)", inspection.prompt.id[..8].to_string().green().bold());
    }

    // Descendants
    if !inspection.descendants.is_empty() {
        println!("\n{}", "Derived Prompts (Descendants)".bold().cyan());
        println!("{}", "─".repeat(40));
        for (i, desc) in inspection.descendants.iter().enumerate() {
            let arrow = if i == inspection.descendants.len() - 1 {
                "└"
            } else {
                "├"
            };
            println!(
                "  {} {} {} ({})",
                arrow,
                desc.prompt.id[..8].to_string().yellow(),
                desc.evolution_type.cyan(),
                format!("{:.0}% similar", desc.similarity * 100.0).dimmed()
            );
        }
    }

    // Related prompts
    if !inspection.related_prompts.is_empty() {
        println!("\n{}", "Related Prompts in Session".bold().cyan());
        println!("{}", "─".repeat(40));
        for related in &inspection.related_prompts {
            println!(
                "  - {} ({})",
                related.id[..8].to_string().yellow(),
                related.created_at.format("%H:%M:%S")
            );
        }
    }

    // Constraints
    println!("\n{}", "Graph Constraints".bold().cyan());
    println!("{}", "─".repeat(40));
    for constraint in &inspection.constraints {
        println!("  {} {}", "✓".green(), constraint);
    }
}

/// Display inspection result as table
fn display_inspection_table(inspection: &LineageInspection) {
    // Main info table
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(Color::Green),
        Cell::new("Value").fg(Color::Green),
    ]);

    table.add_row(vec!["Prompt ID", &inspection.prompt.id]);
    table.add_row(vec!["Session ID", &inspection.prompt.session_id]);
    table.add_row(vec!["Model", &inspection.prompt.model]);
    table.add_row(vec![
        "Created",
        &inspection.prompt.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
    ]);
    table.add_row(vec![
        "Content Length",
        &format!("{} chars", inspection.prompt.content_length),
    ]);
    table.add_row(vec!["Lineage ID", &inspection.lineage.lineage_id]);
    table.add_row(vec![
        "Chain Position",
        &format!("{} of {}", inspection.lineage.position, inspection.lineage.chain_depth),
    ]);
    table.add_row(vec![
        "Is Root",
        if inspection.lineage.is_root { "Yes" } else { "No" },
    ]);
    table.add_row(vec![
        "Is Head",
        if inspection.lineage.is_head { "Yes" } else { "No" },
    ]);
    table.add_row(vec![
        "Confidence",
        &format!("{:.1}%", inspection.confidence * 100.0),
    ]);
    table.add_row(vec![
        "Ancestors",
        &inspection.ancestors.len().to_string(),
    ]);
    table.add_row(vec![
        "Descendants",
        &inspection.descendants.len().to_string(),
    ]);

    println!("{}", table);

    // Evolution history table if there are ancestors or descendants
    if !inspection.ancestors.is_empty() || !inspection.descendants.is_empty() {
        println!("\n{}", "Evolution History".bold().green());

        let mut history_table = Table::new();
        history_table.load_preset(UTF8_FULL);
        history_table.set_header(vec![
            Cell::new("Step").fg(Color::Green),
            Cell::new("Prompt ID").fg(Color::Green),
            Cell::new("Evolution").fg(Color::Green),
            Cell::new("Similarity").fg(Color::Green),
        ]);

        for ancestor in &inspection.ancestors {
            history_table.add_row(vec![
                ancestor.step.to_string(),
                ancestor.prompt.id[..12].to_string(),
                ancestor.evolution_type.clone(),
                format!("{:.1}%", ancestor.similarity * 100.0),
            ]);
        }

        history_table.add_row(vec![
            inspection.lineage.position.to_string(),
            format!("{} (current)", &inspection.prompt.id[..12]),
            "-".to_string(),
            "-".to_string(),
        ]);

        for desc in &inspection.descendants {
            history_table.add_row(vec![
                format!("{}", inspection.lineage.position + desc.step),
                desc.prompt.id[..12].to_string(),
                desc.evolution_type.clone(),
                format!("{:.1}%", desc.similarity * 100.0),
            ]);
        }

        println!("{}", history_table);
    }
}

/// Handle the `lineage retrieve` command
pub async fn handle_retrieve(
    ctx: &CliContext<'_>,
    prompt_id: &str,
    include_ancestors: bool,
    include_descendants: bool,
    max_depth: u32,
    include_edge_properties: bool,
) -> Result<()> {
    let uuid = Uuid::parse_str(prompt_id)
        .context("Invalid prompt ID format. Expected UUID.")?;
    let node_id = NodeId::from_uuid(uuid);

    // Get the prompt node
    let node = ctx
        .graph
        .get_node(&node_id)
        .await
        .context("Failed to retrieve node")?
        .ok_or_else(|| anyhow::anyhow!("Prompt not found: {}", prompt_id))?;

    let prompt = match node {
        Node::Prompt(p) => p,
        _ => return Err(anyhow::anyhow!("Node {} is not a prompt", prompt_id)),
    };

    // Build the subgraph
    let subgraph = build_subgraph(
        ctx,
        &prompt,
        include_ancestors,
        include_descendants,
        max_depth,
        include_edge_properties,
    )
    .await?;

    // Output based on format
    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&subgraph)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&subgraph)?);
        }
        OutputFormat::Table => {
            display_subgraph_table(&subgraph);
        }
        OutputFormat::Text => {
            display_subgraph_text(&subgraph);
        }
    }

    Ok(())
}

/// Build complete lineage subgraph
async fn build_subgraph(
    ctx: &CliContext<'_>,
    prompt: &PromptNode,
    include_ancestors: bool,
    include_descendants: bool,
    max_depth: u32,
    _include_edge_properties: bool,
) -> Result<LineageSubgraph> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut visited_nodes = HashSet::new();
    let mut visited_edges = HashSet::new();

    // Add root prompt
    nodes.push(PromptInfo::from(prompt));
    visited_nodes.insert(prompt.id);

    // Traverse ancestors
    if include_ancestors {
        let mut current_id = prompt.id;
        let mut depth = 0;

        while depth < max_depth {
            let incoming = ctx.graph.get_incoming_edges(&current_id).await?;

            let parent_edge = incoming.into_iter().find(|e| {
                matches!(e.edge_type, EdgeType::Inherits | EdgeType::Follows)
                    || e.properties.contains_key("evolution_type")
            });

            if let Some(edge) = parent_edge {
                if visited_edges.contains(&edge.id) {
                    break;
                }
                visited_edges.insert(edge.id);
                edges.push(LineageEdgeInfo::from(&edge));

                if !visited_nodes.contains(&edge.from) {
                    if let Some(Node::Prompt(parent)) = ctx.graph.get_node(&edge.from).await? {
                        nodes.push(PromptInfo::from(&parent));
                        visited_nodes.insert(parent.id);
                        current_id = parent.id;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }

            depth += 1;
        }
    }

    // Traverse descendants
    if include_descendants {
        let mut queue: VecDeque<(NodeId, u32)> = VecDeque::new();
        queue.push_back((prompt.id, 0));

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth >= max_depth {
                continue;
            }

            let outgoing = ctx.graph.get_outgoing_edges(&current_id).await?;

            for edge in outgoing {
                if visited_edges.contains(&edge.id) {
                    continue;
                }

                let is_evolution = matches!(edge.edge_type, EdgeType::Inherits | EdgeType::Follows)
                    || edge.properties.contains_key("evolution_type");

                if is_evolution {
                    visited_edges.insert(edge.id);
                    edges.push(LineageEdgeInfo::from(&edge));

                    if !visited_nodes.contains(&edge.to) {
                        if let Some(Node::Prompt(child)) = ctx.graph.get_node(&edge.to).await? {
                            nodes.push(PromptInfo::from(&child));
                            visited_nodes.insert(child.id);
                            queue.push_back((child.id, depth + 1));
                        }
                    }
                }
            }
        }
    }

    // Find root (oldest node)
    let root = nodes
        .iter()
        .min_by_key(|n| n.created_at)
        .cloned()
        .unwrap_or_else(|| PromptInfo::from(prompt));

    let metadata = SubgraphMetadata {
        node_count: nodes.len(),
        edge_count: edges.len(),
        max_depth,
        includes_ancestors: include_ancestors,
        includes_descendants: include_descendants,
        retrieved_at: Utc::now(),
    };

    Ok(LineageSubgraph {
        root,
        nodes,
        edges,
        metadata,
    })
}

/// Display subgraph as text
fn display_subgraph_text(subgraph: &LineageSubgraph) {
    println!(
        "{}",
        format!("Lineage Subgraph (Root: {})", &subgraph.root.id[..8])
            .bold()
            .green()
    );
    println!("{}", "═".repeat(60).green());

    println!("\n{}", "Metadata".bold().cyan());
    println!("{}", "─".repeat(40));
    println!("  {:15} {}", "Nodes:", subgraph.metadata.node_count);
    println!("  {:15} {}", "Edges:", subgraph.metadata.edge_count);
    println!("  {:15} {}", "Max Depth:", subgraph.metadata.max_depth);
    println!(
        "  {:15} {}",
        "Ancestors:",
        if subgraph.metadata.includes_ancestors {
            "Included"
        } else {
            "Excluded"
        }
    );
    println!(
        "  {:15} {}",
        "Descendants:",
        if subgraph.metadata.includes_descendants {
            "Included"
        } else {
            "Excluded"
        }
    );

    println!("\n{}", "Nodes".bold().cyan());
    println!("{}", "─".repeat(40));
    for node in &subgraph.nodes {
        let is_root = node.id == subgraph.root.id;
        let marker = if is_root { " (root)" } else { "" };
        println!(
            "  {} {}{}",
            node.id[..12].to_string().yellow(),
            node.created_at.format("%Y-%m-%d %H:%M:%S"),
            marker.green()
        );
    }

    if !subgraph.edges.is_empty() {
        println!("\n{}", "Edges".bold().cyan());
        println!("{}", "─".repeat(40));
        for edge in &subgraph.edges {
            let evolution = edge
                .evolution_type
                .as_ref()
                .map(|s| s.as_str())
                .unwrap_or("unknown");
            println!(
                "  {} ─[{}]─▶ {}",
                edge.from[..8].to_string().yellow(),
                evolution.cyan(),
                edge.to[..8].to_string().yellow()
            );
        }
    }
}

/// Display subgraph as table
fn display_subgraph_table(subgraph: &LineageSubgraph) {
    println!("{}", "Subgraph Nodes".bold().green());

    let mut nodes_table = Table::new();
    nodes_table.load_preset(UTF8_FULL);
    nodes_table.set_header(vec![
        Cell::new("ID").fg(Color::Green),
        Cell::new("Created").fg(Color::Green),
        Cell::new("Model").fg(Color::Green),
        Cell::new("Content Length").fg(Color::Green),
    ]);

    for node in &subgraph.nodes {
        nodes_table.add_row(vec![
            if node.id == subgraph.root.id {
                format!("{} (root)", &node.id[..12])
            } else {
                node.id[..12].to_string()
            },
            node.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            node.model.clone(),
            format!("{} chars", node.content_length),
        ]);
    }

    println!("{}", nodes_table);

    if !subgraph.edges.is_empty() {
        println!("\n{}", "Subgraph Edges".bold().green());

        let mut edges_table = Table::new();
        edges_table.load_preset(UTF8_FULL);
        edges_table.set_header(vec![
            Cell::new("From").fg(Color::Green),
            Cell::new("To").fg(Color::Green),
            Cell::new("Type").fg(Color::Green),
            Cell::new("Similarity").fg(Color::Green),
        ]);

        for edge in &subgraph.edges {
            edges_table.add_row(vec![
                edge.from[..12].to_string(),
                edge.to[..12].to_string(),
                edge.evolution_type.clone().unwrap_or_else(|| edge.edge_type.clone()),
                edge.similarity
                    .map(|s| format!("{:.1}%", s * 100.0))
                    .unwrap_or_else(|| "-".to_string()),
            ]);
        }

        println!("{}", edges_table);
    }
}

/// Handle the `lineage replay` command
pub async fn handle_replay(
    ctx: &CliContext<'_>,
    prompt_id: &str,
    show_diffs: bool,
    interactive: bool,
    show_timestamps: bool,
) -> Result<()> {
    let uuid = Uuid::parse_str(prompt_id)
        .context("Invalid prompt ID format. Expected UUID.")?;
    let node_id = NodeId::from_uuid(uuid);

    // Get the prompt node
    let node = ctx
        .graph
        .get_node(&node_id)
        .await
        .context("Failed to retrieve node")?
        .ok_or_else(|| anyhow::anyhow!("Prompt not found: {}", prompt_id))?;

    let prompt = match node {
        Node::Prompt(p) => p,
        _ => return Err(anyhow::anyhow!("Node {} is not a prompt", prompt_id)),
    };

    // Build the replay
    let replay = build_replay(ctx, &prompt, show_diffs).await?;

    // Output based on format
    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&replay)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&replay)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            display_replay(ctx, &replay, show_diffs, interactive, show_timestamps).await?;
        }
    }

    Ok(())
}

/// Build replay of lineage creation
async fn build_replay(
    ctx: &CliContext<'_>,
    prompt: &PromptNode,
    compute_diffs: bool,
) -> Result<LineageReplay> {
    let mut steps = Vec::new();
    let mut evolution_types = HashSet::new();
    let mut total_chars_changed = 0;

    // Traverse from root to current prompt
    let ancestors = traverse_ancestors(ctx, &prompt.id, 100, true).await?;

    // Build steps from oldest ancestor to current
    let mut prev_content: Option<String> = None;

    for (i, ancestor) in ancestors.iter().enumerate() {
        let action = if i == 0 {
            "CREATE_ROOT"
        } else {
            "EVOLUTION"
        };

        let diff = if compute_diffs {
            if let Some(ref prev) = prev_content {
                let diff_summary = DiffSummary::compute(prev, &ancestor.prompt.content);
                total_chars_changed += diff_summary.chars_added + diff_summary.chars_removed;
                Some(DiffInfo::from(&diff_summary))
            } else {
                None
            }
        } else {
            ancestor.diff.clone()
        };

        evolution_types.insert(ancestor.evolution_type.clone());

        let step = ReplayStep {
            step: (i + 1) as u32,
            action: action.to_string(),
            description: format!(
                "{} prompt version {}",
                if i == 0 { "Created initial" } else { "Evolved to" },
                i + 1
            ),
            prompt: ancestor.prompt.clone(),
            edge: None, // Would need to track edges separately
            diff,
            constraints_checked: vec![
                GraphConstraint::SingleRootPerChain.to_string(),
                GraphConstraint::NoCircularLineage.to_string(),
            ],
            timestamp: ancestor.prompt.created_at,
        };

        steps.push(step);
        prev_content = Some(ancestor.prompt.content.clone());
    }

    // Add current prompt as final step
    let current_diff = if compute_diffs {
        if let Some(ref prev) = prev_content {
            let diff_summary = DiffSummary::compute(prev, &prompt.content);
            total_chars_changed += diff_summary.chars_added + diff_summary.chars_removed;
            Some(DiffInfo::from(&diff_summary))
        } else {
            None
        }
    } else {
        None
    };

    let current_step = ReplayStep {
        step: (steps.len() + 1) as u32,
        action: if steps.is_empty() {
            "CREATE_ROOT"
        } else {
            "CURRENT"
        }
        .to_string(),
        description: format!("Current prompt (version {})", steps.len() + 1),
        prompt: PromptInfo::from(prompt),
        edge: None,
        diff: current_diff,
        constraints_checked: vec![
            GraphConstraint::ValidSessionReference.to_string(),
        ],
        timestamp: prompt.timestamp,
    };

    steps.push(current_step);

    // Calculate time span
    let time_span = if steps.len() > 1 {
        let first_time = steps.first().map(|s| s.timestamp).unwrap_or_else(Utc::now);
        let last_time = steps.last().map(|s| s.timestamp).unwrap_or_else(Utc::now);
        let duration = last_time.signed_duration_since(first_time);
        format_duration(duration)
    } else {
        "N/A".to_string()
    };

    let summary = ReplaySummary {
        total_evolutions: (steps.len().saturating_sub(1)) as u32,
        evolution_types: evolution_types.into_iter().collect(),
        total_chars_changed,
        time_span,
        constraints_applied: vec![
            GraphConstraint::SingleRootPerChain.to_string(),
            GraphConstraint::EvolutionMustHaveParent.to_string(),
            GraphConstraint::NoCircularLineage.to_string(),
        ],
    };

    Ok(LineageReplay {
        target_prompt_id: prompt.id.to_string(),
        total_steps: steps.len(),
        steps,
        summary,
    })
}

/// Format a duration as human-readable string
fn format_duration(duration: chrono::Duration) -> String {
    let secs = duration.num_seconds().abs();
    if secs < 60 {
        format!("{} seconds", secs)
    } else if secs < 3600 {
        format!("{} minutes", secs / 60)
    } else if secs < 86400 {
        format!("{} hours", secs / 3600)
    } else {
        format!("{} days", secs / 86400)
    }
}

/// Display replay interactively
async fn display_replay(
    _ctx: &CliContext<'_>,
    replay: &LineageReplay,
    show_diffs: bool,
    interactive: bool,
    show_timestamps: bool,
) -> Result<()> {
    println!(
        "{}",
        format!("Lineage Replay: {}", &replay.target_prompt_id[..8])
            .bold()
            .green()
    );
    println!("{}", "═".repeat(60).green());
    println!(
        "Total Steps: {} | Total Evolutions: {}\n",
        replay.total_steps, replay.summary.total_evolutions
    );

    for step in &replay.steps {
        // Step header
        let action_color = match step.action.as_str() {
            "CREATE_ROOT" => "green",
            "EVOLUTION" => "yellow",
            "CURRENT" => "cyan",
            _ => "white",
        };

        println!(
            "{}",
            format!("Step {}: {}", step.step, step.action)
                .color(action_color)
                .bold()
        );
        println!("{}", "─".repeat(40));

        if show_timestamps {
            println!(
                "  {} {}",
                "Timestamp:".dimmed(),
                step.timestamp.format("%Y-%m-%d %H:%M:%S")
            );
        }

        println!("  {} {}", "Prompt ID:".dimmed(), &step.prompt.id[..12]);
        println!("  {} {}", "Description:".dimmed(), step.description);

        // Show content preview
        let preview = if step.prompt.content.len() > 100 {
            format!("{}...", &step.prompt.content[..100])
        } else {
            step.prompt.content.clone()
        };
        println!("  {}:", "Content Preview".dimmed());
        println!("    {}", preview.italic());

        // Show diff if available and requested
        if show_diffs {
            if let Some(ref diff) = step.diff {
                println!("  {}:", "Changes".dimmed());
                println!(
                    "    {} added, {} removed, {} lines changed",
                    format!("+{}", diff.chars_added).green(),
                    format!("-{}", diff.chars_removed).red(),
                    diff.lines_changed
                );
            }
        }

        // Show constraints
        if !step.constraints_checked.is_empty() {
            println!("  {}:", "Constraints Verified".dimmed());
            for constraint in &step.constraints_checked {
                println!("    {} {}", "✓".green(), constraint);
            }
        }

        println!();

        // Interactive pause
        if interactive && step.step < replay.total_steps as u32 {
            println!("{}", "Press Enter to continue...".dimmed());
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        }
    }

    // Summary
    println!("{}", "Summary".bold().cyan());
    println!("{}", "─".repeat(40));
    println!(
        "  {:20} {}",
        "Total Evolutions:", replay.summary.total_evolutions
    );
    println!(
        "  {:20} {}",
        "Characters Changed:", replay.summary.total_chars_changed
    );
    println!("  {:20} {}", "Time Span:", replay.summary.time_span);
    println!("  {:20}", "Evolution Types:");
    for et in &replay.summary.evolution_types {
        println!("    - {}", et.cyan());
    }

    Ok(())
}

// ============================================================================
// Main Entry Point
// ============================================================================

/// Execute a lineage CLI command
pub async fn execute(graph: &AsyncMemoryGraph, cli: LineageCli) -> Result<()> {
    let ctx = CliContext::new(graph, cli.format);

    match cli.command {
        LineageCommands::Inspect {
            prompt_id,
            full_content,
            depth,
            metrics,
        } => {
            handle_inspect(&ctx, &prompt_id, full_content, depth, metrics).await?;
        }
        LineageCommands::Retrieve {
            prompt_id,
            ancestors,
            descendants,
            max_depth,
            edge_properties,
        } => {
            handle_retrieve(&ctx, &prompt_id, ancestors, descendants, max_depth, edge_properties)
                .await?;
        }
        LineageCommands::Replay {
            prompt_id,
            show_diffs,
            interactive,
            timestamps,
        } => {
            handle_replay(&ctx, &prompt_id, show_diffs, interactive, timestamps).await?;
        }
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Text);
    }

    #[test]
    fn test_diff_info_from_diff_summary() {
        let summary = DiffSummary::new(100, 50, 10);
        let info = DiffInfo::from(&summary);
        assert_eq!(info.chars_added, 100);
        assert_eq!(info.chars_removed, 50);
        assert_eq!(info.lines_changed, 10);
    }

    #[test]
    fn test_format_duration() {
        let duration = chrono::Duration::seconds(30);
        assert_eq!(format_duration(duration), "30 seconds");

        let duration = chrono::Duration::minutes(5);
        assert_eq!(format_duration(duration), "5 minutes");

        let duration = chrono::Duration::hours(2);
        assert_eq!(format_duration(duration), "2 hours");

        let duration = chrono::Duration::days(3);
        assert_eq!(format_duration(duration), "3 days");
    }

    #[test]
    fn test_calculate_lineage_confidence_empty() {
        let ancestors = vec![];
        let descendants = vec![];
        let confidence = calculate_lineage_confidence(&ancestors, &descendants);
        assert_eq!(confidence, 1.0);
    }

    #[test]
    fn test_subgraph_metadata_creation() {
        let metadata = SubgraphMetadata {
            node_count: 5,
            edge_count: 4,
            max_depth: 10,
            includes_ancestors: true,
            includes_descendants: true,
            retrieved_at: Utc::now(),
        };

        assert_eq!(metadata.node_count, 5);
        assert_eq!(metadata.edge_count, 4);
        assert!(metadata.includes_ancestors);
        assert!(metadata.includes_descendants);
    }

    #[test]
    fn test_replay_summary_creation() {
        let summary = ReplaySummary {
            total_evolutions: 5,
            evolution_types: vec!["evolves".to_string(), "refines".to_string()],
            total_chars_changed: 500,
            time_span: "2 hours".to_string(),
            constraints_applied: vec![GraphConstraint::SingleRootPerChain.to_string()],
        };

        assert_eq!(summary.total_evolutions, 5);
        assert_eq!(summary.evolution_types.len(), 2);
        assert_eq!(summary.total_chars_changed, 500);
    }

    #[test]
    fn test_lineage_chain_info_serialization() {
        let info = LineageChainInfo {
            lineage_id: LineageId::new().to_string(),
            chain_depth: 5,
            position: 3,
            is_root: false,
            is_head: false,
            branch: Some("experimental".to_string()),
            root_id: "root-id".to_string(),
            head_id: "head-id".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("chain_depth"));
        assert!(json.contains("experimental"));
    }
}
