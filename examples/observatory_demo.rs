//! Comprehensive demonstration of Observatory event streaming and metrics
//!
//! This example shows how to use the Observatory integration for real-time
//! event streaming and performance metrics collection.
//!
//! Run with: cargo run --example observatory_demo

use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    observatory::{InMemoryPublisher, ObservatoryConfig},
    types::{Config, TokenUsage},
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Observatory Integration Demo ===\n");

    // Create an in-memory event publisher for demonstration
    let publisher = Arc::new(InMemoryPublisher::new());
    println!("Created in-memory event publisher");

    // Configure Observatory with metrics enabled
    let obs_config = ObservatoryConfig::new()
        .enabled()
        .with_metrics(true)
        .with_batch_size(50)
        .with_flush_interval(1000);

    println!("Observatory configuration:");
    println!("  - Event streaming: enabled");
    println!("  - Metrics collection: enabled");
    println!("  - Batch size: 50");
    println!("  - Flush interval: 1000ms\n");

    // Create graph with Observatory
    let config = Config::new("./data/observatory_demo.db");
    let graph = AsyncMemoryGraph::with_observatory(config, Some(publisher.clone()), obs_config)
        .await?;

    println!("Created AsyncMemoryGraph with Observatory integration\n");

    // === Example 1: Basic Operations with Event Streaming ===
    println!("--- Example 1: Basic Operations ---");

    let session = graph.create_session().await?;
    println!("Created session: {}", session.id);

    let prompt_id = graph
        .add_prompt(
            session.id,
            "What are the benefits of async Rust?".to_string(),
            None,
        )
        .await?;
    println!("Added prompt: {}", prompt_id);

    let usage = TokenUsage::new(15, 100);
    let response_id = graph
        .add_response(
            prompt_id,
            "Async Rust provides non-blocking I/O...".to_string(),
            usage,
            None,
        )
        .await?;
    println!("Added response: {}\n", response_id);

    // Give async events time to publish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // === Example 2: Check Published Events ===
    println!("--- Example 2: Event Inspection ---");

    let all_events = publisher.get_events().await;
    println!("Total events published: {}", all_events.len());

    let node_events = publisher.get_events_by_type("node_created").await;
    println!("Node creation events: {}", node_events.len());

    let prompt_events = publisher.get_events_by_type("prompt_submitted").await;
    println!("Prompt submission events: {}", prompt_events.len());

    let response_events = publisher.get_events_by_type("response_generated").await;
    println!("Response generation events: {}\n", response_events.len());

    // === Example 3: Metrics Collection ===
    println!("--- Example 3: Performance Metrics ---");

    if let Some(metrics) = graph.get_metrics() {
        println!("Current metrics snapshot:");
        println!("  Nodes created: {}", metrics.nodes_created);
        println!("  Prompts submitted: {}", metrics.prompts_submitted);
        println!("  Responses generated: {}", metrics.responses_generated);
        println!("  Avg write latency: {:.3}ms", metrics.avg_write_latency_ms);
        println!("  Avg read latency: {:.3}ms", metrics.avg_read_latency_ms);
    } else {
        println!("Metrics not available");
    }
    println!();

    // === Example 4: High Volume Operations ===
    println!("--- Example 4: High Volume Operations ---");

    println!("Adding 100 prompts...");
    for i in 0..100 {
        graph
            .add_prompt(session.id, format!("Prompt number {}", i), None)
            .await?;
    }

    // Give events time to publish
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let event_count = publisher.count().await;
    println!("Total events after high volume: {}", event_count);

    if let Some(metrics) = graph.get_metrics() {
        println!("Updated metrics:");
        println!("  Total prompts: {}", metrics.prompts_submitted);
        println!("  Total nodes: {}", metrics.nodes_created);
        println!("  Avg write latency: {:.3}ms", metrics.avg_write_latency_ms);
    }
    println!();

    // === Example 5: Event Analysis ===
    println!("--- Example 5: Event Analysis ---");

    let events = publisher.get_events().await;
    println!("Analyzing {} events:", events.len());

    let mut event_type_counts = std::collections::HashMap::new();
    for event in &events {
        *event_type_counts.entry(event.event_type()).or_insert(0) += 1;
    }

    println!("Event type distribution:");
    for (event_type, count) in event_type_counts {
        println!("  {}: {}", event_type, count);
    }
    println!();

    // === Example 6: Filtering Events ===
    println!("--- Example 6: Filtering Events ---");

    let prompt_events = publisher.get_events_by_type("prompt_submitted").await;
    println!(
        "Found {} prompt submission events",
        prompt_events.len()
    );

    if let Some(first_prompt) = prompt_events.first() {
        println!("First prompt event timestamp: {}", first_prompt.timestamp());
    }

    // === Example 7: Metrics Monitoring Loop ===
    println!("\n--- Example 7: Real-time Monitoring ---");
    println!("Monitoring metrics for 3 iterations...");

    for iteration in 1..=3 {
        // Add some operations
        for _ in 0..10 {
            graph
                .add_prompt(session.id, format!("Monitoring iteration {}", iteration), None)
                .await?;
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        if let Some(metrics) = graph.get_metrics() {
            println!(
                "Iteration {}: {} prompts, {:.3}ms avg latency",
                iteration, metrics.prompts_submitted, metrics.avg_write_latency_ms
            );
        }
    }

    // === Final Summary ===
    println!("\n=== Final Summary ===");

    let final_events = publisher.count().await;
    println!("Total events published: {}", final_events);

    if let Some(final_metrics) = graph.get_metrics() {
        println!("\nFinal metrics:");
        println!("  Nodes created: {}", final_metrics.nodes_created);
        println!("  Prompts submitted: {}", final_metrics.prompts_submitted);
        println!("  Responses generated: {}", final_metrics.responses_generated);
        println!(
            "  Average write latency: {:.3}ms",
            final_metrics.avg_write_latency_ms
        );
    }

    println!("\n=== Demo Complete ===");
    println!("\nKey Takeaways:");
    println!("1. Observatory provides real-time event streaming for all graph operations");
    println!("2. InMemoryPublisher is perfect for testing and development");
    println!("3. Metrics are collected automatically for performance monitoring");
    println!("4. Events can be filtered and analyzed by type");
    println!("5. Observatory integration is optional and doesn't impact core functionality");

    Ok(())
}
