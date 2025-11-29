//! Advanced query command with filtering

use anyhow::Result;
use chrono::{DateTime, Utc};
use colored::Colorize;
use llm_memory_graph_types::{Node, NodeType, SessionId};
use uuid::Uuid;

use crate::output::{OutputFormat, TableBuilder};
use super::CommandContext;

/// Helper to get created_at from any node type
fn get_node_created_at(node: &Node) -> DateTime<Utc> {
    match node {
        Node::Prompt(p) => p.timestamp,
        Node::Response(r) => r.timestamp,
        Node::Session(s) => s.created_at,
        Node::ToolInvocation(t) => t.timestamp,
        Node::Agent(a) => a.created_at,
        Node::Template(t) => t.created_at,
    }
}

/// Helper to get session_id from any node type
fn get_node_session_id(node: &Node) -> Option<SessionId> {
    match node {
        Node::Prompt(p) => Some(p.session_id),
        Node::Response(_) => None, // Response nodes don't have direct session_id
        Node::Session(s) => Some(s.id),
        Node::ToolInvocation(_) => None, // ToolInvocation doesn't have session_id
        Node::Agent(_) => None,
        Node::Template(_) => None,
    }
}

/// Query filter options
pub struct QueryFilters {
    pub session_id: Option<String>,
    pub node_type: Option<String>,
    pub after: Option<String>,
    pub before: Option<String>,
    pub limit: Option<usize>,
}

/// Handle the query command
pub async fn handle_query(ctx: &CommandContext<'_>, filters: QueryFilters) -> Result<()> {
    // Parse session ID if provided
    let session_filter = if let Some(ref session_str) = filters.session_id {
        let uuid = Uuid::parse_str(session_str)?;
        Some(SessionId::from(uuid))
    } else {
        None
    };

    // Parse node type if provided
    let node_type_filter = if let Some(ref type_str) = filters.node_type {
        Some(match type_str.to_lowercase().as_str() {
            "prompt" => NodeType::Prompt,
            "response" => NodeType::Response,
            "agent" => NodeType::Agent,
            "template" => NodeType::Template,
            "tool" | "toolinvocation" => NodeType::ToolInvocation,
            "session" => NodeType::Session,
            _ => {
                ctx.format.error(&format!("Invalid node type: {}. Use: prompt, response, agent, template, tool, session", type_str));
                std::process::exit(1);
            }
        })
    } else {
        None
    };

    // Parse time filters if provided
    let after_filter: Option<DateTime<Utc>> = if let Some(ref after_str) = filters.after {
        Some(DateTime::parse_from_rfc3339(after_str)?.into())
    } else {
        None
    };

    let before_filter: Option<DateTime<Utc>> = if let Some(ref before_str) = filters.before {
        Some(DateTime::parse_from_rfc3339(before_str)?.into())
    } else {
        None
    };

    // If session filter is provided, get nodes from that session
    if let Some(session_id) = session_filter {
        let mut nodes = ctx.graph.get_session_nodes(&session_id).await?;

        // Apply node type filter
        if let Some(node_type) = node_type_filter {
            nodes.retain(|n| n.node_type() == node_type);
        }

        // Apply time filters
        if let Some(after) = after_filter {
            nodes.retain(|n| get_node_created_at(n) >= after);
        }
        if let Some(before) = before_filter {
            nodes.retain(|n| get_node_created_at(n) <= before);
        }

        // Apply limit
        if let Some(limit) = filters.limit {
            nodes.truncate(limit);
        }

        print_node_results(ctx.format, &nodes)?;
    } else {
        // No session filter - need to iterate through all sessions
        let stats = ctx.graph.stats().await?;

        ctx.format.warning(&format!(
            "No session filter provided. This will scan all {} sessions. Consider using --session for better performance.",
            stats.session_count
        ));

        // For now, just show a helpful message
        ctx.format.error("Full database scan not yet implemented. Please specify --session <session-id>");
        std::process::exit(1);
    }

    Ok(())
}

/// Print query results in the appropriate format
fn print_node_results(format: &OutputFormat, nodes: &[Node]) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "count": nodes.len(),
                "nodes": nodes
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "count": nodes.len(),
                "nodes": nodes
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
        OutputFormat::Table => {
            if nodes.is_empty() {
                println!("{}", "No nodes found matching the query".yellow());
                return Ok(());
            }

            let mut builder = TableBuilder::new()
                .header(vec!["ID", "Type", "Created", "Session"]);

            for node in nodes {
                builder = builder.row(vec![
                    node.id().to_string(),
                    format!("{:?}", node.node_type()),
                    get_node_created_at(node).format("%Y-%m-%d %H:%M:%S").to_string(),
                    get_node_session_id(node).map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string()),
                ]);
            }

            builder.display();
            println!("\n{} nodes found", nodes.len().to_string().cyan().bold());
        }
        OutputFormat::Text => {
            if nodes.is_empty() {
                println!("{}", "No nodes found matching the query".yellow());
                return Ok(());
            }

            println!("{}", format!("Query Results: {} nodes", nodes.len()).bold().green());
            println!("{}", "=".repeat(50).green());

            for (i, node) in nodes.iter().enumerate() {
                println!("\n{} {}", format!("[{}]", i + 1).bold(), node.id().to_string().cyan());
                println!("  Type:    {:?}", node.node_type());
                println!("  Created: {}", get_node_created_at(node).format("%Y-%m-%d %H:%M:%S"));
                if let Some(session_id) = get_node_session_id(node) {
                    println!("  Session: {}", session_id);
                }
            }

            println!("\n{}", "=".repeat(50).green());
            println!("Total: {} nodes", nodes.len().to_string().cyan().bold());
        }
    }

    Ok(())
}
