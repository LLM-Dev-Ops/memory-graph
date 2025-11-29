//! Database statistics command

use anyhow::Result;
use colored::Colorize;

use crate::output::{OutputFormat, TableBuilder};
use super::CommandContext;

/// Handle the stats command
pub async fn handle_stats(ctx: &CommandContext<'_>) -> Result<()> {
    let stats = ctx.graph.stats().await?;

    match ctx.format {
        OutputFormat::Json => {
            let stats_json = serde_json::json!({
                "node_count": stats.node_count,
                "edge_count": stats.edge_count,
                "session_count": stats.session_count,
            });
            println!("{}", serde_json::to_string_pretty(&stats_json)?);
        }
        OutputFormat::Yaml => {
            let stats_yaml = serde_json::json!({
                "node_count": stats.node_count,
                "edge_count": stats.edge_count,
                "session_count": stats.session_count,
            });
            println!("{}", serde_yaml::to_string(&stats_yaml)?);
        }
        OutputFormat::Table => {
            TableBuilder::new()
                .header(vec!["Metric", "Count"])
                .row(vec!["Total Nodes".to_string(), stats.node_count.to_string()])
                .row(vec!["Total Edges".to_string(), stats.edge_count.to_string()])
                .row(vec!["Total Sessions".to_string(), stats.session_count.to_string()])
                .display();
        }
        OutputFormat::Text => {
            println!("{}", "Database Statistics".bold().green());
            println!("{}", "===================".green());
            println!("{:20} {}", "Total Nodes:", stats.node_count.to_string().cyan());
            println!("{:20} {}", "Total Edges:", stats.edge_count.to_string().cyan());
            println!(
                "{:20} {}",
                "Total Sessions:",
                stats.session_count.to_string().cyan()
            );
        }
    }

    Ok(())
}
