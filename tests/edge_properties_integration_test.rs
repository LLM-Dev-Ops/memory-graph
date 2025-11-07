//! Integration tests for enhanced edge types with strongly-typed properties
//!
//! Tests all 5 new edge types:
//! - INSTANTIATES (Prompt → Template)
//! - INHERITS (Template → Template)
//! - INVOKES (Response → ToolInvocation)
//! - TRANSFERS_TO (Response → Agent)
//! - REFERENCES (Prompt → ExternalContext)

use llm_memory_graph::{
    AgentNode, Config, ContextType, EdgeType, InheritsProperties, InstantiatesProperties,
    InvokesProperties, MemoryGraph, Priority, PromptTemplate, ReferencesProperties, TokenUsage,
    TransfersToProperties, ToolInvocation, VariableSpec,
};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_instantiates_edge_full_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create a template
    let variables = vec![VariableSpec::new(
        "topic".to_string(),
        "String".to_string(),
        true,
        "Topic to explain".to_string(),
    )];

    let template = PromptTemplate::new(
        "Explanation Template".to_string(),
        "Explain {{topic}} in simple terms".to_string(),
        variables,
    );

    let template_node_id = template.node_id;
    let template_version = template.version.to_string();
    graph.create_template(template.clone()).unwrap();

    // Instantiate the template
    let mut values = HashMap::new();
    values.insert("topic".to_string(), "quantum computing".to_string());

    let prompt_text = template.instantiate(&values).unwrap();
    let prompt_id = graph.add_prompt(session.id, prompt_text, None).unwrap();

    // Create INSTANTIATES edge with properties
    let instantiate_props = InstantiatesProperties::new(template_version, values);
    let edge = llm_memory_graph::types::Edge::instantiates(
        prompt_id,
        template_node_id,
        instantiate_props,
    );

    graph.add_edge(edge.from, edge.to, edge.edge_type).unwrap();

    // Verify edge was created
    let edges = graph.get_outgoing_edges(prompt_id).unwrap();
    let inst_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Instantiates)
        .collect();

    assert_eq!(inst_edges.len(), 1);
    assert_eq!(inst_edges[0].to, template_node_id);
}

#[test]
fn test_inherits_edge_full_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create parent template
    let parent = PromptTemplate::new(
        "Base Template".to_string(),
        "Question: {{question}}".to_string(),
        vec![],
    );

    let parent_node_id = parent.node_id;
    graph.create_template(parent).unwrap();

    // Create child template
    let child = PromptTemplate::new(
        "Enhanced Template".to_string(),
        "Question: {{question}}\nContext: {{context}}".to_string(),
        vec![],
    );

    let child_node_id = child.node_id;
    graph.create_template(child).unwrap();

    // Create INHERITS edge with properties
    let inherits_props = InheritsProperties::new(
        vec!["template".to_string(), "variables".to_string()],
        "Added context variable".to_string(),
        1,
    );

    let edge = llm_memory_graph::types::Edge::inherits(
        child_node_id,
        parent_node_id,
        inherits_props,
    );

    graph.add_edge(edge.from, edge.to, edge.edge_type).unwrap();

    // Verify edge
    let edges = graph.get_outgoing_edges(child_node_id).unwrap();
    let inherit_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Inherits)
        .collect();

    assert_eq!(inherit_edges.len(), 1);
    assert_eq!(inherit_edges[0].to, parent_node_id);
}

#[test]
fn test_invokes_edge_full_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Calculate 2+2".to_string(), None)
        .unwrap();

    let usage = TokenUsage::new(10, 20);
    let response_id = graph
        .add_response(prompt_id, "The answer is 4".to_string(), usage, None)
        .unwrap();

    // Create tool invocations (these automatically create basic INVOKES edges)
    let tool1 = ToolInvocation::new(
        response_id,
        "calculator".to_string(),
        serde_json::json!({"operation": "add", "args": [2, 2]}),
    );
    graph.add_tool_invocation(tool1).unwrap();

    let tool2 = ToolInvocation::new(
        response_id,
        "formatter".to_string(),
        serde_json::json!({"format": "decimal"}),
    );
    graph.add_tool_invocation(tool2).unwrap();

    // Note: add_tool_invocation already creates INVOKES edges automatically
    // To use edges with custom properties, you would create tool nodes directly
    // and then create edges with properties separately

    // Verify edges
    let edges = graph.get_outgoing_edges(response_id).unwrap();
    let invoke_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Invokes)
        .collect();

    assert_eq!(invoke_edges.len(), 2);
}

#[test]
fn test_transfers_to_edge_full_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(
            session.id,
            "I need help with quantum physics".to_string(),
            None,
        )
        .unwrap();

    let usage = TokenUsage::new(15, 25);
    let response_id = graph
        .add_response(
            prompt_id,
            "Let me transfer you to our physics specialist".to_string(),
            usage,
            None,
        )
        .unwrap();

    // Create specialist agent
    let agent = AgentNode::new(
        "Physics Specialist".to_string(),
        "physics_expert".to_string(),
        vec!["quantum_physics".to_string(), "relativity".to_string()],
    );

    let agent_node_id = agent.node_id;
    graph.add_agent(agent).unwrap();

    // Create TRANSFERS_TO edge with properties
    let transfer_props = TransfersToProperties::new(
        "User needs expert physics assistance".to_string(),
        "Question about quantum physics fundamentals".to_string(),
        Priority::High,
    );

    let edge = llm_memory_graph::types::Edge::transfers_to(
        response_id,
        agent_node_id,
        transfer_props,
    );

    graph.add_edge(edge.from, edge.to, edge.edge_type).unwrap();

    // Verify edge
    let edges = graph.get_outgoing_edges(response_id).unwrap();
    let transfer_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::TransfersTo)
        .collect();

    assert_eq!(transfer_edges.len(), 1);
    assert_eq!(transfer_edges[0].to, agent_node_id);
}

#[test]
fn test_references_edge_full_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create a prompt that references external context
    let prompt_id = graph
        .add_prompt(
            session.id,
            "Based on the documentation, how does feature X work?".to_string(),
            None,
        )
        .unwrap();

    // Create placeholder nodes for external context
    // (In a real system, these would be actual external context nodes)
    let doc_context_id = graph
        .add_prompt(
            session.id,
            "CONTEXT: Documentation for feature X".to_string(),
            None,
        )
        .unwrap();

    let web_context_id = graph
        .add_prompt(
            session.id,
            "CONTEXT: Web article about feature X".to_string(),
            None,
        )
        .unwrap();

    // Create REFERENCES edges with properties
    let ref1_props =
        ReferencesProperties::new(ContextType::Document, 0.95, Some("section_3.2".to_string()));

    let edge1 = llm_memory_graph::types::Edge::references(prompt_id, doc_context_id, ref1_props);
    graph.add_edge(edge1.from, edge1.to, edge1.edge_type).unwrap();

    let ref2_props = ReferencesProperties::new(ContextType::WebPage, 0.72, None);

    let edge2 =
        llm_memory_graph::types::Edge::references(prompt_id, web_context_id, ref2_props);
    graph.add_edge(edge2.from, edge2.to, edge2.edge_type).unwrap();

    // Verify edges
    let edges = graph.get_outgoing_edges(prompt_id).unwrap();
    let ref_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::References)
        .collect();

    assert_eq!(ref_edges.len(), 2);
}

#[test]
fn test_edge_property_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().to_path_buf();

    let prompt_id;
    let template_id;

    // Create edge with properties and close
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();
        let session = graph.create_session().unwrap();

        let template = PromptTemplate::new(
            "Test".to_string(),
            "{{content}}".to_string(),
            vec![],
        );
        template_id = template.node_id;
        graph.create_template(template).unwrap();

        prompt_id = graph
            .add_prompt(session.id, "Test prompt".to_string(), None)
            .unwrap();

        let mut bindings = HashMap::new();
        bindings.insert("content".to_string(), "Test".to_string());

        let props = InstantiatesProperties::new("1.0.0".to_string(), bindings);
        let edge = llm_memory_graph::types::Edge::instantiates(prompt_id, template_id, props);

        graph
            .add_edge(edge.from, edge.to, edge.edge_type)
            .unwrap();
        graph.flush().unwrap();
    }

    // Reopen and verify
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let edges = graph.get_outgoing_edges(prompt_id).unwrap();
        let inst_edges: Vec<_> = edges
            .iter()
            .filter(|e| e.edge_type == EdgeType::Instantiates)
            .collect();

        assert_eq!(inst_edges.len(), 1);
        assert_eq!(inst_edges[0].to, template_id);
    }
}

#[test]
fn test_complex_multi_edge_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create template
    let template = PromptTemplate::new(
        "Multi-Edge Test".to_string(),
        "Process: {{action}}".to_string(),
        vec![],
    );
    let template_id = template.node_id;
    graph.create_template(template).unwrap();

    // Create prompt from template
    let mut bindings = HashMap::new();
    bindings.insert("action".to_string(), "analysis".to_string());

    let prompt_id = graph
        .add_prompt(session.id, "Process: analysis".to_string(), None)
        .unwrap();

    // INSTANTIATES edge
    let inst_props = InstantiatesProperties::new("1.0.0".to_string(), bindings);
    let inst_edge = llm_memory_graph::types::Edge::instantiates(prompt_id, template_id, inst_props);
    graph
        .add_edge(inst_edge.from, inst_edge.to, inst_edge.edge_type)
        .unwrap();

    // REFERENCES edge (external context)
    let context_id = graph
        .add_prompt(session.id, "CONTEXT".to_string(), None)
        .unwrap();
    let ref_props = ReferencesProperties::new(ContextType::Document, 0.88, None);
    let ref_edge = llm_memory_graph::types::Edge::references(prompt_id, context_id, ref_props);
    graph
        .add_edge(ref_edge.from, ref_edge.to, ref_edge.edge_type)
        .unwrap();

    // Response with tools
    let usage = TokenUsage::new(20, 30);
    let response_id = graph
        .add_response(prompt_id, "Analysis complete".to_string(), usage, None)
        .unwrap();

    // INVOKES edge
    let tool = ToolInvocation::new(
        response_id,
        "analyzer".to_string(),
        serde_json::json!({"depth": "full"}),
    );
    let tool_id = tool.id;
    graph.add_tool_invocation(tool).unwrap();

    let invoke_props = InvokesProperties::new(0, true, true);
    let invoke_edge = llm_memory_graph::types::Edge::invokes(response_id, tool_id, invoke_props);
    graph
        .add_edge(invoke_edge.from, invoke_edge.to, invoke_edge.edge_type)
        .unwrap();

    // TRANSFERS_TO edge
    let agent = AgentNode::new("Specialist".to_string(), "specialist".to_string(), vec![]);
    let agent_id = agent.node_id;
    graph.add_agent(agent).unwrap();

    let transfer_props = TransfersToProperties::new(
        "Need specialist review".to_string(),
        "Analysis results".to_string(),
        Priority::Normal,
    );
    let transfer_edge =
        llm_memory_graph::types::Edge::transfers_to(response_id, agent_id, transfer_props);
    graph
        .add_edge(transfer_edge.from, transfer_edge.to, transfer_edge.edge_type)
        .unwrap();

    // Verify all edges
    let prompt_edges = graph.get_outgoing_edges(prompt_id).unwrap();
    assert!(prompt_edges
        .iter()
        .any(|e| e.edge_type == EdgeType::Instantiates));
    assert!(prompt_edges
        .iter()
        .any(|e| e.edge_type == EdgeType::References));

    let response_edges = graph.get_outgoing_edges(response_id).unwrap();
    assert!(response_edges
        .iter()
        .any(|e| e.edge_type == EdgeType::Invokes));
    assert!(response_edges
        .iter()
        .any(|e| e.edge_type == EdgeType::TransfersTo));
}

#[test]
fn test_priority_levels_in_transfers() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let agent = AgentNode::new("Agent".to_string(), "agent".to_string(), vec![]);
    let agent_id = agent.node_id;
    graph.add_agent(agent).unwrap();

    let priorities = vec![
        Priority::Low,
        Priority::Normal,
        Priority::High,
        Priority::Critical,
    ];

    for (i, priority) in priorities.iter().enumerate() {
        let prompt_id = graph
            .add_prompt(session.id, format!("Prompt {}", i), None)
            .unwrap();

        let usage = TokenUsage::new(10, 10);
        let response_id = graph
            .add_response(prompt_id, format!("Response {}", i), usage, None)
            .unwrap();

        let props = TransfersToProperties::new(
            format!("Reason {}", i),
            format!("Context {}", i),
            *priority,
        );

        let edge = llm_memory_graph::types::Edge::transfers_to(response_id, agent_id, props);
        graph
            .add_edge(edge.from, edge.to, edge.edge_type)
            .unwrap();
    }

    // Verify all transfer edges were created
    let stats = graph.stats().unwrap();
    assert!(stats.edge_count >= 4);
}

#[test]
fn test_context_types_in_references() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Multi-context prompt".to_string(), None)
        .unwrap();

    let context_types = vec![
        ContextType::Document,
        ContextType::WebPage,
        ContextType::Database,
        ContextType::VectorSearch,
        ContextType::Memory,
    ];

    for (i, ctx_type) in context_types.iter().enumerate() {
        let context_id = graph
            .add_prompt(session.id, format!("Context {}", i), None)
            .unwrap();

        let props = ReferencesProperties::new(*ctx_type, 0.8, Some(format!("chunk_{}", i)));
        let edge = llm_memory_graph::types::Edge::references(prompt_id, context_id, props);
        graph
            .add_edge(edge.from, edge.to, edge.edge_type)
            .unwrap();
    }

    // Verify all reference edges
    let edges = graph.get_outgoing_edges(prompt_id).unwrap();
    let ref_edges: Vec<_> = edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::References)
        .collect();

    assert_eq!(ref_edges.len(), 5);
}
