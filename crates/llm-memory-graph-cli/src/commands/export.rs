//! Export command for sessions and database

use anyhow::{Context, Result};
use colored::Colorize;
use llm_memory_graph_types::SessionId;
use std::path::PathBuf;
use uuid::Uuid;

use crate::output::OutputFormat;
use super::CommandContext;

/// Export format options
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    MessagePack,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ExportFormat::Json),
            "msgpack" | "messagepack" => Ok(ExportFormat::MessagePack),
            _ => Err(format!(
                "Invalid export format: '{}'. Use 'json' or 'msgpack'",
                s
            )),
        }
    }
}

/// Handle session export command
pub async fn handle_export_session(
    ctx: &CommandContext<'_>,
    session_id_str: &str,
    output: &PathBuf,
    export_format: ExportFormat,
) -> Result<()> {
    let uuid = Uuid::parse_str(session_id_str)?;
    let session_id = SessionId::from(uuid);
    let session = ctx.graph.get_session(session_id).await
        .context("Failed to get session")?;

    // Get all nodes in the session
    let nodes = ctx.graph.get_session_nodes(&session_id).await
        .context("Failed to get session nodes")?;

    // Create export data structure
    let export_data = serde_json::json!({
        "session": session,
        "nodes": nodes,
        "node_count": nodes.len(),
        "exported_at": chrono::Utc::now(),
    });

    // Write to file based on format
    match export_format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&export_data)?;
            std::fs::write(output, json)
                .context("Failed to write export file")?;
        }
        ExportFormat::MessagePack => {
            let msgpack = rmp_serde::to_vec(&export_data)?;
            std::fs::write(output, msgpack)
                .context("Failed to write export file")?;
        }
    }

    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!(
                "{} Session exported to: {}",
                "✓".green().bold(),
                output.display().to_string().cyan()
            );
            println!("  {} nodes exported", nodes.len());
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "message": "Session exported",
                "output_file": output.display().to_string(),
                "node_count": nodes.len()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "status": "success",
                "message": "Session exported",
                "output_file": output.display().to_string(),
                "node_count": nodes.len()
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
    }

    Ok(())
}

/// Handle full database export command
pub async fn handle_export_database(
    ctx: &CommandContext<'_>,
    output: &PathBuf,
    export_format: ExportFormat,
) -> Result<()> {
    ctx.format.warning("Full database export is an expensive operation. This may take a while...");

    let stats = ctx.graph.stats().await?;

    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!("Exporting database:");
            println!("  {} sessions", stats.session_count);
            println!("  {} nodes", stats.node_count);
            println!("  {} edges", stats.edge_count);
        }
        _ => {}
    }

    // Create export data structure
    let export_data = serde_json::json!({
        "version": "1.0.0",
        "stats": {
            "node_count": stats.node_count,
            "edge_count": stats.edge_count,
            "session_count": stats.session_count,
        },
        "exported_at": chrono::Utc::now(),
        "note": "Full database export feature not yet fully implemented. Use session export instead."
    });

    // Write to file based on format
    match export_format {
        ExportFormat::Json => {
            let json = serde_json::to_string_pretty(&export_data)?;
            std::fs::write(output, json)
                .context("Failed to write export file")?;
        }
        ExportFormat::MessagePack => {
            let msgpack = rmp_serde::to_vec(&export_data)?;
            std::fs::write(output, msgpack)
                .context("Failed to write export file")?;
        }
    }

    ctx.format.success(&format!(
        "Database metadata exported to: {}",
        output.display()
    ));
    ctx.format.warning("Note: Full data export not yet implemented. Only metadata exported.");

    Ok(())
}
