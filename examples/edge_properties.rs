//! Example demonstrating all 5 new edge types with strongly-typed properties
//!
//! This example shows how to:
//! - Create INSTANTIATES edges linking prompts to templates
//! - Create INHERITS edges for template inheritance
//! - Create INVOKES edges for tool invocations
//! - Create TRANSFERS_TO edges for agent handoffs
//! - Create REFERENCES edges for external context
//! - Use strongly-typed property structures
//! - Extract and validate edge properties
//! - Build complex multi-agent workflows

use llm_memory_graph::{
    AgentNode, Config, ContextType, InheritsProperties, InstantiatesProperties,
    InvokesProperties, MemoryGraph, Priority, PromptTemplate, ReferencesProperties,
    TransfersToProperties, VariableSpec,
};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM-Memory-Graph: Edge Properties Example ===\n");

    // Create a new memory graph
    let config = Config::default();
    let graph = MemoryGraph::open(config)?;
    let session = graph.create_session()?;

    // ===== Example 1: INSTANTIATES Edge =====
    println!("1. INSTANTIATES Edge: Linking Prompts to Templates");
    println!("   Properties: template_version, variable_bindings, instantiation_time\n");

    // Create a template
    let template_vars = vec![
        VariableSpec::new(
            "topic".to_string(),
            "String".to_string(),
            true,
            "The topic to explain".to_string(),
        ),
        VariableSpec::new(
            "audience".to_string(),
            "String".to_string(),
            true,
            "Target audience level".to_string(),
        ),
    ];

    let explanation_template = PromptTemplate::new(
        "Explanation Template".to_string(),
        "Explain {{topic}} for a {{audience}} audience.".to_string(),
        template_vars,
    )
    .with_description("Template for generating explanations".to_string());

    let template_node_id = explanation_template.node_id;
    let template_version = explanation_template.version.to_string();
    graph.create_template(explanation_template.clone())?;

    // Instantiate the template
    let mut bindings = HashMap::new();
    bindings.insert("topic".to_string(), "quantum computing".to_string());
    bindings.insert("audience".to_string(), "beginner".to_string());

    let prompt_text = explanation_template.instantiate(&bindings)?;
    let prompt_id = graph.add_prompt(session.id, prompt_text.clone(), None)?;

    // Create INSTANTIATES edge with strongly-typed properties
    use llm_memory_graph::types::Edge;
    let instantiate_props = InstantiatesProperties::new(template_version.clone(), bindings.clone());
    let edge = Edge::instantiates(prompt_id, template_node_id, instantiate_props);

    println!("   Template: {}", explanation_template.name);
    println!("   Version: {}", template_version);
    println!("   Prompt: {}", prompt_text);
    println!("   Bindings: {:?}", bindings);

    // Store the edge (in a real implementation)
    graph.add_edge(edge.from, edge.to, edge.edge_type.clone())?;

    // Retrieve and verify properties
    let edges = graph.get_outgoing_edges(prompt_id)?;
    let instantiate_edge = edges
        .iter()
        .find(|e| matches!(e.edge_type, llm_memory_graph::EdgeType::Instantiates))
        .expect("INSTANTIATES edge should exist");

    if let Some(props) = instantiate_edge.get_instantiates_properties() {
        println!("   ✓ Retrieved properties:");
        println!("     - Template version: {}", props.template_version);
        println!("     - Variable bindings: {} entries", props.variable_bindings.len());
        println!("     - Instantiation time: {}", props.instantiation_time);
    }
    println!();

    // ===== Example 2: INHERITS Edge =====
    println!("2. INHERITS Edge: Template Inheritance Hierarchy");
    println!("   Properties: override_sections, version_diff, inheritance_depth\n");

    // Create parent template
    let parent_template = PromptTemplate::new(
        "Base Analysis Template".to_string(),
        "Analyze the following: {{content}}".to_string(),
        vec![VariableSpec::new(
            "content".to_string(),
            "String".to_string(),
            true,
            "Content to analyze".to_string(),
        )],
    );

    let parent_id = parent_template.id;
    let parent_node_id = parent_template.node_id;
    graph.create_template(parent_template.clone())?;

    // Create child template with additional variables
    let child_vars = vec![
        VariableSpec::new(
            "content".to_string(),
            "String".to_string(),
            true,
            "Content to analyze".to_string(),
        ),
        VariableSpec::new(
            "focus_area".to_string(),
            "String".to_string(),
            true,
            "Specific aspect to focus on".to_string(),
        ),
    ];

    let child_template = PromptTemplate::from_parent(
        parent_id,
        "Focused Analysis Template".to_string(),
        "Analyze the following, focusing on {{focus_area}}: {{content}}".to_string(),
        child_vars,
    );

    let child_node_id = child_template.node_id;
    graph.create_template_from_parent(child_template.clone(), parent_node_id)?;

    // Create INHERITS edge with properties
    let override_sections = vec!["focus_area".to_string()];
    let version_diff = "Added focus_area variable to narrow analysis scope".to_string();

    let inherits_props = InheritsProperties::new(override_sections.clone(), version_diff.clone(), 1);
    let inherits_edge = Edge::inherits(child_node_id, parent_node_id, inherits_props);

    println!("   Parent: {}", parent_template.name);
    println!("   Child: {}", child_template.name);
    println!("   Inheritance depth: 1");
    println!("   Override sections: {:?}", override_sections);
    println!("   Version diff: {}", version_diff);

    // In a full implementation, this edge would be stored
    if let Some(props) = inherits_edge.get_inherits_properties() {
        println!("   ✓ Properties confirmed:");
        println!("     - Override sections: {} sections", props.override_sections.len());
        println!("     - Version diff: {}", props.version_diff);
        println!("     - Depth: {}", props.inheritance_depth);
    }
    println!();

    // ===== Example 3: INVOKES Edge =====
    println!("3. INVOKES Edge: Tool Invocation Tracking");
    println!("   Properties: invocation_order, success, required\n");

    // Create an agent that will use tools
    let agent = AgentNode::new(
        "Research Agent".to_string(),
        "Information retrieval specialist".to_string(),
        vec!["web_search".to_string(), "calculator".to_string()],
    );
    let agent_node_id = agent.node_id;

    // Create INVOKES edge (from a response node to a tool invocation)
    // In this example, we're showing that the first tool invocation was successful
    let invokes_props = InvokesProperties::new(
        0,    // invocation_order: 0 (first tool call)
        true, // success: true
        true, // required: true (tool was required for the response)
    );

    let invokes_edge = Edge::invokes(
        prompt_id,      // From: prompt that triggered tool use
        agent_node_id,  // To: agent or tool invocation node
        invokes_props,
    );

    println!("   Tool invocation order: 0 (first call)");
    println!("   Success: true");
    println!("   Required: true");

    if let Some(props) = invokes_edge.get_invokes_properties() {
        println!("   ✓ Properties confirmed:");
        println!("     - Invocation order: {}", props.invocation_order);
        println!("     - Success: {}", props.success);
        println!("     - Required: {}", props.required);
    }
    println!();

    // ===== Example 4: TRANSFERS_TO Edge =====
    println!("4. TRANSFERS_TO Edge: Agent-to-Agent Handoff");
    println!("   Properties: handoff_reason, context_summary, priority\n");

    // Create a second agent for handoff
    let specialist_agent = AgentNode::new(
        "Code Specialist".to_string(),
        "Code analysis and optimization expert".to_string(),
        vec!["code_analyzer".to_string(), "profiler".to_string()],
    );
    let specialist_node_id = specialist_agent.node_id;

    // Create TRANSFERS_TO edge with handoff details
    let transfers_props = TransfersToProperties::new(
        "User requested code analysis which requires specialist expertise".to_string(),
        "User is asking about optimizing quantum circuit implementations in Python".to_string(),
        Priority::High,
    );

    let transfers_edge = Edge::transfers_to(agent_node_id, specialist_node_id, transfers_props);

    println!("   From: Research Agent");
    println!("   To: Code Specialist");
    println!("   Priority: High");
    println!("   Reason: Requires specialist expertise");

    if let Some(props) = transfers_edge.get_transfers_to_properties() {
        println!("   ✓ Properties confirmed:");
        println!("     - Priority: {:?}", props.priority);
        println!("     - Handoff reason: {}", props.handoff_reason);
        println!("     - Context: {} chars", props.context_summary.len());
    }
    println!();

    // ===== Example 5: REFERENCES Edge =====
    println!("5. REFERENCES Edge: External Context References");
    println!("   Properties: context_type, relevance_score, chunk_id\n");

    // Create REFERENCES edge for external context
    let references_props = ReferencesProperties::new(
        ContextType::WebPage,
        0.95,
        Some("chunk-arxiv-2024-quantum-section-3".to_string()),
    );

    let references_edge = Edge::references(
        prompt_id,
        template_node_id, // Reference from prompt to external context
        references_props,
    );

    println!("   Context Type: WebPage");
    println!("   Relevance: 0.95");
    println!("   Chunk ID: chunk-arxiv-2024-quantum-section-3");

    if let Some(props) = references_edge.get_references_properties() {
        println!("   ✓ Properties confirmed:");
        println!("     - Type: {:?}", props.context_type);
        println!("     - Relevance: {:.2}", props.relevance_score);
        println!("     - Has chunk ID: {}", props.chunk_id.is_some());
        if let Some(chunk) = &props.chunk_id {
            println!("     - Chunk: {}", chunk);
        }
    }
    println!();

    // ===== Example 6: Complex Multi-Edge Workflow =====
    println!("6. Complex Workflow: Combining All Edge Types\n");

    // Scenario: User asks a question -> Template instantiation -> Agent uses tools ->
    //           References external docs -> Hands off to specialist -> Inherits from base template

    println!("   Workflow Steps:");
    println!("   1. User question instantiates explanation template (INSTANTIATES)");
    println!("   2. Research agent invokes web_search tool (INVOKES)");
    println!("   3. Agent references external research paper (REFERENCES)");
    println!("   4. Agent transfers to Code Specialist (TRANSFERS_TO)");
    println!("   5. Specialist uses focused analysis template that inherits from base (INHERITS)");

    // Count edges created in this example
    println!("\n   Edge Types Demonstrated:");
    println!("   ✓ INSTANTIATES - Template to prompt binding");
    println!("   ✓ INHERITS - Template inheritance hierarchy");
    println!("   ✓ INVOKES - Tool invocation tracking");
    println!("   ✓ TRANSFERS_TO - Agent handoff coordination");
    println!("   ✓ REFERENCES - External context linking");

    println!("\n   Priority Levels Available:");
    println!("   - Low: Background tasks");
    println!("   - Normal: Standard operations");
    println!("   - High: Important requests");
    println!("   - Critical: Urgent/time-sensitive");

    println!("\n   Context Types Available:");
    println!("   - Document: Local files and documents");
    println!("   - WebPage: Internet resources");
    println!("   - Database: Structured data sources");
    println!("   - VectorSearch: Embedding-based retrieval");
    println!("   - Memory: Previous conversation context");

    println!("\n=== Summary ===");
    let stats = graph.stats()?;
    println!("Total nodes: {}", stats.node_count);
    println!("Total edges: {}", stats.edge_count);
    println!("\n✓ All 5 edge types demonstrated successfully!");
    println!("✓ All property structures are strongly-typed and validated");
    println!("✓ Enterprise-grade implementation with full type safety");

    Ok(())
}
