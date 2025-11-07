//! Integration tests for LLM-Memory-Graph
//!
//! These tests verify the full workflow of creating sessions, adding prompts/responses,
//! querying the graph, and persistence.

use llm_memory_graph::{
    query::{GraphTraversal, QueryBuilder},
    Config, EdgeType, MemoryGraph, NodeType, PromptMetadata, ResponseMetadata, TokenUsage,
};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_full_conversation_workflow() {
    // Create a temporary database
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create a session
    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "alice".to_string());
    metadata.insert("app".to_string(), "chatbot".to_string());
    let session = graph.create_session_with_metadata(metadata).unwrap();

    assert_eq!(
        session.metadata.get("user"),
        Some(&"alice".to_string())
    );

    // Add first prompt
    let prompt_meta = PromptMetadata {
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_tokens: Some(500),
        tools_available: vec!["web_search".to_string()],
        custom: HashMap::new(),
    };

    let prompt1_id = graph
        .add_prompt(
            session.id,
            "What is the capital of France?".to_string(),
            Some(prompt_meta.clone()),
        )
        .unwrap();

    // Add first response
    let usage1 = TokenUsage::new(15, 8);
    let response_meta = ResponseMetadata {
        model: "gpt-4".to_string(),
        finish_reason: "stop".to_string(),
        latency_ms: 250,
        custom: HashMap::new(),
    };

    let response1_id = graph
        .add_response(
            prompt1_id,
            "The capital of France is Paris.".to_string(),
            usage1,
            Some(response_meta.clone()),
        )
        .unwrap();

    // Add second prompt (follow-up question)
    let prompt2_id = graph
        .add_prompt(
            session.id,
            "What is its population?".to_string(),
            Some(prompt_meta),
        )
        .unwrap();

    // Add second response
    let usage2 = TokenUsage::new(12, 18);
    let response2_id = graph
        .add_response(
            prompt2_id,
            "Paris has a population of approximately 2.2 million people.".to_string(),
            usage2,
            Some(response_meta),
        )
        .unwrap();

    // Verify nodes were created
    let prompt1 = graph.get_node(prompt1_id).unwrap();
    let response1 = graph.get_node(response1_id).unwrap();
    let prompt2 = graph.get_node(prompt2_id).unwrap();
    let response2 = graph.get_node(response2_id).unwrap();

    assert!(matches!(prompt1, llm_memory_graph::types::Node::Prompt(_)));
    assert!(matches!(
        response1,
        llm_memory_graph::types::Node::Response(_)
    ));
    assert!(matches!(prompt2, llm_memory_graph::types::Node::Prompt(_)));
    assert!(matches!(
        response2,
        llm_memory_graph::types::Node::Response(_)
    ));

    // Test querying by session
    let session_nodes = graph.get_session_nodes(session.id).unwrap();
    assert!(session_nodes.len() >= 5); // session + 2 prompts + 2 responses

    // Test querying with filters
    let prompts = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Prompt)
        .execute()
        .unwrap();

    assert_eq!(prompts.len(), 2);

    let responses = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Response)
        .execute()
        .unwrap();

    assert_eq!(responses.len(), 2);
}

#[test]
fn test_edge_creation_and_traversal() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Create a chain of prompts
    let prompt1 = graph
        .add_prompt(session.id, "First prompt".to_string(), None)
        .unwrap();
    let _prompt2 = graph
        .add_prompt(session.id, "Second prompt".to_string(), None)
        .unwrap();
    let prompt3 = graph
        .add_prompt(session.id, "Third prompt".to_string(), None)
        .unwrap();

    // Verify edges exist
    let outgoing_edges = graph.get_outgoing_edges(prompt3).unwrap();
    assert!(!outgoing_edges.is_empty());

    // Find edges of specific type
    let follows_edges: Vec<_> = outgoing_edges
        .iter()
        .filter(|e| e.edge_type == EdgeType::Follows)
        .collect();
    assert!(!follows_edges.is_empty());

    // Test graph traversal
    let traversal = GraphTraversal::new(&graph);
    let bfs_nodes = traversal.bfs(prompt1).unwrap();
    assert!(!bfs_nodes.is_empty());

    let dfs_nodes = traversal.dfs(prompt1).unwrap();
    assert!(!dfs_nodes.is_empty());
}

#[test]
fn test_conversation_thread_retrieval() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Build a conversation
    let prompt1 = graph
        .add_prompt(session.id, "Hello".to_string(), None)
        .unwrap();
    let usage = TokenUsage::new(5, 10);
    let _response1 = graph
        .add_response(prompt1, "Hi there!".to_string(), usage, None)
        .unwrap();

    let prompt2 = graph
        .add_prompt(session.id, "How are you?".to_string(), None)
        .unwrap();
    let _response2 = graph
        .add_response(prompt2, "I'm doing well!".to_string(), usage, None)
        .unwrap();

    let prompt3 = graph
        .add_prompt(session.id, "That's great!".to_string(), None)
        .unwrap();
    let _response3 = graph
        .add_response(prompt3, "Thank you!".to_string(), usage, None)
        .unwrap();

    // Get the full conversation thread
    let traversal = GraphTraversal::new(&graph);
    let thread = traversal.get_conversation_thread(prompt2).unwrap();

    // Should have all prompts and responses in chronological order
    assert_eq!(thread.len(), 6); // 3 prompts + 3 responses

    // Verify order is chronological
    for i in 1..thread.len() {
        let time_prev = match &thread[i - 1] {
            llm_memory_graph::types::Node::Prompt(p) => p.timestamp,
            llm_memory_graph::types::Node::Response(r) => r.timestamp,
            llm_memory_graph::types::Node::Session(s) => s.created_at,
            llm_memory_graph::types::Node::ToolInvocation(t) => t.timestamp,
            llm_memory_graph::types::Node::Agent(a) => a.created_at,
        };
        let time_curr = match &thread[i] {
            llm_memory_graph::types::Node::Prompt(p) => p.timestamp,
            llm_memory_graph::types::Node::Response(r) => r.timestamp,
            llm_memory_graph::types::Node::Session(s) => s.created_at,
            llm_memory_graph::types::Node::ToolInvocation(t) => t.timestamp,
            llm_memory_graph::types::Node::Agent(a) => a.created_at,
        };
        assert!(time_prev <= time_curr);
    }
}

#[test]
fn test_find_responses_to_prompt() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Test prompt".to_string(), None)
        .unwrap();

    // Add multiple responses to the same prompt
    let usage = TokenUsage::new(10, 20);
    let response1_id = graph
        .add_response(prompt_id, "First response".to_string(), usage, None)
        .unwrap();
    let response2_id = graph
        .add_response(prompt_id, "Second response".to_string(), usage, None)
        .unwrap();

    // Find all responses
    let traversal = GraphTraversal::new(&graph);
    let responses = traversal.find_responses(prompt_id).unwrap();

    assert_eq!(responses.len(), 2);

    let response_ids: Vec<_> = responses.iter().map(|n| n.id()).collect();
    assert!(response_ids.contains(&response1_id));
    assert!(response_ids.contains(&response2_id));
}

#[test]
fn test_persistence_close_and_reopen() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().to_path_buf();

    let session_id;
    let prompt_id;

    // Create data and close
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        session_id = session.id;

        prompt_id = graph
            .add_prompt(session.id, "Persistent prompt".to_string(), None)
            .unwrap();

        graph.flush().unwrap();
        // Graph is dropped here
    }

    // Reopen and verify data persists
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        // Verify session exists
        let session = graph.get_session(session_id).unwrap();
        assert_eq!(session.id, session_id);

        // Verify prompt exists
        let node = graph.get_node(prompt_id).unwrap();
        assert!(matches!(
            node,
            llm_memory_graph::types::Node::Prompt(_)
        ));

        if let llm_memory_graph::types::Node::Prompt(p) = node {
            assert_eq!(p.content, "Persistent prompt");
            assert_eq!(p.session_id, session_id);
        }
    }
}

#[test]
fn test_query_with_pagination() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Add 20 prompts
    for i in 0..20 {
        graph
            .add_prompt(session.id, format!("Prompt {i}"), None)
            .unwrap();
    }

    // Query first page (10 items)
    let page1 = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Prompt)
        .limit(10)
        .offset(0)
        .execute()
        .unwrap();

    assert_eq!(page1.len(), 10);

    // Query second page (10 items)
    let page2 = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Prompt)
        .limit(10)
        .offset(10)
        .execute()
        .unwrap();

    assert_eq!(page2.len(), 10);

    // Verify no overlap
    let page1_ids: Vec<_> = page1.iter().map(|n| n.id()).collect();
    let page2_ids: Vec<_> = page2.iter().map(|n| n.id()).collect();

    for id in &page1_ids {
        assert!(!page2_ids.contains(id));
    }
}

#[test]
fn test_storage_statistics() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Initial stats
    let stats_before = graph.stats().unwrap();
    assert_eq!(stats_before.node_count, 0);
    assert_eq!(stats_before.edge_count, 0);

    // Add some data
    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Test".to_string(), None)
        .unwrap();
    let usage = TokenUsage::new(10, 20);
    let _response_id = graph
        .add_response(prompt_id, "Response".to_string(), usage, None)
        .unwrap();

    // Check updated stats
    let stats_after = graph.stats().unwrap();
    assert!(stats_after.node_count >= 3); // session + prompt + response
    assert!(stats_after.edge_count >= 2); // edges created automatically
    assert!(stats_after.storage_bytes > 0);
    assert!(stats_after.session_count >= 1);
}

#[test]
fn test_custom_edges() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    let prompt1 = graph
        .add_prompt(session.id, "Prompt 1".to_string(), None)
        .unwrap();
    let prompt2 = graph
        .add_prompt(session.id, "Prompt 2".to_string(), None)
        .unwrap();

    // Add a custom edge
    graph
        .add_edge(prompt1, prompt2, EdgeType::HandledBy)
        .unwrap();

    // Verify edge exists
    let outgoing = graph.get_outgoing_edges(prompt1).unwrap();
    let handled_by_edges: Vec<_> = outgoing
        .iter()
        .filter(|e| e.edge_type == EdgeType::HandledBy)
        .collect();

    assert_eq!(handled_by_edges.len(), 1);
    assert_eq!(handled_by_edges[0].from, prompt1);
    assert_eq!(handled_by_edges[0].to, prompt2);
}

#[test]
fn test_multiple_sessions() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create multiple sessions
    let session1 = graph.create_session().unwrap();
    let session2 = graph.create_session().unwrap();

    // Add prompts to each session
    graph
        .add_prompt(session1.id, "Session 1 prompt".to_string(), None)
        .unwrap();
    graph
        .add_prompt(session2.id, "Session 2 prompt".to_string(), None)
        .unwrap();

    // Query each session separately
    let s1_prompts = QueryBuilder::new(&graph)
        .session(session1.id)
        .node_type(NodeType::Prompt)
        .execute()
        .unwrap();

    let s2_prompts = QueryBuilder::new(&graph)
        .session(session2.id)
        .node_type(NodeType::Prompt)
        .execute()
        .unwrap();

    assert_eq!(s1_prompts.len(), 1);
    assert_eq!(s2_prompts.len(), 1);

    // Verify they're different prompts
    assert_ne!(s1_prompts[0].id(), s2_prompts[0].id());
}

#[test]
fn test_error_handling_node_not_found() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let fake_id = llm_memory_graph::NodeId::new();
    let result = graph.get_node(fake_id);

    assert!(result.is_err());
}

#[test]
fn test_error_handling_session_not_found() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let fake_session = llm_memory_graph::SessionId::new();
    let result = graph.get_session(fake_session);

    assert!(result.is_err());
}

#[test]
fn test_token_usage_calculation() {
    let usage = TokenUsage::new(100, 50);

    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
    assert_eq!(usage.total_tokens, 150);
}

#[test]
fn test_query_time_filtering() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();

    // Add prompts with delays to ensure different timestamps
    let prompt1 = graph
        .add_prompt(session.id, "First".to_string(), None)
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let timestamp_middle = chrono::Utc::now();
    std::thread::sleep(std::time::Duration::from_millis(10));

    let prompt2 = graph
        .add_prompt(session.id, "Second".to_string(), None)
        .unwrap();

    // Query after middle timestamp
    let recent = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Prompt)
        .after(timestamp_middle)
        .execute()
        .unwrap();

    // Should only include prompt2
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].id(), prompt2);

    // Query before middle timestamp
    let older = QueryBuilder::new(&graph)
        .session(session.id)
        .node_type(NodeType::Prompt)
        .before(timestamp_middle)
        .execute()
        .unwrap();

    // Should only include prompt1
    assert_eq!(older.len(), 1);
    assert_eq!(older[0].id(), prompt1);
}
