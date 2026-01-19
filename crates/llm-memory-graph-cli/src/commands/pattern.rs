//! Long-Term Pattern Agent CLI commands
//!
//! This module provides CLI commands for interacting with the Long-Term Pattern Agent:
//! - `pattern analyze` - Analyze historical memory for patterns
//! - `pattern inspect` - Query a specific pattern analysis by execution_ref
//! - `pattern retrieve` - Retrieve pattern analysis results
//! - `pattern replay` - Re-execute a previous pattern analysis
//!
//! # Contract Compliance
//! - Classification: MEMORY ANALYSIS
//! - Decision Type: long_term_pattern_analysis
//! - Exposes CLI-invokable endpoints (inspect / retrieve / replay)
//! - Returns deterministic, machine-readable output
//! - All persistence via ruvector-service client calls

use super::CommandContext;
use anyhow::{anyhow, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::output::OutputFormat;

/// Long-Term Pattern Agent configuration
#[derive(Debug, Clone)]
pub struct PatternAgentConfig {
    /// Agent service URL (defaults to local TypeScript agent)
    pub service_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for PatternAgentConfig {
    fn default() -> Self {
        Self {
            service_url: std::env::var("LONG_TERM_PATTERN_AGENT_URL")
                .unwrap_or_else(|_| "http://localhost:8082".to_string()),
            timeout_secs: 60, // Longer timeout for pattern analysis
        }
    }
}

/// Pattern types supported by the Long-Term Pattern Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    ConversationFlow,
    TopicRecurrence,
    ResponsePattern,
    ToolUsage,
    ErrorPattern,
    SessionBehavior,
    UserInteraction,
    TemporalTrend,
}

impl std::str::FromStr for PatternType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "conversation_flow" | "conversation-flow" | "flow" => Ok(PatternType::ConversationFlow),
            "topic_recurrence" | "topic-recurrence" | "topic" => Ok(PatternType::TopicRecurrence),
            "response_pattern" | "response-pattern" | "response" => Ok(PatternType::ResponsePattern),
            "tool_usage" | "tool-usage" | "tool" => Ok(PatternType::ToolUsage),
            "error_pattern" | "error-pattern" | "error" => Ok(PatternType::ErrorPattern),
            "session_behavior" | "session-behavior" | "session" => Ok(PatternType::SessionBehavior),
            "user_interaction" | "user-interaction" | "user" => Ok(PatternType::UserInteraction),
            "temporal_trend" | "temporal-trend" | "temporal" => Ok(PatternType::TemporalTrend),
            _ => Err(anyhow!(
                "Invalid pattern type: {}. Valid types: conversation_flow, topic_recurrence, \
                 response_pattern, tool_usage, error_pattern, session_behavior, \
                 user_interaction, temporal_trend",
                s
            )),
        }
    }
}

impl std::fmt::Display for PatternType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternType::ConversationFlow => write!(f, "conversation_flow"),
            PatternType::TopicRecurrence => write!(f, "topic_recurrence"),
            PatternType::ResponsePattern => write!(f, "response_pattern"),
            PatternType::ToolUsage => write!(f, "tool_usage"),
            PatternType::ErrorPattern => write!(f, "error_pattern"),
            PatternType::SessionBehavior => write!(f, "session_behavior"),
            PatternType::UserInteraction => write!(f, "user_interaction"),
            PatternType::TemporalTrend => write!(f, "temporal_trend"),
        }
    }
}

/// Time range for pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub from_timestamp: String,
    pub to_timestamp: String,
}

/// Scope filter for pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AnalysisScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Analysis options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_occurrence_threshold: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_confidence_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_patterns: Option<u32>,
    #[serde(default = "default_true")]
    pub include_examples: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_examples_per_pattern: Option<u32>,
    #[serde(default)]
    pub compute_temporal_distribution: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            min_occurrence_threshold: Some(3),
            min_confidence_threshold: Some(0.7),
            max_patterns: Some(20),
            include_examples: true,
            max_examples_per_pattern: Some(3),
            compute_temporal_distribution: false,
        }
    }
}

/// Input structure for the Long-Term Pattern Agent
#[derive(Debug, Serialize)]
pub struct PatternAnalysisInput {
    pub analysis_id: String,
    pub pattern_types: Vec<PatternType>,
    pub time_range: TimeRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<AnalysisScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<AnalysisOptions>,
}

/// Pattern occurrence
#[derive(Debug, Clone, Deserialize)]
pub struct PatternOccurrence {
    pub occurrence_id: String,
    pub session_id: String,
    #[serde(default)]
    pub node_ids: Option<Vec<String>>,
    pub timestamp: String,
    #[serde(default)]
    pub context_snippet: Option<String>,
}

/// Temporal distribution
#[derive(Debug, Clone, Deserialize)]
pub struct TemporalDistribution {
    #[serde(default)]
    pub hourly_distribution: Option<Vec<u32>>,
    #[serde(default)]
    pub daily_distribution: Option<Vec<u32>>,
    pub trend: String,
    #[serde(default)]
    pub trend_coefficient: Option<f64>,
}

/// Detected pattern
#[derive(Debug, Clone, Deserialize)]
pub struct DetectedPattern {
    pub pattern_id: String,
    pub pattern_type: String,
    pub pattern_signature: String,
    #[serde(default)]
    pub description: Option<String>,
    pub occurrence_count: u32,
    pub confidence: f64,
    pub relevance_score: f64,
    pub first_seen: String,
    pub last_seen: String,
    #[serde(default)]
    pub example_occurrences: Option<Vec<PatternOccurrence>>,
    #[serde(default)]
    pub temporal_distribution: Option<TemporalDistribution>,
    #[serde(default)]
    pub related_patterns: Option<Vec<String>>,
}

/// Analysis statistics
#[derive(Debug, Clone, Deserialize)]
pub struct AnalysisStatistics {
    pub sessions_analyzed: u32,
    pub nodes_scanned: u32,
    #[serde(default)]
    pub edges_traversed: Option<u32>,
    pub patterns_found: u32,
    #[serde(default)]
    pub patterns_filtered: Option<u32>,
    #[serde(default)]
    pub analysis_duration_ms: Option<u32>,
}

/// Output structure from the Long-Term Pattern Agent
#[derive(Debug, Deserialize)]
pub struct PatternAnalysisOutput {
    pub analysis_id: String,
    pub patterns: Vec<DetectedPattern>,
    pub statistics: AnalysisStatistics,
    pub time_range_analyzed: TimeRange,
    pub analysis_timestamp: String,
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
        output: PatternAnalysisOutput,
        decision_event: serde_json::Value,
    },
    Error {
        success: bool,
        error: AgentError,
    },
}

/// Handle pattern analyze command
///
/// Analyzes historical memory for recurring patterns.
pub async fn handle_analyze(
    ctx: &CommandContext<'_>,
    pattern_types: Vec<String>,
    from_timestamp: String,
    to_timestamp: String,
    session_ids: Option<Vec<String>>,
    agent_ids: Option<Vec<String>>,
    tags: Option<Vec<String>>,
    min_occurrence: Option<u32>,
    min_confidence: Option<f64>,
    max_patterns: Option<u32>,
    include_temporal: bool,
) -> Result<()> {
    let config = PatternAgentConfig::default();

    // Parse pattern types
    let types: Vec<PatternType> = pattern_types
        .iter()
        .map(|s| s.parse())
        .collect::<Result<Vec<_>>>()?;

    if types.is_empty() {
        return Err(anyhow!("At least one pattern type is required"));
    }

    let scope = if session_ids.is_some() || agent_ids.is_some() || tags.is_some() {
        Some(AnalysisScope {
            session_ids,
            agent_ids,
            user_ids: None,
            tags,
        })
    } else {
        None
    };

    let options = Some(AnalysisOptions {
        min_occurrence_threshold: min_occurrence.or(Some(3)),
        min_confidence_threshold: min_confidence.or(Some(0.7)),
        max_patterns: max_patterns.or(Some(20)),
        include_examples: true,
        max_examples_per_pattern: Some(3),
        compute_temporal_distribution: include_temporal,
    });

    let input = PatternAnalysisInput {
        analysis_id: Uuid::new_v4().to_string(),
        pattern_types: types,
        time_range: TimeRange {
            from_timestamp,
            to_timestamp,
        },
        scope,
        options,
    };

    let response = invoke_agent(&config, &input).await?;

    match response {
        AgentResponse::Success {
            output,
            decision_event,
            ..
        } => match ctx.format {
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
        },
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

/// Handle pattern inspect command
///
/// Retrieves a specific pattern analysis by execution_ref.
pub async fn handle_inspect(ctx: &CommandContext<'_>, execution_ref: String) -> Result<()> {
    let config = PatternAgentConfig::default();

    // Fetch the DecisionEvent from ruvector-service
    let ruvector_url = std::env::var("RUVECTOR_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let url = format!("{}/api/v1/decision-events/{}", ruvector_url, execution_ref);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .send()
        .await
        .context("Failed to connect to ruvector-service")?;

    if !response.status().is_success() {
        ctx.format
            .error(&format!("DecisionEvent not found: {}", execution_ref));
        std::process::exit(1);
    }

    let event: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse DecisionEvent")?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&event)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&event)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            println!(
                "{}",
                format!("Pattern Analysis: {}", execution_ref)
                    .bold()
                    .cyan()
            );
            println!("{}", "=".repeat(60).cyan());

            if let Some(outputs) = event.get("outputs") {
                if let Some(analysis_id) = outputs.get("analysis_id") {
                    println!("Analysis ID: {}", analysis_id);
                }
                if let Some(stats) = outputs.get("statistics") {
                    println!("\n{}", "Statistics:".bold());
                    if let Some(sessions) = stats.get("sessions_analyzed") {
                        println!("  Sessions Analyzed: {}", sessions);
                    }
                    if let Some(nodes) = stats.get("nodes_scanned") {
                        println!("  Nodes Scanned: {}", nodes);
                    }
                    if let Some(patterns) = stats.get("patterns_found") {
                        println!("  Patterns Found: {}", patterns.to_string().green());
                    }
                }
                if let Some(patterns) = outputs.get("patterns").and_then(|p| p.as_array()) {
                    println!("\n{}", format!("Patterns ({}):", patterns.len()).bold());
                    for (i, pattern) in patterns.iter().enumerate() {
                        if let (Some(ptype), Some(confidence)) = (
                            pattern.get("pattern_type"),
                            pattern.get("confidence").and_then(|c| c.as_f64()),
                        ) {
                            println!(
                                "  {}. {} (confidence: {:.2})",
                                i + 1,
                                ptype,
                                confidence
                            );
                        }
                    }
                }
            }

            if let Some(timestamp) = event.get("timestamp") {
                println!("\nTimestamp: {}", timestamp);
            }
            if let Some(confidence) = event.get("confidence").and_then(|c| c.as_f64()) {
                println!("Overall Confidence: {:.3}", confidence);
            }
        }
    }

    Ok(())
}

/// Handle pattern retrieve command
///
/// Retrieves pattern analysis results based on criteria.
pub async fn handle_retrieve(
    ctx: &CommandContext<'_>,
    from_timestamp: Option<String>,
    to_timestamp: Option<String>,
    limit: Option<u32>,
) -> Result<()> {
    let ruvector_url = std::env::var("RUVECTOR_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let mut url = format!(
        "{}/api/v1/decision-events?agent_id=long-term-pattern-agent&decision_type=long_term_pattern_analysis",
        ruvector_url
    );

    if let Some(from) = &from_timestamp {
        url.push_str(&format!("&from_timestamp={}", from));
    }
    if let Some(to) = &to_timestamp {
        url.push_str(&format!("&to_timestamp={}", to));
    }
    if let Some(l) = limit {
        url.push_str(&format!("&limit={}", l));
    }

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .context("Failed to connect to ruvector-service")?;

    if !response.status().is_success() {
        ctx.format.error("Failed to retrieve pattern analyses");
        std::process::exit(1);
    }

    let result: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse response")?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&result)?);
        }
        OutputFormat::Table | OutputFormat::Text => {
            if let Some(events) = result.get("events").and_then(|e| e.as_array()) {
                println!(
                    "{}",
                    format!("Pattern Analyses ({})", events.len()).bold().green()
                );
                println!("{}", "=".repeat(60).green());

                for event in events {
                    if let Some(execution_ref) = event.get("execution_ref") {
                        println!("\nExecution: {}", execution_ref.to_string().cyan());
                    }
                    if let Some(outputs) = event.get("outputs") {
                        if let Some(stats) = outputs.get("statistics") {
                            if let Some(patterns) = stats.get("patterns_found") {
                                println!("  Patterns Found: {}", patterns);
                            }
                        }
                    }
                    if let Some(timestamp) = event.get("timestamp") {
                        println!("  Timestamp: {}", timestamp);
                    }
                }
            } else {
                println!("No pattern analyses found");
            }
        }
    }

    Ok(())
}

/// Handle pattern replay command
///
/// Re-executes a previous pattern analysis.
pub async fn handle_replay(ctx: &CommandContext<'_>, execution_ref: String) -> Result<()> {
    let config = PatternAgentConfig::default();

    // Fetch the original DecisionEvent
    let ruvector_url = std::env::var("RUVECTOR_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());

    let url = format!("{}/api/v1/decision-events/{}", ruvector_url, execution_ref);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .send()
        .await
        .context("Failed to connect to ruvector-service")?;

    if !response.status().is_success() {
        ctx.format.error(&format!(
            "Original DecisionEvent not found for replay: {}",
            execution_ref
        ));
        std::process::exit(1);
    }

    let event: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse DecisionEvent")?;

    // Note: Full replay would require storing original input in DecisionEvent
    match ctx.format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "operation": "replay",
                "original_execution_ref": execution_ref,
                "message": "Replay requires original input stored in DecisionEvent metadata",
                "original_outputs": event.get("outputs"),
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        _ => {
            println!(
                "{}",
                format!("Replay: {}", execution_ref).bold().cyan()
            );
            println!("{}", "=".repeat(60).cyan());
            println!("{}", "Note: Full replay requires original input in metadata".yellow());

            if let Some(outputs) = event.get("outputs") {
                println!("\n{}", "Original Results:".bold());
                if let Some(stats) = outputs.get("statistics") {
                    if let Some(patterns) = stats.get("patterns_found") {
                        println!("  Patterns Found: {}", patterns);
                    }
                    if let Some(sessions) = stats.get("sessions_analyzed") {
                        println!("  Sessions Analyzed: {}", sessions);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Invoke the Long-Term Pattern Agent
async fn invoke_agent(
    config: &PatternAgentConfig,
    input: &PatternAnalysisInput,
) -> Result<AgentResponse> {
    let client = reqwest::Client::new();

    let response = client
        .post(&config.service_url)
        .json(input)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .send()
        .await
        .context("Failed to connect to Long-Term Pattern Agent")?;

    let body = response
        .text()
        .await
        .context("Failed to read response body")?;

    serde_json::from_str(&body).context("Failed to parse agent response")
}

/// Display the output in the appropriate format
fn display_output(format: &OutputFormat, output: &PatternAnalysisOutput) -> Result<()> {
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
                "Long-Term Pattern Analysis".bold().green()
            );
            println!("{}", "=".repeat(60).green());
            println!("Analysis ID: {}", output.analysis_id);
            println!(
                "Time Range: {} to {}",
                output.time_range_analyzed.from_timestamp,
                output.time_range_analyzed.to_timestamp
            );
            println!("Timestamp: {}", output.analysis_timestamp);

            println!("\n{}", "Statistics:".bold());
            println!(
                "  Sessions Analyzed: {}",
                output.statistics.sessions_analyzed.to_string().cyan()
            );
            println!(
                "  Nodes Scanned: {}",
                output.statistics.nodes_scanned.to_string().cyan()
            );
            if let Some(edges) = output.statistics.edges_traversed {
                println!("  Edges Traversed: {}", edges.to_string().cyan());
            }
            println!(
                "  Patterns Found: {}",
                output.statistics.patterns_found.to_string().green()
            );
            if let Some(filtered) = output.statistics.patterns_filtered {
                println!("  Patterns Filtered: {}", filtered);
            }
            if let Some(duration) = output.statistics.analysis_duration_ms {
                println!("  Analysis Duration: {}ms", duration);
            }

            if !output.patterns.is_empty() {
                println!("\n{}", format!("Detected Patterns ({}):", output.patterns.len()).bold());
                println!("{}", "-".repeat(60));

                for (i, pattern) in output.patterns.iter().enumerate() {
                    let confidence_color = if pattern.confidence > 0.8 {
                        format!("{:.3}", pattern.confidence).green()
                    } else if pattern.confidence > 0.5 {
                        format!("{:.3}", pattern.confidence).yellow()
                    } else {
                        format!("{:.3}", pattern.confidence).red()
                    };

                    println!(
                        "\n{}. {} [{}]",
                        (i + 1).to_string().cyan(),
                        pattern.pattern_type.to_uppercase().bold(),
                        pattern.pattern_signature
                    );

                    if let Some(desc) = &pattern.description {
                        println!("   {}", desc);
                    }

                    println!(
                        "   Occurrences: {}  Confidence: {}  Relevance: {:.3}",
                        pattern.occurrence_count,
                        confidence_color,
                        pattern.relevance_score
                    );
                    println!(
                        "   First Seen: {}  Last Seen: {}",
                        pattern.first_seen, pattern.last_seen
                    );

                    if let Some(temporal) = &pattern.temporal_distribution {
                        println!(
                            "   Trend: {} (coefficient: {:.3})",
                            temporal.trend.magenta(),
                            temporal.trend_coefficient.unwrap_or(0.0)
                        );
                    }

                    if let Some(examples) = &pattern.example_occurrences {
                        if !examples.is_empty() {
                            println!("   Examples ({}):", examples.len());
                            for (j, ex) in examples.iter().take(3).enumerate() {
                                println!(
                                    "     {}. Session: {} at {}",
                                    j + 1,
                                    &ex.session_id[..8],
                                    ex.timestamp
                                );
                            }
                        }
                    }
                }
            } else {
                println!("\n{}", "No patterns detected".yellow());
            }
        }
    }

    Ok(())
}
