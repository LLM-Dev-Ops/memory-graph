//! Comprehensive example demonstrating async streaming queries
//!
//! This example shows how to use the AsyncMemoryGraph with AsyncQueryBuilder
//! to efficiently query large datasets using streaming.
//!
//! Run with: cargo run --example async_streaming_queries

use futures::stream::StreamExt;
use llm_memory_graph::{
    engine::AsyncMemoryGraph, types::NodeType, types::TokenUsage, Config,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Async Streaming Queries Example ===\n");

    // Create async graph
    let config = Config::new("./data/async_streaming_example.db");
    let graph = AsyncMemoryGraph::open(config).await?;

    // Create a session with lots of data
    println!("Creating session and populating with prompts...");
    let session = graph.create_session().await?;

    // Add 1000 prompts to demonstrate streaming
    for i in 0..1000 {
        let prompt_content = format!("Query {}: What is topic number {}?", i, i);
        let prompt_id = graph
            .add_prompt(session.id, prompt_content, None)
            .await?;

        // Add response to every 5th prompt
        if i % 5 == 0 {
            let response_content = format!("Answer to query {}", i);
            let usage = TokenUsage::new(10, 50);
            graph
                .add_response(prompt_id, response_content, usage, None)
                .await?;
        }
    }

    println!("Created 1000 prompts and 200 responses\n");

    // Example 1: Stream all prompts efficiently
    println!("--- Example 1: Stream All Prompts ---");
    let query = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Prompt);

    let mut stream = query.execute_stream();
    let mut count = 0;

    println!("Streaming prompts one at a time (memory-efficient):");
    while let Some(result) = stream.next().await {
        let _node = result?;
        count += 1;

        // Print progress every 100 items
        if count % 100 == 0 {
            println!("  Processed {} prompts...", count);
        }
    }
    println!("Total prompts streamed: {}\n", count);

    // Example 2: Paginated queries with limit and offset
    println!("--- Example 2: Paginated Queries ---");
    let page_size = 50;
    let mut page = 0;

    loop {
        let offset = page * page_size;
        let results = graph
            .query()
            .session(session.id)
            .node_type(NodeType::Prompt)
            .offset(offset)
            .limit(page_size)
            .execute()
            .await?;

        if results.is_empty() {
            break;
        }

        println!(
            "Page {}: Retrieved {} prompts (offset: {})",
            page,
            results.len(),
            offset
        );
        page += 1;

        if page >= 3 {
            println!("  ... (showing first 3 pages only)");
            break;
        }
    }
    println!();

    // Example 3: Count without loading data
    println!("--- Example 3: Efficient Counting ---");
    let prompt_count = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Prompt)
        .count()
        .await?;

    println!("Total prompts (counted efficiently): {}", prompt_count);

    let response_count = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Response)
        .count()
        .await?;

    println!("Total responses (counted efficiently): {}\n", response_count);

    // Example 4: Time range filtering
    println!("--- Example 4: Time Range Filtering ---");
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let one_minute_ago = now - Duration::minutes(1);

    let recent_prompts = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Prompt)
        .time_range(one_minute_ago, now)
        .execute()
        .await?;

    println!(
        "Prompts created in the last minute: {}",
        recent_prompts.len()
    );
    println!("(Since we just created them, this should be 1000)\n");

    // Example 5: Streaming with filters
    println!("--- Example 5: Filtered Streaming ---");
    let filtered_query = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Response)
        .limit(50);

    let mut filtered_stream = filtered_query.execute_stream();
    let mut response_count_stream = 0;

    while let Some(result) = filtered_stream.next().await {
        let _node = result?;
        response_count_stream += 1;
    }

    println!("Streamed {} responses (with limit 50)", response_count_stream);
    println!();

    // Example 6: Multiple queries with filtering combinations
    println!("--- Example 6: Complex Filter Combinations ---");

    // Count prompts created in last minute
    let recent_count = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Prompt)
        .time_range(one_minute_ago, now)
        .count()
        .await?;

    println!("Recent prompts (last minute): {}", recent_count);

    // Get first page of responses
    let first_page_responses = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Response)
        .limit(10)
        .execute()
        .await?;

    println!("First page of responses (10 items): {}", first_page_responses.len());
    println!();

    // Example 7: Processing large datasets in chunks with streaming
    println!("--- Example 7: Chunk Processing with Streaming ---");
    let chunk_size = 100;
    let mut processed = 0;

    let query = graph
        .query()
        .session(session.id)
        .node_type(NodeType::Prompt);

    let mut stream = query.execute_stream();
    let mut chunk = Vec::new();

    while let Some(result) = stream.next().await {
        let node = result?;
        chunk.push(node);

        if chunk.len() >= chunk_size {
            // Process chunk
            processed += chunk.len();
            println!("  Processed chunk of {} items (total: {})", chunk.len(), processed);
            chunk.clear();
        }
    }

    // Process remaining items
    if !chunk.is_empty() {
        processed += chunk.len();
        println!("  Processed final chunk of {} items (total: {})", chunk.len(), processed);
    }
    println!();

    // Cleanup
    println!("--- Cleanup ---");
    println!("Flushing data to disk...");
    graph.flush().await?;

    let stats = graph.stats().await?;
    println!("Final stats:");
    println!("  Total nodes: {}", stats.node_count);
    println!("  Total edges: {}", stats.edge_count);

    println!("\n=== Example Complete ===");
    println!("Key Takeaways:");
    println!("1. Use execute_stream() for memory-efficient processing of large datasets");
    println!("2. Use execute() for loading all results at once (with pagination)");
    println!("3. Use count() to get totals without loading data");
    println!("4. Combine filters (session, node_type, time_range, limit, offset) for precise queries");
    println!("5. Process data in chunks to balance memory usage and performance");
    println!("6. Leverage async/await for non-blocking I/O operations");

    Ok(())
}
