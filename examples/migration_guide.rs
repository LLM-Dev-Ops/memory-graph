//! Comprehensive guide for migrating from sync to async API
//!
//! This example demonstrates various migration strategies and patterns.
//!
//! Run with: cargo run --example migration_guide

use llm_memory_graph::{
    migration::MigrationHelper, AsyncMemoryGraph, Config, MemoryGraph, TokenUsage,
};

/// Example 1: Basic sync code (MVP/v0.1 style)
fn sync_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Sync API (MVP) ===\n");

    let config = Config::new("./data/sync_example.db");
    let graph = MemoryGraph::open(config)?;

    // Create session
    let session = graph.create_session()?;
    println!("Created session: {}", session.id);

    // Add prompt
    let prompt_id = graph.add_prompt(
        session.id,
        "What are the benefits of async Rust?".to_string(),
        None,
    )?;
    println!("Added prompt: {}", prompt_id);

    // Add response
    let usage = TokenUsage::new(15, 120);
    let response_id = graph.add_response(
        prompt_id,
        "Async Rust provides non-blocking I/O...".to_string(),
        usage,
        None,
    )?;
    println!("Added response: {}\n", response_id);

    Ok(())
}

/// Example 2: Async code (Beta/v0.2 style)
async fn async_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 2: Async API (Beta) ===\n");

    let config = Config::new("./data/async_example.db");
    let graph = AsyncMemoryGraph::open(config).await?;

    // Create session
    let session = graph.create_session().await?;
    println!("Created session: {}", session.id);

    // Add prompt
    let prompt_id = graph
        .add_prompt(
            session.id,
            "What are the benefits of async Rust?".to_string(),
            None,
        )
        .await?;
    println!("Added prompt: {}", prompt_id);

    // Add response
    let usage = TokenUsage::new(15, 120);
    let response_id = graph
        .add_response(
            prompt_id,
            "Async Rust provides non-blocking I/O...".to_string(),
            usage,
            None,
        )
        .await?;
    println!("Added response: {}\n", response_id);

    Ok(())
}

/// Example 3: Gradual migration - Mixed sync and async
///
/// This shows how to gradually migrate by having both sync and async
/// components coexist during the transition period.
async fn gradual_migration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 3: Gradual Migration ===\n");

    let config = Config::new("./data/gradual_migration.db");

    // Step 1: Create data with sync API (existing code)
    println!("Step 1: Using sync API to create initial data");
    let session_id = {
        let graph = MemoryGraph::open(config.clone())?;
        let session = graph.create_session()?;
        println!("  Created session (sync): {}", session.id);
        session.id
    };

    // Step 2: Read and extend with async API (new code)
    println!("\nStep 2: Using async API to extend data");
    {
        let graph = AsyncMemoryGraph::open(config.clone()).await?;

        // Verify we can read the session created by sync API
        let session = graph.get_session(session_id).await?;
        println!("  Retrieved session (async): {}", session.id);

        // Add new data with async API
        let prompt_id = graph
            .add_prompt(session_id, "New async prompt".to_string(), None)
            .await?;
        println!("  Added prompt (async): {}", prompt_id);
    }

    // Step 3: Verify with sync API (old code still works)
    println!("\nStep 3: Verifying with sync API");
    {
        let graph = MemoryGraph::open(config)?;
        let session = graph.get_session(session_id)?;
        println!("  Session still accessible (sync): {}", session.id);
    }

    println!("\n✓ Gradual migration successful!\n");
    Ok(())
}

/// Example 4: Using migration helper utilities
async fn migration_helper_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 4: Migration Helper Utilities ===\n");

    let config = Config::new("./data/migration_helper.db");

    // Create some initial data
    {
        let graph = MemoryGraph::open(config.clone())?;
        graph.create_session()?;
        graph.create_session()?;
    }

    // Test 1: Verify compatibility
    println!("Test 1: Verifying API compatibility");
    let compat = MigrationHelper::verify_compatibility(&config).await?;
    println!("  Sync accessible: {}", compat.sync_accessible);
    println!("  Async accessible: {}", compat.async_accessible);
    println!("  Compatible: {}", compat.compatible);
    println!(
        "  Node counts match: {} == {}",
        compat.sync_node_count, compat.async_node_count
    );

    // Test 2: Create checkpoint
    println!("\nTest 2: Creating migration checkpoint");
    let checkpoint = MigrationHelper::create_checkpoint(&config).await?;
    println!("  Checkpoint created at: {}", checkpoint.timestamp);
    println!("  Nodes: {}", checkpoint.node_count);
    println!("  Edges: {}", checkpoint.edge_count);

    // Test 3: Add more data and verify
    println!("\nTest 3: Adding data and verifying checkpoint");
    {
        let graph = AsyncMemoryGraph::open(config.clone()).await?;
        graph.create_session().await?;
    }

    let verification = MigrationHelper::verify_checkpoint(&config, &checkpoint).await?;
    println!("  Checkpoint valid: {}", verification.valid);
    println!("  Nodes added: {}", verification.nodes_added);

    // Test 4: Run complete migration test
    println!("\nTest 4: Running complete migration test");
    let test_report = MigrationHelper::run_migration_test(&config).await?;
    println!("  Test success: {}", test_report.success);
    println!("  Steps completed: {}", test_report.steps_completed.len());
    for step in &test_report.steps_completed {
        println!("    ✓ {}", step);
    }
    if !test_report.errors.is_empty() {
        println!("  Errors:");
        for error in &test_report.errors {
            println!("    ✗ {}", error);
        }
    }

    println!("\n✓ Migration helper tests complete!\n");
    Ok(())
}

/// Example 5: Concurrent access pattern
///
/// Shows how to use both APIs concurrently (e.g., during a rolling migration)
async fn concurrent_access_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 5: Concurrent Access Pattern ===\n");

    let config = Config::new("./data/concurrent_access.db");

    // Create initial data
    let session_id = {
        let graph = MemoryGraph::open(config.clone())?;
        let session = graph.create_session()?;
        println!("Initial session created: {}", session.id);
        session.id
    };

    // Spawn async tasks that can run concurrently
    let config_clone = config.clone();
    let async_handle = tokio::spawn(async move {
        let graph = AsyncMemoryGraph::open(config_clone).await.unwrap();
        for i in 0..5 {
            graph
                .add_prompt(session_id, format!("Async prompt {}", i), None)
                .await
                .unwrap();
        }
        println!("Async tasks completed");
    });

    // Meanwhile, sync code can still work
    std::thread::spawn(move || {
        let graph = MemoryGraph::open(config).unwrap();
        for i in 0..5 {
            graph
                .add_prompt(session_id, format!("Sync prompt {}", i), None)
                .unwrap();
        }
        println!("Sync tasks completed");
    });

    async_handle.await?;

    println!("\n✓ Concurrent access successful!\n");
    Ok(())
}

/// Example 6: Migration checklist
async fn migration_checklist() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 6: Pre-Migration Checklist ===\n");

    let config = Config::new("./data/checklist.db");

    // Create test database
    {
        let graph = MemoryGraph::open(config.clone())?;
        graph.create_session()?;
    }

    println!("Running pre-migration checks...\n");

    // Check 1: Database accessibility
    print!("□ Database accessible with sync API... ");
    match MemoryGraph::open(config.clone()) {
        Ok(_) => println!("✓"),
        Err(e) => {
            println!("✗ Error: {}", e);
            return Err(e.into());
        }
    }

    // Check 2: Database accessible with async API
    print!("□ Database accessible with async API... ");
    match AsyncMemoryGraph::open(config.clone()).await {
        Ok(_) => println!("✓"),
        Err(e) => {
            println!("✗ Error: {}", e);
            return Err(e.into());
        }
    }

    // Check 3: APIs compatible
    print!("□ APIs are compatible... ");
    let compat = MigrationHelper::verify_compatibility(&config).await?;
    if compat.compatible {
        println!("✓");
    } else {
        println!("✗ Compatibility issue detected");
        return Err("APIs not compatible".into());
    }

    // Check 4: Create backup checkpoint
    print!("□ Creating backup checkpoint... ");
    let checkpoint = MigrationHelper::create_checkpoint(&config).await?;
    println!("✓ ({} nodes, {} edges)", checkpoint.node_count, checkpoint.edge_count);

    // Check 5: Test migration
    print!("□ Running migration test... ");
    let test = MigrationHelper::run_migration_test(&config).await?;
    if test.success {
        println!("✓");
    } else {
        println!("✗ Migration test failed");
        for error in &test.errors {
            println!("  Error: {}", error);
        }
        return Err("Migration test failed".into());
    }

    println!("\n✅ All pre-migration checks passed!");
    println!("\nReady to migrate! Follow these steps:");
    println!("1. Add tokio = {{ version = \"1\", features = [\"full\"] }} to Cargo.toml");
    println!("2. Add #[tokio::main] to your main function");
    println!("3. Add .await? to all graph operations");
    println!("4. Change function signatures to async");
    println!("5. Test thoroughly before deploying\n");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 LLM-Memory-Graph Migration Guide\n");
    println!("This example demonstrates migrating from sync (MVP) to async (Beta) API\n");
    println!("═══════════════════════════════════════════════════════════════════════\n");

    // Run all examples
    sync_example()?;
    async_example().await?;
    gradual_migration_example().await?;
    migration_helper_example().await?;
    concurrent_access_example().await?;
    migration_checklist().await?;

    println!("═══════════════════════════════════════════════════════════════════════\n");
    println!("✅ All migration examples completed successfully!\n");
    println!("Key Takeaways:");
    println!("1. Both sync and async APIs use the same storage format");
    println!("2. You can migrate gradually, one component at a time");
    println!("3. Use MigrationHelper utilities to verify compatibility");
    println!("4. Create checkpoints before major migrations");
    println!("5. Both APIs can coexist during the transition period\n");

    Ok(())
}
