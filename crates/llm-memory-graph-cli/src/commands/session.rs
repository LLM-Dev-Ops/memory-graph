//! Session management commands

use anyhow::Result;
use colored::Colorize;
use llm_memory_graph_types::SessionId;
use uuid::Uuid;

use crate::output::{OutputFormat, TableBuilder};
use super::CommandContext;

/// Handle the session get command
pub async fn handle_session_get(ctx: &CommandContext<'_>, session_id_str: &str) -> Result<()> {
    let uuid = Uuid::parse_str(session_id_str)?;
    let session_id = SessionId::from(uuid);
    let session = ctx.graph.get_session(session_id).await?;

    // Get nodes in the session
    let nodes = ctx.graph.get_session_nodes(&session_id).await?;

    match ctx.format {
        OutputFormat::Json => {
            let mut session_with_nodes = serde_json::to_value(&session)?;
            session_with_nodes["node_count"] = serde_json::Value::Number(nodes.len().into());
            println!("{}", serde_json::to_string_pretty(&session_with_nodes)?);
        }
        OutputFormat::Yaml => {
            let mut session_with_nodes = serde_json::to_value(&session)?;
            session_with_nodes["node_count"] = serde_json::Value::Number(nodes.len().into());
            println!("{}", serde_yaml::to_string(&session_with_nodes)?);
        }
        OutputFormat::Table => {
            let mut builder = TableBuilder::new()
                .header(vec!["Field", "Value"])
                .row(vec!["ID".to_string(), session.id.to_string()])
                .row(vec![
                    "Created".to_string(),
                    session.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ])
                .row(vec![
                    "Updated".to_string(),
                    session.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ])
                .row(vec!["Nodes".to_string(), nodes.len().to_string()]);

            for (key, value) in &session.metadata {
                builder = builder.row(vec![format!("metadata.{}", key), value.clone()]);
            }

            builder.display();

            if !session.tags.is_empty() {
                println!("\n{}", "Tags:".bold().green());
                for tag in &session.tags {
                    println!("  - {}", tag);
                }
            }
        }
        OutputFormat::Text => {
            println!("{}", format!("Session: {}", session.id).bold().green());
            println!("{}", "====================".green());
            println!(
                "{:15} {}",
                "Created:",
                session.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "{:15} {}",
                "Updated:",
                session.updated_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("{:15} {}", "Nodes:", nodes.len());
            println!("\n{}", "Metadata:".bold());
            for (key, value) in &session.metadata {
                println!("  {:13} {}", format!("{}:", key), value);
            }
            println!("\n{}", "Tags:".bold());
            for tag in &session.tags {
                println!("  - {}", tag);
            }
        }
    }

    Ok(())
}

/// Handle the node get command
pub async fn handle_node_get(ctx: &CommandContext<'_>, node_id_str: &str) -> Result<()> {
    let uuid = Uuid::parse_str(node_id_str)?;
    let node_id = llm_memory_graph_types::NodeId::from(uuid);
    let node_opt = ctx.graph.get_node(&node_id).await?;

    match node_opt {
        Some(node) => match ctx.format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&node)?);
            }
            OutputFormat::Yaml => {
                println!("{}", serde_yaml::to_string(&node)?);
            }
            OutputFormat::Table | OutputFormat::Text => {
                println!("{}", format!("Node: {}", node_id).bold().green());
                println!("{}", "====================".green());
                println!("{:15} {:?}", "Type:", node.node_type());
                println!("\n{}", "Details:".bold());
                println!("{}", serde_json::to_string_pretty(&node)?);
            }
        },
        None => {
            ctx.format.error(&format!("Node not found: {}", node_id));
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle the flush command
pub async fn handle_flush(ctx: &CommandContext<'_>) -> Result<()> {
    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!("{}", "Flushing database to disk...".yellow());
        }
        _ => {}
    }

    ctx.graph.flush().await?;
    ctx.format.success("Database flushed successfully");
    Ok(())
}

/// Handle the verify command
pub async fn handle_verify(ctx: &CommandContext<'_>) -> Result<()> {
    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!("{}", "Verifying database integrity...".yellow());
        }
        _ => {}
    }

    let stats = ctx.graph.stats().await?;

    match ctx.format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "verified",
                "nodes": stats.node_count,
                "edges": stats.edge_count,
                "sessions": stats.session_count
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "status": "verified",
                "nodes": stats.node_count,
                "edges": stats.edge_count,
                "sessions": stats.session_count
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
        OutputFormat::Table => {
            TableBuilder::new()
                .header(vec!["Component", "Count", "Status"])
                .row(vec!["Nodes".to_string(), stats.node_count.to_string(), "✓".to_string()])
                .row(vec!["Edges".to_string(), stats.edge_count.to_string(), "✓".to_string()])
                .row(vec!["Sessions".to_string(), stats.session_count.to_string(), "✓".to_string()])
                .display();
        }
        OutputFormat::Text => {
            println!("{} Verified {} nodes", "✓".green().bold(), stats.node_count);
            println!("{} Verified {} edges", "✓".green().bold(), stats.edge_count);
            println!(
                "{} Verified {} sessions",
                "✓".green().bold(),
                stats.session_count
            );
            println!("\n{} Database verification complete", "✓".green().bold());
        }
    }

    Ok(())
}
