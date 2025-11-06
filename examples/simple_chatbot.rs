//! Simple chatbot example demonstrating LLM-Memory-Graph usage
//!
//! This example shows how to:
//! - Create and manage conversation sessions
//! - Store prompts and responses
//! - Retrieve conversation history
//! - Track token usage
//!
//! Run with: cargo run --example simple_chatbot

use llm_memory_graph::{
    query::{GraphTraversal, QueryBuilder},
    Config, MemoryGraph, PromptMetadata, ResponseMetadata, TokenUsage,
};
use std::collections::HashMap;
use std::io::{self, Write};

/// Mock LLM response generator (simulates an actual LLM API call)
fn mock_llm_response(prompt: &str) -> (String, TokenUsage) {
    // In a real application, this would call an actual LLM API
    let response = match prompt.to_lowercase().as_str() {
        p if p.contains("hello") || p.contains("hi") => {
            "Hello! I'm a chatbot powered by LLM-Memory-Graph. How can I help you today?"
        }
        p if p.contains("weather") => {
            "I'm a demo chatbot and don't have access to real weather data, but I can help you understand how conversation memory works!"
        }
        p if p.contains("name") => {
            "I'm a demonstration chatbot. You can call me Demo Bot!"
        }
        p if p.contains("capabilities") || p.contains("what can you do") => {
            "I can demonstrate conversation memory, track our discussion history, and show how prompts and responses are stored in a graph structure."
        }
        _ => {
            "That's an interesting question! In a real implementation, I would process this with an actual LLM. For now, I'm demonstrating the memory graph structure."
        }
    };

    // Simulate token usage
    let prompt_tokens = (prompt.len() / 4) as u32; // Rough approximation
    let completion_tokens = (response.len() / 4) as u32;
    let usage = TokenUsage::new(prompt_tokens, completion_tokens);

    (response.to_string(), usage)
}

/// Display the conversation history
fn display_history(graph: &MemoryGraph, session_id: llm_memory_graph::SessionId) -> io::Result<()> {
    println!("\n--- Conversation History ---");

    let traversal = GraphTraversal::new(graph);

    // Get all nodes in the session
    let nodes = match QueryBuilder::new(graph)
        .session(session_id)
        .execute()
    {
        Ok(nodes) => nodes,
        Err(e) => {
            eprintln!("Error retrieving history: {}", e);
            return Ok(());
        }
    };

    // Separate prompts and responses
    let mut prompts = Vec::new();
    let mut responses = Vec::new();

    for node in nodes {
        match node {
            llm_memory_graph::types::Node::Prompt(p) => prompts.push(p),
            llm_memory_graph::types::Node::Response(r) => responses.push(r),
            _ => {}
        }
    }

    // Sort by timestamp
    prompts.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    // Display each prompt with its responses
    for prompt in prompts {
        println!("\nUser: {}", prompt.content);
        println!("  [Model: {}, Temp: {}]", prompt.metadata.model, prompt.metadata.temperature);

        // Find responses to this prompt
        if let Ok(prompt_responses) = traversal.find_responses(prompt.id) {
            for resp_node in prompt_responses {
                if let llm_memory_graph::types::Node::Response(resp) = resp_node {
                    println!("\nBot: {}", resp.content);
                    println!("  [Tokens: {} prompt + {} completion = {} total, Latency: {}ms]",
                        resp.usage.prompt_tokens,
                        resp.usage.completion_tokens,
                        resp.usage.total_tokens,
                        resp.metadata.latency_ms
                    );
                }
            }
        }
    }

    println!("\n--- End of History ---\n");
    Ok(())
}

/// Display statistics about the conversation
fn display_stats(graph: &MemoryGraph) {
    match graph.stats() {
        Ok(stats) => {
            println!("\n=== Graph Statistics ===");
            println!("Total nodes: {}", stats.node_count);
            println!("Total edges: {}", stats.edge_count);
            println!("Total sessions: {}", stats.session_count);
            println!("Storage size: {} bytes", stats.storage_bytes);
            println!("=======================\n");
        }
        Err(e) => {
            eprintln!("Error retrieving stats: {}", e);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LLM-Memory-Graph Simple Chatbot Demo ===\n");

    // Initialize the graph
    let config = Config::new("./data/chatbot_demo.db")
        .with_cache_size(50)
        .with_compression(5);

    println!("Initializing memory graph at: {:?}", config.path);
    let graph = MemoryGraph::open(config)?;

    // Create a new session with metadata
    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "demo_user".to_string());
    metadata.insert("app_version".to_string(), "1.0.0".to_string());

    let session = graph.create_session_with_metadata(metadata)?;
    println!("Created session: {}\n", session.id);

    // Setup prompt metadata (simulating GPT-4 parameters)
    let prompt_metadata = PromptMetadata {
        model: "gpt-4".to_string(),
        temperature: 0.7,
        max_tokens: Some(500),
        tools_available: vec!["conversation_history".to_string()],
        custom: HashMap::new(),
    };

    println!("Commands:");
    println!("  - Type your message and press Enter to chat");
    println!("  - Type 'history' to view conversation history");
    println!("  - Type 'stats' to view graph statistics");
    println!("  - Type 'quit' or 'exit' to end the session\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Prompt for user input
        print!("You: ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim();

        // Handle special commands
        match input.to_lowercase().as_str() {
            "" => continue,
            "quit" | "exit" => {
                println!("\nEnding session. Goodbye!");
                break;
            }
            "history" => {
                display_history(&graph, session.id)?;
                continue;
            }
            "stats" => {
                display_stats(&graph);
                continue;
            }
            _ => {}
        }

        // Add the user's prompt to the graph
        let prompt_id = graph.add_prompt(
            session.id,
            input.to_string(),
            Some(prompt_metadata.clone()),
        )?;

        // Simulate LLM processing with latency tracking
        print!("Bot: ");
        stdout.flush()?;

        let start = std::time::Instant::now();
        let (response_text, usage) = mock_llm_response(input);
        let latency_ms = start.elapsed().as_millis() as u64;

        println!("{}\n", response_text);

        // Store the response with metadata
        let response_metadata = ResponseMetadata {
            model: "gpt-4".to_string(),
            finish_reason: "stop".to_string(),
            latency_ms,
            custom: HashMap::new(),
        };

        graph.add_response(
            prompt_id,
            response_text,
            usage,
            Some(response_metadata),
        )?;

        // Flush to ensure persistence
        graph.flush()?;
    }

    // Display final statistics
    display_stats(&graph);

    // Show the full conversation before exiting
    println!("Final conversation summary:");
    display_history(&graph, session.id)?;

    Ok(())
}
