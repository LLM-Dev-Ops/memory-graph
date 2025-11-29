//! Template management commands

use anyhow::{Context, Result};
use colored::Colorize;
use llm_memory_graph_types::{PromptTemplate, TemplateId, Version};
use uuid::Uuid;

use crate::output::{OutputFormat, TableBuilder};
use super::CommandContext;

/// Handle template create command
pub async fn handle_template_create(
    ctx: &CommandContext<'_>,
    name: String,
    content: String,
    description: Option<String>,
    _category: Option<String>,
) -> Result<()> {
    let now = chrono::Utc::now();
    let template = PromptTemplate {
        id: TemplateId::new(),
        node_id: llm_memory_graph_types::NodeId::new(),
        name: name.clone(),
        template: content,
        description: description.unwrap_or_default(),
        variables: vec![],
        version: Version::new(1, 0, 0),
        parent_id: None,
        created_at: now,
        updated_at: now,
        author: "cli".to_string(),
        usage_count: 0,
        tags: vec![],
        metadata: std::collections::HashMap::new(),
    };

    let template_id = ctx.graph.create_template(template.clone()).await
        .context("Failed to create template")?;

    match ctx.format {
        OutputFormat::Text | OutputFormat::Table => {
            println!(
                "{} Template created: {}",
                "✓".green().bold(),
                template_id.to_string().cyan()
            );
            println!("  Name: {}", name);
        }
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "success",
                "template_id": template_id.to_string(),
                "name": name,
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "status": "success",
                "template_id": template_id.to_string(),
                "name": name,
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
    }

    Ok(())
}

/// Handle template get command
pub async fn handle_template_get(
    ctx: &CommandContext<'_>,
    template_id_str: &str,
) -> Result<()> {
    let uuid = Uuid::parse_str(template_id_str)?;
    let template_id = TemplateId::from_uuid(uuid);
    let template = ctx.graph.get_template(template_id).await
        .context("Failed to get template")?;

    match ctx.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&template)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&template)?);
        }
        OutputFormat::Table => {
            TableBuilder::new()
                .header(vec!["Field", "Value"])
                .row(vec!["ID".to_string(), template.id.to_string()])
                .row(vec!["Name".to_string(), template.name.clone()])
                .row(vec!["Version".to_string(), template.version.to_string()])
                .row(vec![
                    "Description".to_string(),
                    template.description.clone(),
                ])
                .row(vec!["Variables".to_string(), template.variables.len().to_string()])
                .row(vec![
                    "Created".to_string(),
                    template.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ])
                .display();

            println!("\n{}", "Content:".bold().green());
            println!("{}", template.template);

            if !template.variables.is_empty() {
                println!("\n{}", "Variables:".bold().green());
                for var in &template.variables {
                    println!("  - {}: {}", var.name, var.description);
                }
            }
        }
        OutputFormat::Text => {
            println!("{}", format!("Template: {}", template.name).bold().green());
            println!("{}", "====================".green());
            println!("{:15} {}", "ID:", template.id);
            println!("{:15} {}", "Version:", template.version);
            println!("{:15} {}", "Description:", template.description);
            println!("{:15} {}", "Created:", template.created_at.format("%Y-%m-%d %H:%M:%S"));

            println!("\n{}", "Content:".bold());
            println!("{}", template.template);

            if !template.variables.is_empty() {
                println!("\n{}", "Variables:".bold());
                for var in &template.variables {
                    println!("  - {}: {}", var.name, var.description);
                }
            }
        }
    }

    Ok(())
}

/// Handle template list command
pub async fn handle_template_list(
    ctx: &CommandContext<'_>,
    category: Option<String>,
) -> Result<()> {
    ctx.format.warning("Template listing not yet implemented.");

    match ctx.format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "status": "error",
                "message": "Template listing not yet implemented",
                "category_filter": category
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "status": "error",
                "message": "Template listing not yet implemented",
                "category_filter": category
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
        _ => {
            println!("Category filter: {:?}", category);
        }
    }

    Ok(())
}

/// Handle template instantiate command
pub async fn handle_template_instantiate(
    ctx: &CommandContext<'_>,
    template_id_str: &str,
    variables: Vec<(String, String)>,
) -> Result<()> {
    let uuid = Uuid::parse_str(template_id_str)?;
    let template_id = TemplateId::from_uuid(uuid);
    let template = ctx.graph.get_template(template_id).await
        .context("Failed to get template")?;

    // Instantiate template by replacing variables
    let mut content = template.template.clone();
    for (key, value) in &variables {
        let placeholder = format!("{{{{{}}}}}", key);
        content = content.replace(&placeholder, value);
    }

    match ctx.format {
        OutputFormat::Json => {
            let result = serde_json::json!({
                "template_id": template_id.to_string(),
                "template_name": template.name,
                "instantiated_content": content,
                "variables": variables.iter().map(|(k, v)| serde_json::json!({k: v})).collect::<Vec<_>>()
            });
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        OutputFormat::Yaml => {
            let result = serde_json::json!({
                "template_id": template_id.to_string(),
                "template_name": template.name,
                "instantiated_content": content,
                "variables": variables.iter().map(|(k, v)| serde_json::json!({k: v})).collect::<Vec<_>>()
            });
            println!("{}", serde_yaml::to_string(&result)?);
        }
        OutputFormat::Text | OutputFormat::Table => {
            println!("{}", format!("Template: {}", template.name).bold().green());
            println!("{}", "====================".green());
            println!("\n{}", "Instantiated Content:".bold());
            println!("{}", content);
        }
    }

    Ok(())
}
