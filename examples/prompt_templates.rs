//! Example demonstrating PromptTemplate functionality
//!
//! This example shows how to:
//! - Create versioned prompt templates with variables
//! - Validate variables using regex patterns
//! - Instantiate templates with values
//! - Track template usage
//! - Create template inheritance hierarchies
//! - Link prompts to their source templates

use llm_memory_graph::{Config, MemoryGraph, PromptTemplate, VariableSpec, VersionLevel};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM-Memory-Graph: Prompt Templates Example ===\n");

    // Create a new memory graph
    let config = Config::default();
    let graph = MemoryGraph::open(config)?;
    let session = graph.create_session()?;

    // ===== Example 1: Simple Template =====
    println!("1. Creating a simple greeting template...");

    let greeting_var = VariableSpec::new(
        "name".to_string(),
        "String".to_string(),
        true,
        "The name of the person to greet".to_string(),
    );

    let greeting_template = PromptTemplate::new(
        "Simple Greeting".to_string(),
        "Hello, {{name}}! How can I help you today?".to_string(),
        vec![greeting_var],
    )
    .with_description("A simple greeting template for starting conversations".to_string())
    .with_author("System".to_string());

    let greeting_node_id = greeting_template.node_id;
    let _greeting_id = graph.create_template(greeting_template.clone())?;
    println!("   Created template: {} (v{})", greeting_template.name, greeting_template.version);

    // Instantiate the template
    let mut values = HashMap::new();
    values.insert("name".to_string(), "Alice".to_string());
    let prompt_text = greeting_template.instantiate(&values)?;
    println!("   Instantiated: {}", prompt_text);

    let prompt_id = graph.add_prompt(session.id, prompt_text, None)?;
    graph.link_prompt_to_template(prompt_id, greeting_node_id)?;
    println!("   ✓ Template linked to prompt\n");

    // ===== Example 2: Template with Validation =====
    println!("2. Creating an email template with validation...");

    let email_vars = vec![
        VariableSpec::new(
            "recipient_email".to_string(),
            "String".to_string(),
            true,
            "Recipient's email address".to_string(),
        )
        .with_validation(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
        VariableSpec::new(
            "subject".to_string(),
            "String".to_string(),
            true,
            "Email subject line".to_string(),
        ),
        VariableSpec::new(
            "priority".to_string(),
            "String".to_string(),
            false,
            "Email priority level".to_string(),
        )
        .with_default("normal".to_string())
        .with_validation("^(low|normal|high|urgent)$".to_string()),
    ];

    let mut email_template = PromptTemplate::new(
        "Professional Email".to_string(),
        "To: {{recipient_email}}\nPriority: {{priority}}\nSubject: {{subject}}\n\nDear recipient,\n\n[Body content here]\n\nBest regards".to_string(),
        email_vars,
    )
    .with_description("Professional email template with validation".to_string());

    email_template.add_tag("email".to_string());
    email_template.add_tag("professional".to_string());
    email_template.add_metadata("category".to_string(), "communication".to_string());

    let _email_node_id = email_template.node_id;
    graph.create_template(email_template.clone())?;
    println!("   Created template: {}", email_template.name);

    // Valid instantiation (using default priority)
    let mut values = HashMap::new();
    values.insert("recipient_email".to_string(), "john@example.com".to_string());
    values.insert("subject".to_string(), "Meeting Reminder".to_string());

    match email_template.instantiate(&values) {
        Ok(_text) => {
            println!("   ✓ Valid instantiation with defaults:");
            println!("     Priority: normal (default used)");
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    // Invalid instantiation (bad email format)
    let mut invalid_values = HashMap::new();
    invalid_values.insert("recipient_email".to_string(), "not-an-email".to_string());
    invalid_values.insert("subject".to_string(), "Test".to_string());

    match email_template.instantiate(&invalid_values) {
        Ok(_) => println!("   ✗ Should have failed validation!"),
        Err(e) => println!("   ✓ Validation caught invalid email: {}", e),
    }
    println!();

    // ===== Example 3: Template Versioning =====
    println!("3. Demonstrating template versioning...");

    let mut code_template = PromptTemplate::new(
        "Code Review Template".to_string(),
        "Review this code:\n{{code}}".to_string(),
        vec![],
    );

    let _code_node_id = code_template.node_id;
    graph.create_template(code_template.clone())?;
    println!("   Initial version: {}", code_template.version);

    // Bump patch version for minor fix
    code_template.bump_version(VersionLevel::Patch);
    graph.update_template(code_template.clone())?;
    println!("   After patch: {}", code_template.version);

    // Bump minor version for new feature
    code_template.bump_version(VersionLevel::Minor);
    graph.update_template(code_template.clone())?;
    println!("   After minor update: {}", code_template.version);

    // Bump major version for breaking change
    code_template.bump_version(VersionLevel::Major);
    let final_version = code_template.version.clone();
    graph.update_template(code_template)?;
    println!("   After major update: {}", final_version);
    println!();

    // ===== Example 4: Template Inheritance =====
    println!("4. Creating template inheritance hierarchy...");

    // Base template
    let base_vars = vec![VariableSpec::new(
        "topic".to_string(),
        "String".to_string(),
        true,
        "The topic to discuss".to_string(),
    )];

    let base_template = PromptTemplate::new(
        "Base Question Template".to_string(),
        "Can you explain {{topic}}?".to_string(),
        base_vars,
    );

    let base_id = base_template.id;
    let base_node_id = base_template.node_id;
    graph.create_template(base_template)?;
    println!("   Created base template: Base Question Template");

    // Child template with additional context
    let child_vars = vec![
        VariableSpec::new(
            "topic".to_string(),
            "String".to_string(),
            true,
            "The topic to discuss".to_string(),
        ),
        VariableSpec::new(
            "context".to_string(),
            "String".to_string(),
            true,
            "Additional context".to_string(),
        ),
    ];

    let child_template = PromptTemplate::from_parent(
        base_id,
        "Detailed Question Template".to_string(),
        "Can you explain {{topic}} in the context of {{context}}?".to_string(),
        child_vars,
    );

    let child_node_id = child_template.node_id;
    graph.create_template_from_parent(child_template, base_node_id)?;
    println!("   Created child template: Detailed Question Template");
    println!("   ✓ Inheritance relationship established");

    // Verify the inheritance edge
    let edges = graph.get_outgoing_edges(child_node_id)?;
    let inherits_count = edges
        .iter()
        .filter(|e| matches!(e.edge_type, llm_memory_graph::EdgeType::Inherits))
        .count();
    println!("   ✓ Found {} Inherits edge(s)\n", inherits_count);

    // ===== Example 5: Usage Tracking =====
    println!("5. Tracking template usage...");

    let mut usage_template = PromptTemplate::new(
        "Usage Tracking Template".to_string(),
        "Track usage for: {{item}}".to_string(),
        vec![],
    );

    let usage_node_id = usage_template.node_id;
    graph.create_template(usage_template.clone())?;
    println!("   Initial usage count: {}", usage_template.usage_count);

    // Simulate multiple uses
    for i in 1..=5 {
        usage_template.record_usage();
        graph.update_template(usage_template.clone())?;
        println!("   After use {}: count = {}", i, usage_template.usage_count);
    }

    // Retrieve and verify
    let retrieved = graph.get_template_by_node_id(usage_node_id)?;
    println!("   ✓ Verified final count: {}\n", retrieved.usage_count);

    // ===== Example 6: Complex Template with Multiple Features =====
    println!("6. Complex template combining all features...");

    let complex_vars = vec![
        VariableSpec::new(
            "user_name".to_string(),
            "String".to_string(),
            true,
            "User's full name".to_string(),
        )
        .with_validation(r"^[A-Z][a-z]+ [A-Z][a-z]+$".to_string()),
        VariableSpec::new(
            "task_type".to_string(),
            "String".to_string(),
            true,
            "Type of task".to_string(),
        )
        .with_validation("^(coding|review|documentation|testing)$".to_string()),
        VariableSpec::new(
            "urgency".to_string(),
            "String".to_string(),
            false,
            "Task urgency".to_string(),
        )
        .with_default("medium".to_string())
        .with_validation("^(low|medium|high)$".to_string()),
        VariableSpec::new(
            "deadline".to_string(),
            "String".to_string(),
            false,
            "Task deadline".to_string(),
        )
        .with_default("end of week".to_string()),
    ];

    let mut complex_template = PromptTemplate::new(
        "Task Assignment Template".to_string(),
        "Task for {{user_name}}:\nType: {{task_type}}\nUrgency: {{urgency}}\nDeadline: {{deadline}}\n\nPlease complete this task according to the specifications.".to_string(),
        complex_vars,
    )
    .with_description("Comprehensive task assignment template".to_string())
    .with_author("Task Manager".to_string());

    complex_template.add_tag("tasks".to_string());
    complex_template.add_tag("assignments".to_string());
    complex_template.add_tag("production".to_string());
    complex_template.add_metadata("department".to_string(), "engineering".to_string());
    complex_template.add_metadata("approval_required".to_string(), "false".to_string());

    graph.create_template(complex_template.clone())?;
    println!("   Created: {}", complex_template.name);
    println!("   Variables: {}", complex_template.variables.len());
    println!("   Tags: {:?}", complex_template.tags);
    println!("   Metadata: {:?}", complex_template.metadata);

    // Instantiate with various scenarios
    let mut scenario1 = HashMap::new();
    scenario1.insert("user_name".to_string(), "John Doe".to_string());
    scenario1.insert("task_type".to_string(), "coding".to_string());

    match complex_template.instantiate(&scenario1) {
        Ok(text) => {
            println!("\n   ✓ Scenario 1 (with defaults):");
            println!("   {}", text.lines().next().unwrap());
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    let mut scenario2 = HashMap::new();
    scenario2.insert("user_name".to_string(), "Jane Smith".to_string());
    scenario2.insert("task_type".to_string(), "review".to_string());
    scenario2.insert("urgency".to_string(), "high".to_string());
    scenario2.insert("deadline".to_string(), "tomorrow".to_string());

    match complex_template.instantiate(&scenario2) {
        Ok(text) => {
            println!("\n   ✓ Scenario 2 (custom values):");
            println!("   {}", text.lines().next().unwrap());
        }
        Err(e) => println!("   ✗ Error: {}", e),
    }

    println!("\n=== Summary ===");
    let stats = graph.stats()?;
    println!("Total nodes: {}", stats.node_count);
    println!("Total edges: {}", stats.edge_count);
    println!("\n✓ Template example completed successfully!");

    Ok(())
}
