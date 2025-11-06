//! Tool Invocation Example for LLM-Memory-Graph
//!
//! This example demonstrates how to:
//! - Track LLM tool/function calls
//! - Store tool invocation results
//! - Handle tool execution failures and retries
//! - Query tool invocation history
//! - Track performance metrics for tool calls
//!
//! Run with: cargo run --example tool_invocations

use llm_memory_graph::{
    Config, MemoryGraph, PromptMetadata, ResponseMetadata, TokenUsage, ToolInvocation,
};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

/// Simulate a web search tool
fn execute_web_search(query: &str) -> Result<serde_json::Value, String> {
    println!("  🔍 Executing web search for: '{}'", query);
    thread::sleep(Duration::from_millis(300)); // Simulate API latency

    // Simulate successful search results
    Ok(serde_json::json!({
        "results": [
            {
                "title": format!("Result for {}", query),
                "url": format!("https://example.com/search?q={}", query.replace(' ', "+")),
                "snippet": format!("Relevant information about {}", query)
            },
            {
                "title": format!("Advanced {} Tutorial", query),
                "url": "https://example.com/tutorial",
                "snippet": "Learn more about this topic"
            }
        ],
        "count": 2
    }))
}

/// Simulate a calculator tool
fn execute_calculator(operation: &str, a: f64, b: f64) -> Result<serde_json::Value, String> {
    println!(
        "  🧮 Executing calculator: {} {} {}",
        a, operation, b
    );
    thread::sleep(Duration::from_millis(50)); // Simulate processing

    let result = match operation {
        "add" => a + b,
        "subtract" => a - b,
        "multiply" => a * b,
        "divide" => {
            if b == 0.0 {
                return Err("Division by zero".to_string());
            }
            a / b
        }
        _ => return Err(format!("Unknown operation: {}", operation)),
    };

    Ok(serde_json::json!({
        "operation": operation,
        "operands": [a, b],
        "result": result
    }))
}

/// Simulate a weather API tool (with occasional failures)
fn execute_weather_api(city: &str, attempt: u32) -> Result<serde_json::Value, String> {
    println!("  🌤️  Fetching weather for: {} (attempt {})", city, attempt);
    thread::sleep(Duration::from_millis(200));

    // Simulate rate limiting on first attempt
    if attempt == 1 {
        return Err("Rate limit exceeded. Please retry.".to_string());
    }

    // Success on retry
    Ok(serde_json::json!({
        "city": city,
        "temperature": 72,
        "condition": "Sunny",
        "humidity": 65,
        "wind_speed": 12
    }))
}

/// Display tool invocation history for a response
fn display_tool_history(
    graph: &MemoryGraph,
    response_id: llm_memory_graph::NodeId,
) -> Result<(), Box<dyn std::error::Error>> {
    let tools = graph.get_response_tools(response_id)?;

    if tools.is_empty() {
        println!("  No tools invoked for this response.\n");
        return Ok(());
    }

    println!("  Tool Invocations ({}):", tools.len());
    for (i, tool) in tools.iter().enumerate() {
        println!("\n  {}. {} ({})", i + 1, tool.tool_name, tool.status());
        println!("     Duration: {}ms", tool.duration_ms);
        println!("     Retries: {}", tool.retry_count);

        if let Some(result) = &tool.result {
            println!("     Result: {}", serde_json::to_string_pretty(result)?);
        }

        if let Some(error) = &tool.error {
            println!("     Error: {}", error);
        }

        if !tool.metadata.is_empty() {
            println!("     Metadata: {:?}", tool.metadata);
        }
    }
    println!();

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM Tool Invocation Demo ===\n");

    // Initialize the graph
    let config = Config::new("./data/tool_demo.db")
        .with_cache_size(50)
        .with_compression(5);

    println!("Initializing memory graph at: {:?}\n", config.path);
    let graph = MemoryGraph::open(config)?;

    // Create a session
    let mut session_metadata = HashMap::new();
    session_metadata.insert("scenario".to_string(), "tool_demo".to_string());
    let session = graph.create_session_with_metadata(session_metadata)?;
    println!("Created session: {}\n", session.id);

    // ===== SCENARIO 1: Successful Tool Invocation =====
    println!("📋 SCENARIO 1: Successful Tool Invocation\n");

    let prompt1 = "Search for information about Rust programming language";
    println!("User: {}", prompt1);

    let prompt1_id = graph.add_prompt(
        session.id,
        prompt1.to_string(),
        Some(PromptMetadata {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: Some(500),
            tools_available: vec!["web_search".to_string()],
            custom: HashMap::new(),
        }),
    )?;

    let response1_text = "I'll search for information about Rust for you.";
    println!("Bot: {}", response1_text);

    let response1_id = graph.add_response(
        prompt1_id,
        response1_text.to_string(),
        TokenUsage::new(15, 10),
        Some(ResponseMetadata {
            model: "gpt-4".to_string(),
            finish_reason: "tool_calls".to_string(),
            latency_ms: 150,
            custom: HashMap::new(),
        }),
    )?;

    // Execute tool and track it
    let tool1 = ToolInvocation::new(
        response1_id,
        "web_search".to_string(),
        serde_json::json!({"query": "Rust programming language"}),
    );
    let tool1_id = graph.add_tool_invocation(tool1)?;

    let start = std::time::Instant::now();
    match execute_web_search("Rust programming language") {
        Ok(result) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(tool1_id, true, result.to_string(), duration)?;
            println!("  ✅ Tool completed successfully in {}ms", duration);
        }
        Err(e) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(tool1_id, false, e, duration)?;
            println!("  ❌ Tool failed");
        }
    }

    display_tool_history(&graph, response1_id)?;

    // ===== SCENARIO 2: Multiple Tool Invocations =====
    println!("📋 SCENARIO 2: Multiple Tool Invocations\n");

    let prompt2 = "Calculate 42 * 1.5 and then search for the result";
    println!("User: {}", prompt2);

    let prompt2_id = graph.add_prompt(
        session.id,
        prompt2.to_string(),
        Some(PromptMetadata {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: Some(500),
            tools_available: vec!["calculator".to_string(), "web_search".to_string()],
            custom: HashMap::new(),
        }),
    )?;

    let response2_text = "I'll calculate that and search for more information.";
    println!("Bot: {}", response2_text);

    let response2_id = graph.add_response(
        prompt2_id,
        response2_text.to_string(),
        TokenUsage::new(20, 12),
        Some(ResponseMetadata {
            model: "gpt-4".to_string(),
            finish_reason: "tool_calls".to_string(),
            latency_ms: 180,
            custom: HashMap::new(),
        }),
    )?;

    // First tool: calculator
    let calc_tool = ToolInvocation::new(
        response2_id,
        "calculator".to_string(),
        serde_json::json!({"operation": "multiply", "a": 42.0, "b": 1.5}),
    );
    let calc_tool_id = graph.add_tool_invocation(calc_tool)?;

    let start = std::time::Instant::now();
    match execute_calculator("multiply", 42.0, 1.5) {
        Ok(result) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(calc_tool_id, true, result.to_string(), duration)?;
            println!("  ✅ Calculator completed in {}ms", duration);
        }
        Err(e) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(calc_tool_id, false, e, duration)?;
        }
    }

    // Second tool: web search based on calculation result
    let search_tool = ToolInvocation::new(
        response2_id,
        "web_search".to_string(),
        serde_json::json!({"query": "63"}),
    );
    let search_tool_id = graph.add_tool_invocation(search_tool)?;

    let start = std::time::Instant::now();
    match execute_web_search("63") {
        Ok(result) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(search_tool_id, true, result.to_string(), duration)?;
            println!("  ✅ Search completed in {}ms", duration);
        }
        Err(e) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(search_tool_id, false, e, duration)?;
        }
    }

    display_tool_history(&graph, response2_id)?;

    // ===== SCENARIO 3: Tool Failure and Retry =====
    println!("📋 SCENARIO 3: Tool Failure with Retry\n");

    let prompt3 = "What's the weather in San Francisco?";
    println!("User: {}", prompt3);

    let prompt3_id = graph.add_prompt(
        session.id,
        prompt3.to_string(),
        Some(PromptMetadata {
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_tokens: Some(500),
            tools_available: vec!["weather_api".to_string()],
            custom: HashMap::new(),
        }),
    )?;

    let response3_text = "Let me check the weather for you.";
    println!("Bot: {}", response3_text);

    let response3_id = graph.add_response(
        prompt3_id,
        response3_text.to_string(),
        TokenUsage::new(12, 8),
        Some(ResponseMetadata {
            model: "gpt-4".to_string(),
            finish_reason: "tool_calls".to_string(),
            latency_ms: 120,
            custom: HashMap::new(),
        }),
    )?;

    // Create tool invocation with retry logic
    let mut weather_tool = ToolInvocation::new(
        response3_id,
        "weather_api".to_string(),
        serde_json::json!({"city": "San Francisco"}),
    );

    let mut attempt = 1;
    let max_retries = 3;

    loop {
        let start = std::time::Instant::now();
        match execute_weather_api("San Francisco", attempt) {
            Ok(result) => {
                let duration = start.elapsed().as_millis() as u64;
                weather_tool.mark_success(result, duration);
                println!("  ✅ Weather API succeeded on attempt {}", attempt);
                break;
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                println!("  ⚠️  Attempt {} failed: {}", attempt, e);

                if attempt >= max_retries {
                    weather_tool.mark_failed(e, duration);
                    println!("  ❌ Max retries reached");
                    break;
                }

                weather_tool.record_retry();
                attempt += 1;
                thread::sleep(Duration::from_millis(500)); // Backoff
            }
        }
    }

    // Add metadata about the retry process
    weather_tool.add_metadata("max_retries".to_string(), max_retries.to_string());
    weather_tool.add_metadata("backoff_ms".to_string(), "500".to_string());

    let _weather_tool_id = graph.add_tool_invocation(weather_tool)?;
    display_tool_history(&graph, response3_id)?;

    // ===== SCENARIO 4: Tool Error Handling =====
    println!("📋 SCENARIO 4: Tool Error Handling\n");

    let prompt4 = "Calculate 10 divided by 0";
    println!("User: {}", prompt4);

    let prompt4_id = graph.add_prompt(session.id, prompt4.to_string(), None)?;

    let response4_text = "I'll perform that calculation.";
    println!("Bot: {}", response4_text);

    let response4_id = graph.add_response(
        prompt4_id,
        response4_text.to_string(),
        TokenUsage::new(10, 6),
        None,
    )?;

    let error_tool = ToolInvocation::new(
        response4_id,
        "calculator".to_string(),
        serde_json::json!({"operation": "divide", "a": 10.0, "b": 0.0}),
    );
    let error_tool_id = graph.add_tool_invocation(error_tool)?;

    let start = std::time::Instant::now();
    match execute_calculator("divide", 10.0, 0.0) {
        Ok(result) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(error_tool_id, true, result.to_string(), duration)?;
        }
        Err(e) => {
            let duration = start.elapsed().as_millis() as u64;
            graph.update_tool_invocation(error_tool_id, false, e.clone(), duration)?;
            println!("  ❌ Tool failed: {}", e);
        }
    }

    display_tool_history(&graph, response4_id)?;

    // ===== FINAL STATISTICS =====
    println!("📊 FINAL STATISTICS\n");

    let stats = graph.stats()?;
    println!("Total nodes: {}", stats.node_count);
    println!("Total edges: {}", stats.edge_count);
    println!("Sessions: {}", stats.session_count);
    println!("Storage size: {} bytes", stats.storage_bytes);

    // Count tool invocations across all scenarios
    let mut total_tools = 0;
    let mut successful_tools = 0;
    let mut failed_tools = 0;

    for response_id in [response1_id, response2_id, response3_id, response4_id] {
        let tools = graph.get_response_tools(response_id)?;
        total_tools += tools.len();
        successful_tools += tools.iter().filter(|t| t.is_success()).count();
        failed_tools += tools.iter().filter(|t| t.is_failed()).count();
    }

    println!("\nTool Invocation Summary:");
    println!("  Total invocations: {}", total_tools);
    println!("  Successful: {}", successful_tools);
    println!("  Failed: {}", failed_tools);
    println!("  Success rate: {:.1}%", (successful_tools as f64 / total_tools as f64) * 100.0);

    println!("\n✅ Demo completed successfully!");
    println!("Database persisted at: ./data/tool_demo.db");

    Ok(())
}
