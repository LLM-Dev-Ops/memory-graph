//! Command-line interface for LLM Memory Graph management
//!
//! This tool provides comprehensive commands for managing and querying the memory graph database:
//! - Database inspection and statistics
//! - Session and node queries
//! - Advanced filtering and search
//! - Data export/import
//! - Template management
//! - Agent lifecycle management
//! - Server management
//! - Benchmark execution and reporting
//! - Multiple output formats (text, JSON, YAML, table)

mod commands;
mod output;

use anyhow::Result;
use clap::{Parser, Subcommand};
use llm_memory_graph::{engine::AsyncMemoryGraph, Config};
use std::path::PathBuf;

use commands::CommandContext;
use output::OutputFormat;

/// LLM Memory Graph CLI - Enterprise-grade database management and query tool
#[derive(Parser)]
#[command(name = "llm-memory-graph")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the database directory
    #[arg(short, long, default_value = "./data")]
    db_path: PathBuf,

    /// Output format (text, json, yaml, table)
    #[arg(short = 'f', long, default_value = "text")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show database statistics
    Stats,

    /// Session management commands
    #[command(subcommand)]
    Session(SessionCommands),

    /// Node operations
    #[command(subcommand)]
    Node(NodeCommands),

    /// Advanced query with filters
    Query {
        /// Filter by session ID (UUID format)
        #[arg(short, long)]
        session: Option<String>,

        /// Filter by node type (prompt, response, agent, template, tool)
        #[arg(short = 't', long)]
        node_type: Option<String>,

        /// Filter by creation time (after this timestamp, RFC3339 format)
        #[arg(short, long)]
        after: Option<String>,

        /// Filter by creation time (before this timestamp, RFC3339 format)
        #[arg(short, long)]
        before: Option<String>,

        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Export operations
    #[command(subcommand)]
    Export(ExportCommands),

    /// Import operations
    Import {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Import format (json, msgpack)
        #[arg(long, default_value = "json")]
        import_format: commands::import::ImportFormat,

        /// Dry run - validate without importing
        #[arg(long)]
        dry_run: bool,
    },

    /// Template management
    #[command(subcommand)]
    Template(TemplateCommands),

    /// Agent management
    #[command(subcommand)]
    Agent(AgentCommands),

    /// Server management
    #[command(subcommand)]
    Server(ServerCommands),

    /// Benchmark operations
    #[command(subcommand)]
    Benchmark(BenchmarkCommands),

    /// Flush database to disk
    Flush,

    /// Verify database integrity
    Verify,
}

#[derive(Subcommand)]
enum SessionCommands {
    /// Get session details
    Get {
        /// Session ID (UUID format)
        session_id: String,
    },
}

#[derive(Subcommand)]
enum NodeCommands {
    /// Get node details
    Get {
        /// Node ID (UUID format)
        node_id: String,
    },
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export a session
    Session {
        /// Session ID (UUID format)
        session_id: String,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, msgpack)
        #[arg(long, default_value = "json")]
        export_format: commands::export::ExportFormat,
    },

    /// Export entire database
    Database {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Export format (json, msgpack)
        #[arg(long, default_value = "json")]
        export_format: commands::export::ExportFormat,
    },
}

#[derive(Subcommand)]
enum TemplateCommands {
    /// Create a new template
    Create {
        /// Template name
        #[arg(short, long)]
        name: String,

        /// Template content
        #[arg(short, long)]
        content: String,

        /// Template description
        #[arg(short, long)]
        description: Option<String>,

        /// Template category
        #[arg(long)]
        category: Option<String>,
    },

    /// Get template details
    Get {
        /// Template ID (UUID format)
        template_id: String,
    },

    /// List all templates
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Instantiate a template with variables
    Instantiate {
        /// Template ID (UUID format)
        template_id: String,

        /// Variables in key=value format
        #[arg(short = 'v', long)]
        variables: Vec<String>,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    /// Create a new agent
    Create {
        /// Agent name
        #[arg(short, long)]
        name: String,

        /// Agent description
        #[arg(short, long)]
        description: Option<String>,

        /// Model identifier (e.g., gpt-4, claude-3-opus)
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Get agent details
    Get {
        /// Agent ID (UUID format)
        agent_id: String,
    },

    /// List all agents
    List,

    /// Update an agent
    Update {
        /// Agent ID (UUID format)
        agent_id: String,

        /// New name
        #[arg(short, long)]
        name: Option<String>,

        /// New description
        #[arg(short, long)]
        description: Option<String>,

        /// New model
        #[arg(short, long)]
        model: Option<String>,

        /// New temperature
        #[arg(short, long)]
        temperature: Option<f32>,
    },

    /// Assign agent to a prompt
    Assign {
        /// Agent ID (UUID format)
        agent_id: String,

        /// Prompt ID (UUID format)
        prompt_id: String,
    },
}

#[derive(Subcommand)]
enum ServerCommands {
    /// Start the gRPC server
    Start {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Server port
        #[arg(long, default_value = "50051")]
        port: u16,
    },

    /// Check server health
    Health {
        /// Server URL
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        url: String,
    },

    /// Get server metrics
    Metrics {
        /// Server URL
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        url: String,
    },
}

#[derive(Subcommand)]
enum BenchmarkCommands {
    /// Run all benchmarks and generate reports
    Run,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Open database
    let config = Config::new(cli.db_path.to_str().unwrap());
    let graph = AsyncMemoryGraph::open(config).await?;

    // Create command context
    let ctx = CommandContext::new(&graph, &cli.format);

    match cli.command {
        Commands::Stats => commands::stats::handle_stats(&ctx).await?,

        Commands::Session(session_cmd) => match session_cmd {
            SessionCommands::Get { session_id } => {
                commands::session::handle_session_get(&ctx, &session_id).await?
            }
        },

        Commands::Node(node_cmd) => match node_cmd {
            NodeCommands::Get { node_id } => {
                commands::session::handle_node_get(&ctx, &node_id).await?
            }
        },

        Commands::Query {
            session,
            node_type,
            after,
            before,
            limit,
        } => {
            let filters = commands::query::QueryFilters {
                session_id: session,
                node_type,
                after,
                before,
                limit,
            };
            commands::query::handle_query(&ctx, filters).await?;
        }

        Commands::Export(export_cmd) => match export_cmd {
            ExportCommands::Session {
                session_id,
                output,
                export_format,
            } => {
                commands::export::handle_export_session(&ctx, &session_id, &output, export_format)
                    .await?
            }
            ExportCommands::Database {
                output,
                export_format,
            } => commands::export::handle_export_database(&ctx, &output, export_format).await?,
        },

        Commands::Import {
            input,
            import_format,
            dry_run,
        } => commands::import::handle_import(&ctx, &input, import_format, dry_run).await?,

        Commands::Template(template_cmd) => match template_cmd {
            TemplateCommands::Create {
                name,
                content,
                description,
                category,
            } => {
                commands::template::handle_template_create(&ctx, name, content, description, category)
                    .await?
            }
            TemplateCommands::Get { template_id } => {
                commands::template::handle_template_get(&ctx, &template_id).await?
            }
            TemplateCommands::List { category } => {
                commands::template::handle_template_list(&ctx, category).await?
            }
            TemplateCommands::Instantiate {
                template_id,
                variables,
            } => {
                // Parse variables from key=value format
                let parsed_vars: Vec<(String, String)> = variables
                    .iter()
                    .filter_map(|v| {
                        let parts: Vec<&str> = v.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            Some((parts[0].to_string(), parts[1].to_string()))
                        } else {
                            None
                        }
                    })
                    .collect();

                commands::template::handle_template_instantiate(&ctx, &template_id, parsed_vars)
                    .await?
            }
        },

        Commands::Agent(agent_cmd) => match agent_cmd {
            AgentCommands::Create {
                name,
                description,
                model,
            } => commands::agent::handle_agent_create(&ctx, name, description, model).await?,
            AgentCommands::Get { agent_id } => {
                commands::agent::handle_agent_get(&ctx, &agent_id).await?
            }
            AgentCommands::List => commands::agent::handle_agent_list(&ctx).await?,
            AgentCommands::Update {
                agent_id,
                name,
                description,
                model,
                temperature,
            } => {
                commands::agent::handle_agent_update(
                    &ctx,
                    &agent_id,
                    name,
                    description,
                    model,
                    temperature,
                )
                .await?
            }
            AgentCommands::Assign {
                agent_id,
                prompt_id,
            } => commands::agent::handle_agent_assign(&ctx, &agent_id, &prompt_id).await?,
        },

        Commands::Server(server_cmd) => match server_cmd {
            ServerCommands::Start { host, port } => {
                commands::server::handle_server_start(&ctx, host, port).await?
            }
            ServerCommands::Health { url } => {
                commands::server::handle_server_health(&ctx, url).await?
            }
            ServerCommands::Metrics { url } => {
                commands::server::handle_server_metrics(&ctx, url).await?
            }
        },

        Commands::Benchmark(benchmark_cmd) => match benchmark_cmd {
            BenchmarkCommands::Run => commands::benchmark::handle_benchmark_run(&ctx).await?,
        },

        Commands::Flush => commands::session::handle_flush(&ctx).await?,
        Commands::Verify => commands::session::handle_verify(&ctx).await?,
    }

    Ok(())
}
