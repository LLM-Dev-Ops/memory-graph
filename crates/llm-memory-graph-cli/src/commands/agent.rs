//! Agent management commands

use anyhow::{Context, Result};
use colored::Colorize;
use llm_memory_graph_types::{AgentId, AgentNode, AgentConfig, AgentStatus, AgentMetrics};
use uuid::Uuid;

use crate::output::{OutputFormat, TableBuilder};
use super::CommandContext;

/// Handle agent create command
pub async fn handle_agent_create(
    ctx: &CommandContext<'_>,
    name: String,
    _description: Option<String>,
    model: Option<String>,
) -> Result<()> {
    let now = chrono::Utc::now();
    let agent = AgentNode {
        id: AgentId::new(),
        node_id: llm_memory_graph_types::NodeId::new(),
        name: name.clone(),
        role: "assistant".to_string(),
        capabilities: vec![],
        model: model.unwrap_or_else(|| "gpt-4".to_string()),
        created_at: now,
        last_active: now,
        status: AgentStatus::Idle,
        config: AgentConfig {
            temperature: 0.7,
            max_tokens: 2000,
            timeout_seconds: 300,
            max_retries: 3,
            tools_enabled: vec![],
        },
        tags: vec![],
        metrics: AgentMetrics {
            total_prompts: 0,
            successful_tasks: 0,
            failed_tasks: 0,
            average_latency_ms: 0.0,
            total_tokens_used: 0,
        },
    };

    let agent_id = ctx.graph.add_agent(agent.clone()).await
        .context("Failed to create agent")?;

    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!(
                "{} Agent created: {}",
                "✓".green().bold(),
                agent_id.to_string().cyan()
            );
            println!("  Name: {}", name);
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "agent_id": agent_id.to_string(),
                "name": name,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "status": "success",
                "agent_id": agent_id.to_string(),
                "name": name,
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
    }

    Ok(())
}

/// Handle agent get command
pub async fn handle_agent_get(
    ctx: &CommandContext<'_>,
    agent_id_str: &str,
) -> Result<()> {
    let uuid = Uuid::parse_str(agent_id_str)?;
    let agent_id = AgentId::from_uuid(uuid);

    // Get the agent node
    let node_id = llm_memory_graph_types::NodeId::from(*agent_id.as_uuid());
    let node = ctx.graph.get_node(&node_id).await
        .context("Failed to get agent")?;

    match node {
        Some(llm_memory_graph_types::Node::Agent(agent)) => {
            match ctx.format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&agent)?);
                }
                OutputFormat::Yaml => {
                    println!("{}", serde_yaml::to_string(&agent)?);
                }
                OutputFormat::Table => {
                    TableBuilder::new()
                        .header(vec!["Field", "Value"])
                        .row(vec!["ID".to_string(), agent.id.to_string()])
                        .row(vec!["Name".to_string(), agent.name.clone()])
                        .row(vec!["Role".to_string(), agent.role.clone()])
                        .row(vec!["Model".to_string(), agent.model.clone()])
                        .row(vec!["Status".to_string(), format!("{:?}", agent.status)])
                        .row(vec![
                            "Created".to_string(),
                            agent.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        ])
                        .row(vec![
                            "Last Active".to_string(),
                            agent.last_active.format("%Y-%m-%d %H:%M:%S").to_string(),
                        ])
                        .display();
                }
                OutputFormat::Text => {
                    println!("{}", format!("Agent: {}", agent.name).bold().green());
                    println!("{}", "====================".green());
                    println!("{:15} {}", "ID:", agent.id);
                    println!("{:15} {}", "Role:", agent.role);
                    println!("{:15} {}", "Model:", agent.model);
                    println!("{:15} {:?}", "Status:", agent.status);
                    println!("{:15} {}", "Created:", agent.created_at.format("%Y-%m-%d %H:%M:%S"));
                    println!("{:15} {}", "Last Active:", agent.last_active.format("%Y-%m-%d %H:%M:%S"));

                    if !agent.capabilities.is_empty() {
                        println!("\n{}", "Capabilities:".bold());
                        for cap in &agent.capabilities {
                            println!("  - {}", cap);
                        }
                    }
                }
            }
        }
        Some(_) => {
            ctx.format.error(&format!("Node {} is not an agent", agent_id));
            std::process::exit(1);
        }
        None => {
            ctx.format.error(&format!("Agent not found: {}", agent_id));
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle agent list command
pub async fn handle_agent_list(ctx: &CommandContext<'_>) -> Result<()> {
    ctx.format.warning("Agent listing not yet implemented.");
    ctx.format.warning("This requires indexing agents by type, which is not yet available.");
    Ok(())
}

/// Handle agent update command
pub async fn handle_agent_update(
    ctx: &CommandContext<'_>,
    agent_id_str: &str,
    name: Option<String>,
    _description: Option<String>,
    model: Option<String>,
    temperature: Option<f32>,
) -> Result<()> {
    let uuid = Uuid::parse_str(agent_id_str)?;
    let agent_id = AgentId::from_uuid(uuid);

    // Get the current agent
    let node_id = llm_memory_graph_types::NodeId::from(*agent_id.as_uuid());
    let node = ctx.graph.get_node(&node_id).await
        .context("Failed to get agent")?;

    match node {
        Some(llm_memory_graph_types::Node::Agent(mut agent)) => {
            // Update fields
            if let Some(n) = name {
                agent.name = n;
            }
            if let Some(m) = model {
                agent.model = m;
            }
            if let Some(t) = temperature {
                agent.config.temperature = t;
            }
            agent.last_active = chrono::Utc::now();

            ctx.graph.update_agent(agent).await
                .context("Failed to update agent")?;

            ctx.format.success(&format!("Agent {} updated successfully", agent_id));
        }
        Some(_) => {
            ctx.format.error(&format!("Node {} is not an agent", agent_id));
            std::process::exit(1);
        }
        None => {
            ctx.format.error(&format!("Agent not found: {}", agent_id));
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle assign agent to prompt command
pub async fn handle_agent_assign(
    ctx: &CommandContext<'_>,
    agent_id_str: &str,
    prompt_id_str: &str,
) -> Result<()> {
    let agent_uuid = Uuid::parse_str(agent_id_str)?;
    let prompt_uuid = Uuid::parse_str(prompt_id_str)?;

    let agent_node_id = llm_memory_graph_types::NodeId::from_uuid(agent_uuid);
    let prompt_node_id = llm_memory_graph_types::NodeId::from_uuid(prompt_uuid);

    ctx.graph.assign_agent_to_prompt(prompt_node_id, agent_node_id).await
        .context("Failed to assign agent to prompt")?;

    ctx.format.success(&format!(
        "Agent {} assigned to prompt {}",
        agent_id_str, prompt_id_str
    ));

    Ok(())
}
