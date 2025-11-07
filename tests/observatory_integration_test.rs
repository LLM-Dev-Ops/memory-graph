//! Integration tests for Observatory functionality

use llm_memory_graph::{
    engine::AsyncMemoryGraph,
    observatory::{InMemoryPublisher, ObservatoryConfig},
    types::{Config, TokenUsage},
};
use std::sync::Arc;

#[tokio::test]
async fn test_observatory_integration() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());
    let publisher = Arc::new(InMemoryPublisher::new());

    let obs_config = ObservatoryConfig::new().enabled().with_metrics(true);

    let graph = AsyncMemoryGraph::with_observatory(config, Some(publisher.clone()), obs_config)
        .await
        .unwrap();

    // Create session
    let session = graph.create_session().await.unwrap();

    // Add prompt
    let prompt_id = graph
        .add_prompt(session.id, "Test prompt".to_string(), None)
        .await
        .unwrap();

    // Add response
    let usage = TokenUsage::new(10, 20);
    let response_id = graph
        .add_response(prompt_id, "Test response".to_string(), usage, None)
        .await
        .unwrap();

    // Give async events time to publish
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Check events were published
    let events = publisher.get_events().await;
    assert!(!events.is_empty(), "Expected events to be published");

    // Should have at least PromptSubmitted event
    let prompt_events = publisher.get_events_by_type("prompt_submitted").await;
    assert_eq!(
        prompt_events.len(),
        1,
        "Expected exactly one PromptSubmitted event"
    );

    // Check metrics
    let metrics = graph.get_metrics().expect("Metrics should be available");
    assert_eq!(metrics.prompts_submitted, 1);
    assert!(
        metrics.nodes_created >= 2,
        "Expected at least 2 nodes created (session + prompt, possibly response)"
    );
}

#[tokio::test]
async fn test_observatory_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());

    let obs_config = ObservatoryConfig::new(); // Not enabled

    let graph = AsyncMemoryGraph::with_observatory(config, None, obs_config)
        .await
        .unwrap();

    // Create session and prompt
    let session = graph.create_session().await.unwrap();
    graph
        .add_prompt(session.id, "Test".to_string(), None)
        .await
        .unwrap();

    // Metrics should still work if enabled
    let metrics = graph.get_metrics();
    assert!(
        metrics.is_some(),
        "Metrics should be available even when events are disabled"
    );
}

#[tokio::test]
async fn test_metrics_collection() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());
    let publisher = Arc::new(InMemoryPublisher::new());

    let obs_config = ObservatoryConfig::new().enabled().with_metrics(true);

    let graph = AsyncMemoryGraph::with_observatory(config, Some(publisher), obs_config)
        .await
        .unwrap();

    let session = graph.create_session().await.unwrap();

    // Add multiple prompts
    for i in 0..10 {
        graph
            .add_prompt(session.id, format!("Prompt {}", i), None)
            .await
            .unwrap();
    }

    let metrics = graph.get_metrics().unwrap();
    assert_eq!(metrics.prompts_submitted, 10);
    assert!(
        metrics.avg_write_latency_ms > 0.0,
        "Average write latency should be recorded"
    );
}

#[tokio::test]
async fn test_concurrent_event_publishing() {
    let dir = tempfile::tempdir().unwrap();
    let config = Config::new(dir.path());
    let publisher = Arc::new(InMemoryPublisher::new());

    let obs_config = ObservatoryConfig::new().enabled();

    let graph = Arc::new(
        AsyncMemoryGraph::with_observatory(config, Some(publisher.clone()), obs_config)
            .await
            .unwrap(),
    );

    let session = graph.create_session().await.unwrap();

    // Create 50 prompts concurrently
    let mut handles = vec![];
    for i in 0..50 {
        let graph_clone = Arc::clone(&graph);
        let session_id = session.id;
        let handle = tokio::spawn(async move {
            graph_clone
                .add_prompt(session_id, format!("Concurrent prompt {}", i), None)
                .await
        });
        handles.push(handle);
    }

    // Wait for all
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Give events time to publish
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Check metrics
    let metrics = graph.get_metrics().unwrap();
    assert_eq!(metrics.prompts_submitted, 50);

    // Check events
    let events = publisher.get_events().await;
    let prompt_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type() == "prompt_submitted")
        .collect();
    assert_eq!(prompt_events.len(), 50);
}
