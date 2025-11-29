//! Import command for data restoration

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

use crate::output::OutputFormat;
use super::CommandContext;

/// Import format options
#[derive(Debug, Clone)]
pub enum ImportFormat {
    Json,
    MessagePack,
}

impl std::str::FromStr for ImportFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(ImportFormat::Json),
            "msgpack" | "messagepack" => Ok(ImportFormat::MessagePack),
            _ => Err(format!(
                "Invalid import format: '{}'. Use 'json' or 'msgpack'",
                s
            )),
        }
    }
}

/// Handle import command
pub async fn handle_import(
    ctx: &CommandContext<'_>,
    input: &PathBuf,
    import_format: ImportFormat,
    dry_run: bool,
) -> Result<()> {
    // Read file
    let file_data = std::fs::read(input)
        .context("Failed to read import file")?;

    // Parse based on format
    let import_data: serde_json::Value = match import_format {
        ImportFormat::Json => {
            serde_json::from_slice(&file_data)
                .context("Failed to parse JSON import file")?
        }
        ImportFormat::MessagePack => {
            rmp_serde::from_slice(&file_data)
                .context("Failed to parse MessagePack import file")?
        }
    };

    // Check if this is a session export or full database export
    let is_session_export = import_data.get("session").is_some();
    let is_db_export = import_data.get("version").is_some();

    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!("{}", "Import Summary".bold().green());
            println!("{}", "==============".green());

            if is_session_export {
                if let Some(nodes) = import_data.get("nodes").and_then(|n| n.as_array()) {
                    println!("Type:       Session export");
                    println!("Nodes:      {}", nodes.len());
                }
            } else if is_db_export {
                println!("Type:       Database export");
                if let Some(version) = import_data.get("version").and_then(|v| v.as_str()) {
                    println!("Version:    {}", version);
                }
            } else {
                println!("Type:       Unknown format");
            }

            if dry_run {
                println!("\n{}", "DRY RUN - No changes will be made".yellow().bold());
            }
        }
        OutputFormat::Json => {
            let summary = serde_json::json!({
                "type": if is_session_export { "session" } else if is_db_export { "database" } else { "unknown" },
                "dry_run": dry_run,
                "file": input.display().to_string()
            });
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        OutputFormat::Yaml => {
            let summary = serde_json::json!({
                "type": if is_session_export { "session" } else if is_db_export { "database" } else { "unknown" },
                "dry_run": dry_run,
                "file": input.display().to_string()
            });
            println!("{}", serde_yaml::to_string(&summary)?);
        }
    }

    if !dry_run {
        ctx.format.warning("Import functionality not yet fully implemented.");
        ctx.format.warning("Use --dry-run to validate import files.");
    } else {
        ctx.format.success("Dry run completed. File format is valid.");
    }

    Ok(())
}
