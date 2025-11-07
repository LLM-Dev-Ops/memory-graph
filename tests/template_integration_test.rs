//! Integration tests for PromptTemplate functionality
//!
//! These tests verify the full workflow of creating templates, instantiating them,
//! tracking usage, and managing template inheritance.

use llm_memory_graph::{
    Config, EdgeType, MemoryGraph, PromptTemplate, VariableSpec, Version, VersionLevel,
};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_template_creation_and_retrieval() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create a template
    let variables = vec![VariableSpec::new(
        "name".to_string(),
        "String".to_string(),
        true,
        "User's name".to_string(),
    )];

    let template = PromptTemplate::new(
        "Greeting Template".to_string(),
        "Hello, {{name}}!".to_string(),
        variables,
    )
    .with_description("A simple greeting template".to_string())
    .with_author("Test Author".to_string());

    let template_node_id = template.node_id;
    let template_id = graph.create_template(template).unwrap();

    // Retrieve the template
    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();

    assert_eq!(retrieved.id, template_id);
    assert_eq!(retrieved.name, "Greeting Template");
    assert_eq!(retrieved.template, "Hello, {{name}}!");
    assert_eq!(retrieved.description, "A simple greeting template");
    assert_eq!(retrieved.author, "Test Author");
    assert_eq!(retrieved.variables.len(), 1);
    assert_eq!(retrieved.version, Version::new(1, 0, 0));
}

#[test]
fn test_template_instantiation_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create a template
    let variables = vec![
        VariableSpec::new(
            "user_name".to_string(),
            "String".to_string(),
            true,
            "User's name".to_string(),
        ),
        VariableSpec::new(
            "topic".to_string(),
            "String".to_string(),
            true,
            "Topic to discuss".to_string(),
        ),
    ];

    let template = PromptTemplate::new(
        "Question Template".to_string(),
        "Hey {{user_name}}, can you explain {{topic}} to me?".to_string(),
        variables,
    );

    let template_node_id = template.node_id;
    graph.create_template(template.clone()).unwrap();

    // Instantiate the template
    let mut values = HashMap::new();
    values.insert("user_name".to_string(), "Alice".to_string());
    values.insert("topic".to_string(), "quantum computing".to_string());

    let prompt_text = template.instantiate(&values).unwrap();
    assert_eq!(
        prompt_text,
        "Hey Alice, can you explain quantum computing to me?"
    );

    // Create a prompt from the template
    let prompt_id = graph.add_prompt(session.id, prompt_text, None).unwrap();

    // Link prompt to template
    graph
        .link_prompt_to_template(prompt_id, template_node_id)
        .unwrap();

    // Verify the link exists
    let outgoing = graph.get_outgoing_edges(prompt_id).unwrap();
    let instantiates_edges: Vec<_> = outgoing
        .iter()
        .filter(|e| e.edge_type == EdgeType::Instantiates)
        .collect();

    assert_eq!(instantiates_edges.len(), 1);
    assert_eq!(instantiates_edges[0].to, template_node_id);
}

#[test]
fn test_template_with_defaults() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let variables = vec![
        VariableSpec::new(
            "name".to_string(),
            "String".to_string(),
            true,
            "Name".to_string(),
        ),
        VariableSpec::new(
            "greeting".to_string(),
            "String".to_string(),
            false,
            "Greeting".to_string(),
        )
        .with_default("Hi".to_string()),
    ];

    let template = PromptTemplate::new(
        "Flexible Greeting".to_string(),
        "{{greeting}}, {{name}}!".to_string(),
        variables,
    );

    graph.create_template(template.clone()).unwrap();

    // Use with default
    let mut values = HashMap::new();
    values.insert("name".to_string(), "Bob".to_string());
    let result = template.instantiate(&values).unwrap();
    assert_eq!(result, "Hi, Bob!");

    // Override default
    let mut values = HashMap::new();
    values.insert("name".to_string(), "Bob".to_string());
    values.insert("greeting".to_string(), "Hello".to_string());
    let result = template.instantiate(&values).unwrap();
    assert_eq!(result, "Hello, Bob!");
}

#[test]
fn test_template_validation() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let variables = vec![
        VariableSpec::new(
            "email".to_string(),
            "String".to_string(),
            true,
            "Email address".to_string(),
        )
        .with_validation(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
    ];

    let template = PromptTemplate::new(
        "Email Template".to_string(),
        "Send email to: {{email}}".to_string(),
        variables,
    );

    graph.create_template(template.clone()).unwrap();

    // Valid email
    let mut values = HashMap::new();
    values.insert("email".to_string(), "test@example.com".to_string());
    assert!(template.instantiate(&values).is_ok());

    // Invalid email
    let mut values = HashMap::new();
    values.insert("email".to_string(), "not-an-email".to_string());
    assert!(template.instantiate(&values).is_err());
}

#[test]
fn test_template_usage_tracking() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let mut template = PromptTemplate::new(
        "Usage Test".to_string(),
        "Template {{x}}".to_string(),
        vec![],
    );

    let template_node_id = template.node_id;
    graph.create_template(template.clone()).unwrap();

    assert_eq!(template.usage_count, 0);

    // Record usage
    template.record_usage();
    assert_eq!(template.usage_count, 1);

    // Update template
    graph.update_template(template.clone()).unwrap();

    // Retrieve and verify
    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.usage_count, 1);

    // Record more usage
    template.record_usage();
    template.record_usage();
    assert_eq!(template.usage_count, 3);

    graph.update_template(template).unwrap();

    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.usage_count, 3);
}

#[test]
fn test_template_versioning() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let mut template = PromptTemplate::new(
        "Versioned Template".to_string(),
        "Version {{x}}".to_string(),
        vec![],
    );

    let template_node_id = template.node_id;
    graph.create_template(template.clone()).unwrap();

    assert_eq!(template.version, Version::new(1, 0, 0));

    // Bump patch version
    template.bump_version(VersionLevel::Patch);
    assert_eq!(template.version, Version::new(1, 0, 1));
    graph.update_template(template.clone()).unwrap();

    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.version, Version::new(1, 0, 1));

    // Bump minor version
    template.bump_version(VersionLevel::Minor);
    assert_eq!(template.version, Version::new(1, 1, 0));
    graph.update_template(template.clone()).unwrap();

    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.version, Version::new(1, 1, 0));

    // Bump major version
    template.bump_version(VersionLevel::Major);
    assert_eq!(template.version, Version::new(2, 0, 0));
    graph.update_template(template).unwrap();

    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.version, Version::new(2, 0, 0));
}

#[test]
fn test_template_inheritance() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create parent template
    let parent = PromptTemplate::new(
        "Parent Template".to_string(),
        "Base prompt: {{content}}".to_string(),
        vec![],
    );

    let parent_id = parent.id;
    let parent_node_id = parent.node_id;
    graph.create_template(parent).unwrap();

    // Create child template that inherits from parent
    let child = PromptTemplate::from_parent(
        parent_id,
        "Child Template".to_string(),
        "Extended prompt: {{content}} with {{extra}}".to_string(),
        vec![],
    );

    let child_node_id = child.node_id;
    let child_id = graph
        .create_template_from_parent(child, parent_node_id)
        .unwrap();

    // Verify child template
    let retrieved_child = graph.get_template_by_node_id(child_node_id).unwrap();
    assert_eq!(retrieved_child.id, child_id);
    assert_eq!(retrieved_child.parent_id, Some(parent_id));
    assert_eq!(retrieved_child.name, "Child Template");

    // Verify Inherits edge exists
    let outgoing = graph.get_outgoing_edges(child_node_id).unwrap();
    let inherits_edges: Vec<_> = outgoing
        .iter()
        .filter(|e| e.edge_type == EdgeType::Inherits)
        .collect();

    assert_eq!(inherits_edges.len(), 1);
    assert_eq!(inherits_edges[0].to, parent_node_id);
}

#[test]
fn test_template_tags_and_metadata() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let mut template = PromptTemplate::new(
        "Tagged Template".to_string(),
        "Content: {{x}}".to_string(),
        vec![],
    );

    let template_node_id = template.node_id;

    template.add_tag("production".to_string());
    template.add_tag("verified".to_string());
    template.add_metadata("category".to_string(), "greeting".to_string());
    template.add_metadata("priority".to_string(), "high".to_string());

    graph.create_template(template).unwrap();

    // Retrieve and verify
    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.tags.len(), 2);
    assert!(retrieved.tags.contains(&"production".to_string()));
    assert!(retrieved.tags.contains(&"verified".to_string()));
    assert_eq!(retrieved.metadata.len(), 2);
    assert_eq!(
        retrieved.metadata.get("category"),
        Some(&"greeting".to_string())
    );
    assert_eq!(
        retrieved.metadata.get("priority"),
        Some(&"high".to_string())
    );
}

#[test]
fn test_template_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().to_path_buf();

    let template_node_id;
    let template_id;

    // Create template and close
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let variables = vec![VariableSpec::new(
            "var".to_string(),
            "String".to_string(),
            true,
            "Variable".to_string(),
        )];

        let template = PromptTemplate::new(
            "Persistent Template".to_string(),
            "Value: {{var}}".to_string(),
            variables,
        );

        template_node_id = template.node_id;
        template_id = graph.create_template(template).unwrap();

        graph.flush().unwrap();
        // Graph is dropped here
    }

    // Reopen and verify template persists
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
        assert_eq!(retrieved.id, template_id);
        assert_eq!(retrieved.name, "Persistent Template");
        assert_eq!(retrieved.template, "Value: {{var}}");
        assert_eq!(retrieved.variables.len(), 1);
        assert_eq!(retrieved.variables[0].name, "var");
    }
}

#[test]
fn test_complex_template_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create a complex template with multiple variables and validation
    let variables = vec![
        VariableSpec::new(
            "recipient".to_string(),
            "String".to_string(),
            true,
            "Recipient name".to_string(),
        ),
        VariableSpec::new(
            "subject".to_string(),
            "String".to_string(),
            true,
            "Email subject".to_string(),
        ),
        VariableSpec::new(
            "urgency".to_string(),
            "String".to_string(),
            false,
            "Urgency level".to_string(),
        )
        .with_default("normal".to_string())
        .with_validation("^(low|normal|high|critical)$".to_string()),
        VariableSpec::new(
            "signature".to_string(),
            "String".to_string(),
            false,
            "Email signature".to_string(),
        )
        .with_default("Best regards".to_string()),
    ];

    let mut template = PromptTemplate::new(
        "Email Template".to_string(),
        "To: {{recipient}}\nSubject: [{{urgency}}] {{subject}}\n\nDear {{recipient}},\n\n[Message body here]\n\n{{signature}}".to_string(),
        variables,
    ).with_description("Professional email template".to_string())
      .with_author("System".to_string());

    template.add_tag("email".to_string());
    template.add_tag("professional".to_string());
    template.add_metadata("category".to_string(), "communication".to_string());

    let template_node_id = template.node_id;
    graph.create_template(template.clone()).unwrap();

    // Instantiate with minimal values (using defaults)
    let mut values = HashMap::new();
    values.insert("recipient".to_string(), "John".to_string());
    values.insert("subject".to_string(), "Meeting Reminder".to_string());

    let result = template.instantiate(&values).unwrap();
    assert!(result.contains("To: John"));
    assert!(result.contains("[normal] Meeting Reminder"));
    assert!(result.contains("Best regards"));

    // Create prompt from template
    let prompt_id = graph.add_prompt(session.id, result, None).unwrap();
    graph
        .link_prompt_to_template(prompt_id, template_node_id)
        .unwrap();

    // Record usage and update version
    template.record_usage();
    template.bump_version(VersionLevel::Patch);
    graph.update_template(template).unwrap();

    // Verify everything persisted correctly
    let retrieved = graph.get_template_by_node_id(template_node_id).unwrap();
    assert_eq!(retrieved.usage_count, 1);
    assert_eq!(retrieved.version, Version::new(1, 0, 1));
    assert_eq!(retrieved.tags.len(), 2);
    assert_eq!(retrieved.metadata.len(), 1);

    // Verify edges
    let instantiates_edges = graph.get_outgoing_edges(prompt_id).unwrap();
    assert!(instantiates_edges
        .iter()
        .any(|e| e.edge_type == EdgeType::Instantiates && e.to == template_node_id));
}
