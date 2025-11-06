//! Integration tests for ToolInvocation functionality
//!
//! These tests verify the full workflow of tool invocations with LLM responses,
//! including creation, updates, querying, and edge relationships.

use llm_memory_graph::{
    MemoryGraph, Config, TokenUsage, ResponseMetadata, ToolInvocation,
};
use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_tool_invocation_workflow() {
    // Create a temporary database
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    // Create a session and prompt
    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(
            session.id,
            "Search the web for information about Rust programming".to_string(),
            None,
        )
        .unwrap();

    // Add a response that uses tools
    let usage = TokenUsage::new(25, 15);
    let response_meta = ResponseMetadata {
        model: "gpt-4".to_string(),
        finish_reason: "tool_calls".to_string(),
        latency_ms: 350,
        custom: HashMap::new(),
    };

    let response_id = graph
        .add_response(
            prompt_id,
            "I'll search for that information.".to_string(),
            usage,
            Some(response_meta),
        )
        .unwrap();

    // Create a tool invocation
    let params = serde_json::json!({
        "query": "Rust programming language features",
        "max_results": 5
    });

    let tool = ToolInvocation::new(response_id, "web_search".to_string(), params.clone());
    let tool_id = graph.add_tool_invocation(tool).unwrap();

    // Verify the tool was created
    let retrieved_node = graph.get_node(tool_id).unwrap();
    if let llm_memory_graph::types::Node::ToolInvocation(t) = retrieved_node {
        assert_eq!(t.tool_name, "web_search");
        assert_eq!(t.parameters, params);
        assert_eq!(t.response_id, response_id);
        assert!(t.is_pending());
        assert!(!t.is_success());
        assert!(!t.is_failed());
    } else {
        panic!("Expected ToolInvocation node");
    }

    // Simulate successful tool execution
    let result = serde_json::json!({
        "results": [
            {"title": "Rust Language", "url": "https://rust-lang.org"},
            {"title": "Rust Book", "url": "https://doc.rust-lang.org/book/"}
        ]
    });

    graph
        .update_tool_invocation(tool_id, true, result.to_string(), 450)
        .unwrap();

    // Verify the tool was updated
    let updated_node = graph.get_node(tool_id).unwrap();
    if let llm_memory_graph::types::Node::ToolInvocation(t) = updated_node {
        assert!(t.is_success());
        assert!(!t.is_pending());
        assert!(!t.is_failed());
        assert_eq!(t.duration_ms, 450);
        assert_eq!(t.result, Some(result));
        assert_eq!(t.error, None);
    } else {
        panic!("Expected ToolInvocation node");
    }

    // Retrieve tools for the response
    let tools = graph.get_response_tools(response_id).unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].id, tool_id);
    assert_eq!(tools[0].tool_name, "web_search");
}

#[test]
fn test_multiple_tool_invocations() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(
            session.id,
            "Get weather and news for San Francisco".to_string(),
            None,
        )
        .unwrap();

    let usage = TokenUsage::new(20, 10);
    let response_id = graph
        .add_response(
            prompt_id,
            "I'll fetch both weather and news.".to_string(),
            usage,
            None,
        )
        .unwrap();

    // Create multiple tool invocations for the same response
    let weather_tool = ToolInvocation::new(
        response_id,
        "get_weather".to_string(),
        serde_json::json!({"city": "San Francisco"}),
    );
    let weather_id = graph.add_tool_invocation(weather_tool).unwrap();

    let news_tool = ToolInvocation::new(
        response_id,
        "get_news".to_string(),
        serde_json::json!({"city": "San Francisco", "limit": 10}),
    );
    let news_id = graph.add_tool_invocation(news_tool).unwrap();

    // Update weather tool - success
    graph
        .update_tool_invocation(
            weather_id,
            true,
            serde_json::json!({"temp": 65, "condition": "Sunny"}).to_string(),
            200,
        )
        .unwrap();

    // Update news tool - failure
    graph
        .update_tool_invocation(
            news_id,
            false,
            "API rate limit exceeded".to_string(),
            100,
        )
        .unwrap();

    // Retrieve all tools for the response
    let tools = graph.get_response_tools(response_id).unwrap();
    assert_eq!(tools.len(), 2);

    // Verify tool states
    let weather = tools.iter().find(|t| t.tool_name == "get_weather").unwrap();
    assert!(weather.is_success());
    assert_eq!(weather.duration_ms, 200);

    let news = tools.iter().find(|t| t.tool_name == "get_news").unwrap();
    assert!(news.is_failed());
    assert_eq!(news.error, Some("API rate limit exceeded".to_string()));
    assert_eq!(news.duration_ms, 100);
}

#[test]
fn test_tool_invocation_retry_workflow() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Fetch data from API".to_string(), None)
        .unwrap();

    let usage = TokenUsage::new(15, 8);
    let response_id = graph
        .add_response(prompt_id, "Calling API...".to_string(), usage, None)
        .unwrap();

    // Create tool invocation
    let mut tool = ToolInvocation::new(
        response_id,
        "api_call".to_string(),
        serde_json::json!({"endpoint": "/data"}),
    );

    // Simulate retries
    tool.record_retry();
    assert_eq!(tool.retry_count, 1);

    tool.record_retry();
    assert_eq!(tool.retry_count, 2);

    // Finally succeed
    tool.mark_success(serde_json::json!({"data": "success"}), 300);
    assert_eq!(tool.retry_count, 2); // Retry count preserved
    assert!(tool.is_success());

    let tool_id = graph.add_tool_invocation(tool).unwrap();

    // Verify retry count persisted
    let retrieved = graph.get_node(tool_id).unwrap();
    if let llm_memory_graph::types::Node::ToolInvocation(t) = retrieved {
        assert_eq!(t.retry_count, 2);
        assert!(t.is_success());
    } else {
        panic!("Expected ToolInvocation node");
    }
}

#[test]
fn test_tool_invocation_with_metadata() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Test".to_string(), None)
        .unwrap();

    let usage = TokenUsage::new(10, 5);
    let response_id = graph
        .add_response(prompt_id, "Test response".to_string(), usage, None)
        .unwrap();

    // Create tool with metadata
    let mut tool = ToolInvocation::new(
        response_id,
        "calculator".to_string(),
        serde_json::json!({"operation": "add", "a": 5, "b": 3}),
    );

    tool.add_metadata("cache_hit".to_string(), "false".to_string());
    tool.add_metadata("execution_node".to_string(), "node-1".to_string());
    tool.add_metadata("priority".to_string(), "high".to_string());

    tool.mark_success(serde_json::json!({"result": 8}), 50);

    let tool_id = graph.add_tool_invocation(tool).unwrap();

    // Verify metadata persisted
    let retrieved = graph.get_node(tool_id).unwrap();
    if let llm_memory_graph::types::Node::ToolInvocation(t) = retrieved {
        assert_eq!(t.metadata.len(), 3);
        assert_eq!(t.metadata.get("cache_hit"), Some(&"false".to_string()));
        assert_eq!(
            t.metadata.get("execution_node"),
            Some(&"node-1".to_string())
        );
        assert_eq!(t.metadata.get("priority"), Some(&"high".to_string()));
    } else {
        panic!("Expected ToolInvocation node");
    }
}

#[test]
fn test_tool_invocation_persistence() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().to_path_buf();

    let tool_id;
    let response_id_saved;

    // Create and save tool invocation
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let session = graph.create_session().unwrap();
        let prompt_id = graph
            .add_prompt(session.id, "Test persistence".to_string(), None)
            .unwrap();

        let usage = TokenUsage::new(10, 10);
        response_id_saved = graph
            .add_response(prompt_id, "Response".to_string(), usage, None)
            .unwrap();

        let mut tool = ToolInvocation::new(
            response_id_saved,
            "test_tool".to_string(),
            serde_json::json!({"test": "value"}),
        );

        tool.mark_success(serde_json::json!({"result": "ok"}), 100);
        tool_id = graph.add_tool_invocation(tool).unwrap();

        graph.flush().unwrap();
    }

    // Reopen and verify
    {
        let config = Config::new(&db_path);
        let graph = MemoryGraph::open(config).unwrap();

        let retrieved = graph.get_node(tool_id).unwrap();
        if let llm_memory_graph::types::Node::ToolInvocation(t) = retrieved {
            assert_eq!(t.id, tool_id);
            assert_eq!(t.response_id, response_id_saved);
            assert_eq!(t.tool_name, "test_tool");
            assert!(t.is_success());
            assert_eq!(t.duration_ms, 100);
        } else {
            panic!("Expected ToolInvocation node");
        }

        // Verify edge relationship persisted
        let tools = graph.get_response_tools(response_id_saved).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].id, tool_id);
    }
}

#[test]
fn test_tool_invocation_status_transitions() {
    let dir = tempdir().unwrap();
    let config = Config::new(dir.path());
    let graph = MemoryGraph::open(config).unwrap();

    let session = graph.create_session().unwrap();
    let prompt_id = graph
        .add_prompt(session.id, "Test".to_string(), None)
        .unwrap();

    let usage = TokenUsage::new(10, 5);
    let response_id = graph
        .add_response(prompt_id, "Test".to_string(), usage, None)
        .unwrap();

    let mut tool = ToolInvocation::new(
        response_id,
        "test_tool".to_string(),
        serde_json::json!({}),
    );

    // Initially pending
    assert_eq!(tool.status(), "pending");
    assert!(tool.is_pending());

    // Mark as failed
    tool.mark_failed("Error".to_string(), 50);
    assert_eq!(tool.status(), "failed");
    assert!(tool.is_failed());
    assert!(!tool.is_pending());
    assert!(!tool.is_success());

    // Can't go from failed to success directly in this test,
    // but we can create a new tool that succeeds
    let mut tool2 = ToolInvocation::new(
        response_id,
        "test_tool_2".to_string(),
        serde_json::json!({}),
    );

    tool2.mark_success(serde_json::json!({"result": "ok"}), 100);
    assert_eq!(tool2.status(), "success");
    assert!(tool2.is_success());
    assert!(!tool2.is_pending());
    assert!(!tool2.is_failed());

    // Store both and verify
    let _tool1_id = graph.add_tool_invocation(tool).unwrap();
    let _tool2_id = graph.add_tool_invocation(tool2).unwrap();

    let tools = graph.get_response_tools(response_id).unwrap();
    assert_eq!(tools.len(), 2);

    let failed_tool = tools.iter().find(|t| t.tool_name == "test_tool").unwrap();
    assert!(failed_tool.is_failed());

    let success_tool = tools
        .iter()
        .find(|t| t.tool_name == "test_tool_2")
        .unwrap();
    assert!(success_tool.is_success());
}
