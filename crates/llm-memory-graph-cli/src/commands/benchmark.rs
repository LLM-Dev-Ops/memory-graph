//! Benchmark command implementation

use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use llm_memory_graph::benchmarks::{
    io::{ensure_output_dirs, output_dir, write_results_csv, write_results_json},
    markdown::write_summary,
    run_all_benchmarks,
};

use super::CommandContext;
use crate::output::{OutputFormat, TableBuilder};

/// Handle the benchmark run command
pub async fn handle_benchmark_run(ctx: &CommandContext<'_>) -> Result<()> {
    // Print header
    match ctx.format {
        OutputFormat::Text => {
            println!("{}", "LLM Memory Graph Benchmark Suite".bold().green());
            println!("{}", "=================================".green());
            println!();
        }
        _ => {}
    }

    // Ensure output directories exist
    ensure_output_dirs()?;

    // Run all benchmarks
    match ctx.format {
        OutputFormat::Text => {
            println!("{}", "Running benchmarks...".bold());
            println!();
        }
        _ => {}
    }

    let results = run_all_benchmarks().await?;

    if results.is_empty() {
        eprintln!("{}", "No benchmarks were executed".red());
        return Ok(());
    }

    // Generate timestamp for this benchmark run
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();

    // Write raw results to JSON
    let json_filename = format!("benchmarks_{}.json", timestamp);
    let json_path = write_results_json(&results, &json_filename)?;

    // Write raw results to CSV
    let csv_filename = format!("benchmarks_{}.csv", timestamp);
    let csv_path = write_results_csv(&results, &csv_filename)?;

    // Write summary markdown
    let summary_path = write_summary(&results, output_dir())?;

    // Display results based on output format
    match ctx.format {
        OutputFormat::Json => {
            let summary = serde_json::json!({
                "total_benchmarks": results.len(),
                "timestamp": timestamp,
                "results": results,
                "outputs": {
                    "json": json_path.display().to_string(),
                    "csv": csv_path.display().to_string(),
                    "summary": summary_path.display().to_string(),
                }
            });
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        OutputFormat::Yaml => {
            let summary = serde_json::json!({
                "total_benchmarks": results.len(),
                "timestamp": timestamp,
                "results": results,
                "outputs": {
                    "json": json_path.display().to_string(),
                    "csv": csv_path.display().to_string(),
                    "summary": summary_path.display().to_string(),
                }
            });
            println!("{}", serde_yaml::to_string(&summary)?);
        }
        OutputFormat::Table => {
            let mut table = TableBuilder::new();
            table.header(vec![
                "Benchmark",
                "Mean (ms)",
                "Std Dev (ms)",
                "Throughput (ops/s)",
            ]);

            for result in &results {
                table.row(vec![
                    result.target_id.clone(),
                    format_opt_ms(result.mean_ns()),
                    format_opt_ms(result.std_dev_ns()),
                    format_opt_f64(result.throughput()),
                ]);
            }

            table.display();

            println!();
            println!("{}", "Output Files:".bold());
            println!("  JSON:    {}", json_path.display().to_string().cyan());
            println!("  CSV:     {}", csv_path.display().to_string().cyan());
            println!("  Summary: {}", summary_path.display().to_string().cyan());
        }
        OutputFormat::Text => {
            println!("{}", "Benchmark Results".bold().green());
            println!("{}", "=================".green());
            println!();

            // Group by category if available
            let mut categorized: std::collections::HashMap<String, Vec<_>> =
                std::collections::HashMap::new();

            for result in &results {
                let category = result
                    .metrics
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("uncategorized")
                    .to_string();
                categorized.entry(category).or_default().push(result);
            }

            for (category, category_results) in categorized.iter() {
                println!("{}", category.to_uppercase().bold().yellow());
                println!();

                for result in category_results {
                    println!("  {} {}", "•".cyan(), result.target_id.bold());

                    if let Some(mean_ns) = result.mean_ns() {
                        let mean_ms = mean_ns / 1_000_000.0;
                        println!("    Mean: {:.2} ms", mean_ms);
                    }

                    if let Some(throughput) = result.throughput() {
                        println!("    Throughput: {:.0} ops/s", throughput);
                    }

                    println!();
                }
            }

            println!("{}", "Output Files".bold().green());
            println!("{}", "============".green());
            println!("  JSON:    {}", json_path.display().to_string().cyan());
            println!("  CSV:     {}", csv_path.display().to_string().cyan());
            println!("  Summary: {}", summary_path.display().to_string().cyan());
            println!();
            println!(
                "{} {}",
                "✓".green().bold(),
                format!("Completed {} benchmarks", results.len()).bold()
            );
        }
    }

    Ok(())
}

/// Format optional f64 as milliseconds
fn format_opt_ms(opt: Option<f64>) -> String {
    match opt {
        Some(ns) => format!("{:.2}", ns / 1_000_000.0),
        None => "-".to_string(),
    }
}

/// Format optional f64 value
fn format_opt_f64(opt: Option<f64>) -> String {
    match opt {
        Some(val) => format!("{:.0}", val),
        None => "-".to_string(),
    }
}
